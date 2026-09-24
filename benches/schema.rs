//! Benchmark for the JSON Schema evaluation path (`colander::schema`).
//!
//! `harness = false`, plain `fn main` with `Instant` and `black_box`, no
//! framework and no new dependency — the same methodology as `engine.rs`.
//! The schema evaluator is a hot path in its own right: a wrapper that
//! validates a draft against a caller-supplied schema pays it per request, so
//! a structural change there needs a before/after number like any other.
//!
//! The cases are chosen to cover the work the evaluator actually does:
//! combinator branching, `$ref` resolution, `properties`/`items` descent, the
//! error-collection path, and the two deterministic bounds.

use std::hint::black_box;
use std::time::Instant;

use colander::schema::validate_text;

/// Samples taken per measurement.
const SAMPLES: usize = 30;

/// Timed warmup batches run before sampling.
const WARMUP_BATCHES: usize = 2;

/// Base duration of one timed sample, in nanoseconds (2 ms).
const TARGET_BATCH_NANOS: u128 = 2_000_000;

/// Upper bound on the calibrated batch size.
const MAX_BATCH: u64 = 50_000;

/// The shipped template against the whole golden-vectors corpus: the largest
/// legitimate workload in this repository.
const GOLDEN_SCHEMA: &str = include_str!("../schemas/golden-vectors.schema.json");
const GOLDEN_INSTANCE: &str = include_str!("../tests/golden/vectors/validate.json");

/// A chain of shared `$ref`s where each level lists the next reference twice.
/// Below the step budget this measures `2^depth` real evaluations; above it,
/// the bounded failure path.
fn shared_ref_chain(depth: usize) -> String {
    let mut definitions = String::new();
    for level in 0..depth {
        if level > 0 {
            definitions.push(',');
        }
        let next = format!("#/$defs/n{}", level + 1);
        definitions.push_str(&format!(
            r#""n{level}":{{"anyOf":[{{"$ref":"{next}"}},{{"$ref":"{next}"}}]}}"#
        ));
    }
    definitions.push_str(r#""leaf":{"type":"string"}"#);
    let mut schema = String::from(r##"{"$ref":"#/$defs/n0","$defs":{"##);
    schema.push_str(&definitions);
    schema.push_str("}}");
    schema
}

/// A flat object schema: the shape a real form schema has.
fn object_schema(fields: usize) -> String {
    let mut properties = String::new();
    for index in 0..fields {
        if index > 0 {
            properties.push(',');
        }
        properties.push_str(&format!(r#""f{index}":{{"type":"string","minLength":1}}"#));
    }
    let mut schema = String::from(r#"{"type":"object","properties":{"#);
    schema.push_str(&properties);
    schema.push_str("}}");
    schema
}

fn object_instance(fields: usize) -> String {
    let mut members = String::new();
    for index in 0..fields {
        if index > 0 {
            members.push(',');
        }
        members.push_str(&format!(r#""f{index}":"v{index}""#));
    }
    let mut instance = String::from("{");
    instance.push_str(&members);
    instance.push('}');
    instance
}

/// An array whose items run a two-branch combinator: schema work times
/// instance length, the multiplication the audit asked about.
fn array_case(items: usize) -> (String, String) {
    // `$defs` sits at the document root: a local reference is resolved
    // against the root, not against the subschema that mentions it.
    let schema = concat!(
        r#"{"$defs":{"alt":{"type":"number"}},"#,
        r#""type":"array","items":{"#,
        r##""anyOf":[{"type":"string"},{"$ref":"#/$defs/alt"}]}}"##,
    )
    .to_string();
    let mut elements = String::new();
    for index in 0..items {
        if index > 0 {
            elements.push(',');
        }
        elements.push('"');
        elements.push_str(&index.to_string());
        elements.push('"');
    }
    let instance = format!("[{elements}]");
    (schema, instance)
}

fn measure(label: &str, schema: &str, instance: &str) {
    // The outcome is not the measurement, but a silently broken setup would
    // make every number below meaningless, so assert it is a real decision.
    let valid = validate_text(schema, instance, "instance").is_ok();
    println!("{label:16} valid={valid}");

    let start = Instant::now();
    for _ in 0..10 {
        black_box(validate_text(schema, instance, "instance").ok());
    }
    let per_iter = start.elapsed().as_nanos().max(1) / 10;
    let batch = (TARGET_BATCH_NANOS / per_iter).clamp(1, u128::from(MAX_BATCH)) as u64;

    for _ in 0..WARMUP_BATCHES {
        for _ in 0..batch {
            black_box(validate_text(schema, instance, "instance").ok());
        }
    }
    let mut samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        for _ in 0..batch {
            black_box(validate_text(schema, instance, "instance").ok());
        }
        samples.push(start.elapsed().as_nanos() / u128::from(batch));
    }
    samples.sort_unstable();
    let min = samples[0] as f64 / 1000.0;
    let median = samples[SAMPLES / 2] as f64 / 1000.0;
    let mean = samples.iter().sum::<u128>() as f64 / SAMPLES as f64 / 1000.0;
    println!(
        "{label:16} batch {batch:>6}   min_us {min:>10.3}   median_us {median:>10.3}   mean_us {mean:>10.3}   ops/s {ops:>9.0}",
        ops = 1_000_000.0 / median
    );
}

fn main() {
    println!("colander schema evaluation benchmark");
    println!(
        "os={} arch={} samples={SAMPLES}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    println!(
        "{label:16} {batch:>11}   {min:>14}   {median:>14}   {mean:>14}   {ops:>13}",
        label = "case",
        batch = "",
        min = "min_us",
        median = "median_us",
        mean = "mean_us",
        ops = "ops/s"
    );

    // The largest legitimate workload in the repository.
    measure("golden-vectors", GOLDEN_SCHEMA, GOLDEN_INSTANCE);

    // Flat object descent: `properties` over N present keys.
    for fields in [10, 100] {
        let schema = object_schema(fields);
        let instance = object_instance(fields);
        measure(&format!("object-{fields}"), &schema, &instance);
    }

    // Combinator branching that stays inside the budget: 2^12 evaluations.
    for depth in [8, 12] {
        let schema = shared_ref_chain(depth);
        measure(&format!("shared-ref-{depth}"), &schema, "{}");
    }

    // Instance multiplication: two branches per element.
    for items in [1_000, 20_000] {
        let (schema, instance) = array_case(items);
        measure(&format!("array-{items}"), &schema, &instance);
    }

    // The bounded failure paths, which callers hit on hostile input.
    measure("bounded-anyOf-26", &shared_ref_chain(26), "{}");
    measure(
        "error-cap",
        r#"{"type":"array","items":false}"#,
        &format!("[{}]", vec!["null"; 1_500].join(",")),
    );
}
