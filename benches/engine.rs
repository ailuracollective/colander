//! Engine benchmark: the per-call cost of `evaluate_rules`, `validate_response`
//! and `compile` through the C ABI.
//!
//! # What this measures
//!
//! Each measurement drives one FFI entry point with a complete JSON request
//! string and reads the envelope back, which is what a language binding pays
//! per call: parse the request, run the core, serialize the response. The
//! request strings are built once, outside every timed region, so the numbers
//! are the library's cost for fixed inputs. A live caller that serializes its
//! own values on every keystroke pays that on top, bounded separately by the
//! codec numbers (`benches/codecs.rs`: around twenty microseconds for a small
//! form).
//!
//! Every request is asserted to succeed once, outside the timed region, so the
//! numbers below are the happy path, not an error path.
//!
//! # What it does not measure
//!
//! Caller-side serialization, network, UI rendering, multi-threaded contention,
//! or repeater-heavy forms: both workloads are scalar, plus one component in
//! the compile path. A repeater/A2 scaling study is a follow-up.
//!
//! # Method
//!
//! `harness = false`: plain `fn main` with `Instant` and `black_box`, no
//! framework, no new dependency. Each measurement auto-scales a batch to last
//! around the target, runs warmup batches, then takes samples and divides.
//! Numbers are indicative, not authoritative: one process, one machine, no
//! statistical framework.

use std::ffi::c_char;
use std::hint::black_box;
use std::time::Instant;

use colander::ffi::compile::colander_compile;
use colander::ffi::response::colander_validate_response;
use colander::ffi::rules::colander_evaluate_rules;
use colander::ffi::session::call_with_text;
use colander::json;

/// Samples taken per measurement.
const SAMPLES: usize = 30;

/// Timed warmup batches run before sampling.
const WARMUP_BATCHES: usize = 2;

/// Base duration of one timed sample, in nanoseconds (2 ms).
const TARGET_BATCH_NANOS: u128 = 2_000_000;

/// Upper bound on the calibrated batch size.
const MAX_BATCH: u64 = 50_000;

type EntryFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;

fn measure(label: &str, operation: &str, request: &str, entry: EntryFn) {
    let probe = call_with_text(request, entry);
    assert!(
        probe.contains("\"ok\":true"),
        "setup failed for {label} {operation}: {probe}"
    );

    let start = Instant::now();
    for _ in 0..10 {
        black_box(call_with_text(request, entry));
    }
    let per_iter = start.elapsed().as_nanos().max(1) / 10;
    let batch = (TARGET_BATCH_NANOS / per_iter).clamp(1, u128::from(MAX_BATCH)) as u64;

    for _ in 0..WARMUP_BATCHES {
        for _ in 0..batch {
            black_box(call_with_text(request, entry));
        }
    }
    let mut samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        for _ in 0..batch {
            black_box(call_with_text(request, entry));
        }
        samples.push(start.elapsed().as_nanos() / u128::from(batch));
    }
    samples.sort_unstable();
    let min = samples[0] as f64 / 1000.0;
    let median = samples[SAMPLES / 2] as f64 / 1000.0;
    let mean = samples.iter().sum::<u128>() as f64 / SAMPLES as f64 / 1000.0;
    println!(
        "{label:8} {operation:17} batch {batch:>6}   min_us {min:>9.3}   median_us {median:>9.3}   mean_us {mean:>9.3}   ops/s {ops:>8.0}",
        ops = 1_000_000.0 / median
    );
}

/// A JSON string literal for embedding a document inside a request.
fn quoted(text: &str) -> String {
    let mut out = String::new();
    json::write_string(&mut out, text);
    out
}

struct Docs {
    form: String,
    ui: String,
    rules: String,
    values: String,
    answers: String,
}

/// A generated form with `field_count` scalar fields. Field 0 and 1 are
/// numbers to anchor calculations; every later number field calculates from
/// the previous numeric code, so the dependency graph is a chain with no
/// cycles. Calculated fields are read-only, every validation carries an
/// `assert`, and schema versions match, so the dependency check passes.
fn build_form(field_count: usize) -> Docs {
    let mut fields = Vec::new();
    let mut rules_fields = Vec::new();
    let mut layout = Vec::new();
    let mut values = Vec::new();
    let mut answers = Vec::new();
    let mut last_numeric: Option<String> = None;

    for index in 0..field_count {
        let id = format!("f{index}");
        let code = format!("c{index}");
        let field_type = if index < 2 {
            "number"
        } else {
            match index % 6 {
                0 => "text",
                1 => "integer",
                2 => "boolean",
                3 => "date",
                4 => "choice",
                _ => "number",
            }
        };

        let mut field = format!("{{\"id\":\"{id}\",\"code\":\"{code}\",\"type\":\"{field_type}\"");
        if field_type == "choice" {
            field.push_str(",\"options\":[{\"value\":\"v0\",\"label\":\"V0\"},{\"value\":\"v1\",\"label\":\"V1\"}]");
        }
        // Every tenth number field calculates from the previous numeric code.
        let calculates = field_type == "number" && index >= 2 && index % 10 == 2;
        if calculates {
            field.push_str(",\"readOnly\":true");
        }
        field.push('}');
        fields.push(field);

        if calculates {
            let previous = last_numeric.clone().expect("c0 and c1 seed the chain");
            rules_fields.push(format!(
                "\"{id}\":{{\"calculate\":{{\"op\":\"add\",\"args\":[{{\"ref\":\"{previous}\"}},{{\"lit\":1}}]}}}}"
            ));
        }
        if index % 5 == 0 && index > 0 {
            rules_fields.push(format!(
                "\"{id}\":{{\"visibleWhen\":{{\"op\":\"eq\",\"args\":[{{\"ref\":\"c0\"}},{{\"lit\":1}}]}}}}"
            ));
        }
        if index % 7 == 0 && index > 0 {
            rules_fields.push(format!(
                "\"{id}\":{{\"requiredWhen\":{{\"op\":\"eq\",\"args\":[{{\"ref\":\"c1\"}},{{\"lit\":2}}]}}}}"
            ));
        }
        if field_type == "number" {
            last_numeric = Some(code.clone());
        }

        layout.push(format!("{{\"type\":\"field\",\"fieldId\":\"{id}\"}}"));

        if calculates {
            continue;
        }
        let value = match field_type {
            "text" => "\"x\"".to_string(),
            "integer" => "3".to_string(),
            "boolean" => "true".to_string(),
            "date" => "\"2024-01-02\"".to_string(),
            "choice" => "\"v0\"".to_string(),
            _ => "1.5".to_string(),
        };
        values.push(format!("\"{code}\":{value}"));
        answers.push(format!("\"{code}\":{value}"));
    }

    let form = format!(
        "{{\"schemaVersion\":\"1.0.0\",\"fields\":[{}]}}",
        fields.join(",")
    );
    let ui = format!(
        "{{\"schemaVersion\":\"1.0.0\",\"formSchemaVersion\":\"1.0.0\",\"fields\":{{\"f5\":{{\"hidden\":true}}}},\"layout\":[{}]}}",
        layout.join(",")
    );
    let rules = format!(
        "{{\"schemaVersion\":\"1.0.0\",\"formSchemaVersion\":\"1.0.0\",\"fields\":{{{}}},\"validations\":[{{\"code\":\"V0\",\"assert\":{{\"op\":\"lte\",\"args\":[{{\"ref\":\"c0\"}},{{\"ref\":\"c1\"}}]}},\"message\":\"c0 above c1\"}},{{\"code\":\"V1\",\"assert\":{{\"op\":\"gte\",\"args\":[{{\"ref\":\"c1\"}},{{\"ref\":\"c0\"}}]}},\"message\":\"c1 below c0\"}}]}}",
        rules_fields.join(",")
    );
    Docs {
        form,
        ui,
        rules,
        values: format!("{{{}}}", values.join(",")),
        answers: format!("{{{}}}", answers.join(",")),
    }
}

/// The compile request: the form gains one `component-ref` field and the batch
/// carries its component, unpinned, exactly as a caller publishes it.
fn compile_request(docs: &Docs) -> String {
    let component_form = "{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"x\",\"code\":\"x\",\"type\":\"text\"}]}";
    let mut form_fields = docs
        .form
        .trim_start_matches("{\"schemaVersion\":\"1.0.0\",\"fields\":[");
    form_fields = form_fields.trim_end_matches("]}").trim_end_matches(']');
    let form = format!(
        "{{\"schemaVersion\":\"1.0.0\",\"fields\":[{form_fields}{comma}{{\"id\":\"compref\",\"code\":\"compref\",\"type\":\"component-ref\",\"componentCode\":\"comp\",\"componentVersion\":\"1.0.0\"}}]}}",
        comma = if form_fields.is_empty() { "" } else { "," }
    );
    let components = format!(
        "[{{\"code\":\"comp\",\"version\":\"1.0.0\",\"formSchemaJson\":{}}}]",
        quoted(component_form)
    );
    format!(
        "{{\"formSchemaJson\":{},\"uiSchemaJson\":{},\"rulesSchemaJson\":{},\"components\":{}}}",
        quoted(&form),
        quoted(&docs.ui),
        quoted(&docs.rules),
        components
    )
}

fn evaluate_request(docs: &Docs) -> String {
    format!(
        "{{\"formSchemaJson\":{},\"rulesSchemaJson\":{},\"uiSchemaJson\":{},\"values\":{}}}",
        quoted(&docs.form),
        quoted(&docs.rules),
        quoted(&docs.ui),
        docs.values
    )
}

fn validate_request(docs: &Docs, mode: &str) -> String {
    format!(
        "{{\"formSchemaJson\":{},\"answersJson\":{},\"mode\":\"{mode}\"}}",
        quoted(&docs.form),
        quoted(&docs.answers)
    )
}

fn main() {
    println!("colander engine benchmark");
    println!(
        "os={} arch={} samples={SAMPLES}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    println!(
        "label    operation           batch          min_us   median_us     mean_us      ops/s"
    );

    for (label, count) in [("small", 30), ("medium", 150)] {
        let docs = build_form(count);
        measure(
            label,
            "evaluate_rules",
            &evaluate_request(&docs),
            colander_evaluate_rules,
        );
        measure(
            label,
            "validate_draft",
            &validate_request(&docs, "Draft"),
            colander_validate_response,
        );
        measure(
            label,
            "validate_complete",
            &validate_request(&docs, "Complete"),
            colander_validate_response,
        );
        measure(label, "compile", &compile_request(&docs), colander_compile);
    }
}
