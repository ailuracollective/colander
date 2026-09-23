//! Comparative benchmark: the JSON codec against the MessagePack codec on the
//! same documents.
//!
//! # Method
//!
//! This is a `harness = false` benchmark: a plain `fn main` that times
//! operations with [`std::time::Instant`] and feeds every input and result
//! through [`std::hint::black_box`]. It needs no benchmark framework and adds
//! no dependency. `#[bench]` is not used and would require nightly anyway.
//!
//! For every workload and codec the benchmark measures four operations:
//!
//! - `decode` — the codec's own encoded bytes back to [`Json`].
//! - `encode_ordered` — [`Json`] to bytes, document order.
//! - `encode_canonical` — [`Json`] to bytes, keys sorted. The same [`Json`]
//!   input feeds both encoders.
//! - `round_trip` — `decode` then `encode_ordered`, because a production
//!   request crosses the codec roughly twice.
//!
//! Each codec decodes its OWN encoding of the document. The document is built
//! once, each codec encodes it once, and the result is converted to `Vec<u8>`
//! once — all outside every timed region.
//!
//! # Sampling
//!
//! A `BENCH_SCALE` environment variable (default `1`) scales the target batch
//! duration, so a longer run is possible without changing the workload. Each
//! measurement first calibrates a batch size so that one timed sample lasts
//! around the target, then takes [`SAMPLES`] samples of that batch and divides
//! each batch duration by the batch size. The batch size is printed per row so
//! the run stays reproducible.
//!
//! # Limits
//!
//! The numbers are indicative, not authoritative. They come from one process
//! on one machine with no statistical framework, and the two codecs do
//! different work: the JSON decoder validates UTF-8 and the JSON writer escapes
//! every code point at or above U+007F as `\uXXXX`, while the MessagePack codec
//! copies its bytes through. Reading the numbers without that asymmetry in mind
//! is a mistake.

use std::hint::black_box;
use std::time::Instant;

use colander::codec::{Codec, json::JsonCodec, messagepack::MessagePackCodec};
use colander::json::{Json, JsonMap};

/// Samples taken per measurement. The task floor is 30; the extra samples buy a
/// little stability at almost no cost.
const SAMPLES: usize = 50;

/// Timed warmup batches run after calibration and before sampling.
const WARMUP_BATCHES: usize = 2;

/// Base duration of one timed sample, in nanoseconds (2 ms). Scaled by
/// `BENCH_SCALE`.
const TARGET_BATCH_NANOS: u128 = 2_000_000;

/// Upper bound on the calibrated batch size, so an accidentally free operation
/// cannot make the run loop forever.
const MAX_BATCH: u64 = 200_000;

/// Number of field objects in the two form documents.
const FIELD_COUNT: usize = 20;

/// Number of answer objects in the flat `rows` document.
const ANSWER_COUNT: usize = 2000;

// ---------------------------------------------------------------------------
// Documents
// ---------------------------------------------------------------------------

/// A realistic nested ASCII form: twenty field descriptors plus a `ui` object.
fn ascii_form() -> Json {
    let fields = (0..FIELD_COUNT)
        .map(|index| {
            let kind = match index % 3 {
                0 => "text",
                1 => "number",
                _ => "checkbox",
            };
            let mut field = JsonMap::new();
            field.insert("name".to_string(), Json::string(format!("field_{index}")));
            field.insert("type".to_string(), Json::string(kind));
            field.insert("required".to_string(), Json::Bool(index % 2 == 0));
            field.insert("order".to_string(), Json::integer(index as i64));
            Json::Object(field)
        })
        .collect();

    let mut ui = JsonMap::new();
    ui.insert("layout".to_string(), Json::string("vertical"));
    ui.insert("columns".to_string(), Json::integer(2));
    ui.insert("labelPosition".to_string(), Json::string("top"));

    let mut root = JsonMap::new();
    root.insert("title".to_string(), Json::string("Registration form"));
    root.insert("version".to_string(), Json::integer(3));
    root.insert("fields".to_string(), Json::array(fields));
    root.insert("ui".to_string(), Json::Object(ui));
    Json::Object(root)
}

/// The same shape with Spanish label text, accents, an inverted question mark
/// and an emoji. This is where the two formats diverge: the JSON writer escapes
/// every code point at or above U+007F as `\uXXXX`, while MessagePack stores
/// the UTF-8 bytes as they are.
fn non_ascii_form() -> Json {
    let labels = [
        "Año",
        "Dirección",
        "Teléfono",
        "¿Cómo estás?",
        "Descripción",
    ];
    let fields = (0..FIELD_COUNT)
        .map(|index| {
            let label = labels[index % labels.len()];
            let mut field = JsonMap::new();
            field.insert("name".to_string(), Json::string(format!("campo_{index}")));
            field.insert("label".to_string(), Json::string(label));
            field.insert("required".to_string(), Json::Bool(index % 2 == 0));
            field.insert("order".to_string(), Json::integer(index as i64));
            Json::Object(field)
        })
        .collect();

    let mut ui = JsonMap::new();
    ui.insert("layout".to_string(), Json::string("vertical"));
    ui.insert("labelPosition".to_string(), Json::string("arriba"));

    let mut root = JsonMap::new();
    root.insert(
        "title".to_string(),
        Json::string("Formulario de registro 🙂"),
    );
    root.insert("version".to_string(), Json::integer(3));
    root.insert("fields".to_string(), Json::array(fields));
    root.insert("ui".to_string(), Json::Object(ui));
    Json::Object(root)
}

/// A large flat document: an `answers` array of small objects, for throughput
/// at scale.
fn rows_document() -> Json {
    let answers = (0..ANSWER_COUNT)
        .map(|index| {
            let mut answer = JsonMap::new();
            answer.insert("id".to_string(), Json::string(format!("q{index}")));
            answer.insert("value".to_string(), Json::integer((index % 10) as i64));
            answer.insert("valid".to_string(), Json::Bool(index % 2 == 0));
            Json::Object(answer)
        })
        .collect();

    let mut root = JsonMap::new();
    root.insert("answers".to_string(), Json::array(answers));
    Json::Object(root)
}

// ---------------------------------------------------------------------------
// Measurement
// ---------------------------------------------------------------------------

/// Summary of one measurement, holding the calibrated batch size alongside the
/// per-operation timings in nanoseconds.
struct Stats {
    batch: u64,
    min: f64,
    median: f64,
    mean: f64,
}

impl Stats {
    fn from_samples(mut samples: Vec<f64>, batch: u64) -> Self {
        samples.sort_by(f64::total_cmp);
        let min = samples[0];
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let middle = samples.len() / 2;
        let median = if samples.len() % 2 == 1 {
            samples[middle]
        } else {
            (samples[middle - 1] + samples[middle]) / 2.0
        };
        Stats {
            batch,
            min,
            median,
            mean,
        }
    }

    /// Operations per second implied by the median sample.
    fn ops_per_second(&self) -> f64 {
        1e9 / self.median
    }
}

/// Grow the batch until one timed repetition is close to `target_nanos`.
fn calibrate(target_nanos: u128, operation: &mut impl FnMut()) -> u64 {
    let mut batch: u64 = 1;
    loop {
        let start = Instant::now();
        for _ in 0..batch {
            operation();
        }
        let elapsed = start.elapsed().as_nanos();
        if elapsed >= target_nanos || batch >= MAX_BATCH {
            return batch;
        }
        let ratio = (target_nanos as f64 / elapsed.max(1) as f64).max(1.5);
        let grown = ((batch as f64) * ratio).ceil() as u64;
        batch = grown.clamp(batch + 1, MAX_BATCH);
    }
}

/// Warm up, then take [`SAMPLES`] batch samples and report the per-operation
/// timings.
fn measure(mut operation: impl FnMut(), target_nanos: u128) -> Stats {
    let batch = calibrate(target_nanos, &mut operation);
    for _ in 0..WARMUP_BATCHES {
        for _ in 0..batch {
            operation();
        }
    }

    let mut samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        for _ in 0..batch {
            operation();
        }
        samples.push(start.elapsed().as_nanos() as f64 / batch as f64);
    }
    Stats::from_samples(samples, batch)
}

/// All four measurements for one codec over one document.
struct CodecTables {
    name: &'static str,
    encoded_len: usize,
    decode: Stats,
    encode_ordered: Stats,
    encode_canonical: Stats,
    round_trip: Stats,
}

fn bench_codec<C: Codec>(codec: &C, document: &Json, target_nanos: u128) -> CodecTables {
    let encoded: Vec<u8> = codec.encode_ordered(document).into();
    let decoded = codec
        .decode(&encoded, "benchmark document")
        .expect("the codec must decode its own encoding");
    assert_eq!(
        &decoded, document,
        "the codec changed the document on a round trip"
    );

    let decode = measure(
        || {
            let value = codec.decode(black_box(&encoded), black_box("benchmark document"));
            let _ = black_box(value);
        },
        target_nanos,
    );

    let encode_ordered = measure(
        || {
            let value = codec.encode_ordered(black_box(document));
            let _ = black_box(value);
        },
        target_nanos,
    );

    let encode_canonical = measure(
        || {
            let value = codec.encode_canonical(black_box(document));
            let _ = black_box(value);
        },
        target_nanos,
    );

    let round_trip = measure(
        || {
            let value = codec
                .decode(black_box(&encoded), black_box("benchmark document"))
                .expect("the codec must decode its own encoding");
            let value = codec.encode_ordered(black_box(&value));
            let _ = black_box(value);
        },
        target_nanos,
    );

    CodecTables {
        name: codec.name(),
        encoded_len: encoded.len(),
        decode,
        encode_ordered,
        encode_canonical,
        round_trip,
    }
}

// ---------------------------------------------------------------------------
// Reporting
// ---------------------------------------------------------------------------

fn print_header() {
    println!(
        "{:<12} {:<16} {:>8} {:>13} {:>13} {:>13} {:>17}",
        "codec", "operation", "batch", "min_us", "median_us", "mean_us", "ops/s"
    );
}

fn print_rows(tables: &CodecTables) {
    let rows = [
        ("decode", &tables.decode),
        ("encode_ordered", &tables.encode_ordered),
        ("encode_canonical", &tables.encode_canonical),
        ("round_trip", &tables.round_trip),
    ];
    for (operation, stats) in rows {
        println!(
            "{:<12} {:<16} {:>8} {:>13.3} {:>13.3} {:>13.3} {:>17.0}",
            tables.name,
            operation,
            stats.batch,
            stats.min / 1_000.0,
            stats.median / 1_000.0,
            stats.mean / 1_000.0,
            stats.ops_per_second()
        );
    }
}

fn bench_workload(name: &str, document: &Json, target_nanos: u128) {
    let json = bench_codec(&JsonCodec, document, target_nanos);
    let messagepack = bench_codec(&MessagePackCodec, document, target_nanos);

    println!();
    println!("=== workload: {name} ===");
    print_header();
    print_rows(&json);
    print_rows(&messagepack);

    let ratio = json.encoded_len as f64 / messagepack.encoded_len as f64;
    println!(
        "payload: json {} B, messagepack {} B, ratio json/messagepack = {:.3}",
        json.encoded_len, messagepack.encoded_len, ratio
    );
}

fn scale_from_env() -> u64 {
    match std::env::var("BENCH_SCALE") {
        Ok(value) => value.trim().parse().unwrap_or(1).max(1),
        Err(_) => 1,
    }
}

fn print_caveats() {
    println!();
    println!("Caveats");
    println!("-------");
    println!("- The numbers come from one process on one machine; a different");
    println!("  machine, allocator, or build profile will produce different figures.");
    println!("- No statistical framework is used. There is no confidence interval,");
    println!("  no hypothesis test, and no claim of statistical significance.");
    println!("- The two codecs do different amounts of work. The JSON decoder");
    println!("  validates UTF-8 before parsing, and the JSON writer escapes every");
    println!("  code point at or above U+007F as \\uXXXX. MessagePack copies its");
    println!("  bytes through. The comparison is indicative, not authoritative.");
    println!("- Prefer min (least interference) and median (robust to outliers)");
    println!("  over mean; a single scheduler preemption can inflate the mean.");
}

fn main() {
    let scale = scale_from_env();
    let target_nanos = TARGET_BATCH_NANOS * u128::from(scale);

    println!("colander codec benchmark");
    println!(
        "os={} arch={} samples={} scale={} target_batch_us={}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        SAMPLES,
        scale,
        target_nanos / 1_000
    );

    bench_workload("ascii_form", &ascii_form(), target_nanos);
    bench_workload("non_ascii_form", &non_ascii_form(), target_nanos);
    bench_workload("rows", &rows_document(), target_nanos);

    print_caveats();
}
