//! Measure per-query latency percentiles and single-thread throughput.
//!
//! The index is built first, then borrowed — build cost is never part of the
//! timing. A short warm-up cycles the index before the measured loop begins so
//! one-shot first-call costs (cold caches, page faults) don't skew the tail.
//!
//! ```sh
//! cargo run --example latency_report --release
//! ```

use iqdb_eval::{LatencyConfig, build_index_from_base, latency};
use iqdb_flat::{FlatConfig, FlatIndex};
use iqdb_types::{DistanceMetric, SearchParams};

const DIM: usize = 16;

/// Tiny deterministic SplitMix64 so the example is reproducible.
fn rows(seed: u64, count: usize) -> Vec<Vec<f32>> {
    let mut state = seed;
    let mut next = || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        let bits = ((z ^ (z >> 31)) >> 40) as u32;
        (bits as f32) / ((1_u32 << 24) as f32) * 2.0 - 1.0
    };
    (0..count)
        .map(|_| (0..DIM).map(|_| next()).collect())
        .collect()
}

fn main() -> Result<(), iqdb_eval::EvalError> {
    let metric = DistanceMetric::Euclidean;
    let base = rows(0xC0DE, 4_096);
    let queries = rows(0xBEEF, 512);

    let idx: FlatIndex = build_index_from_base(FlatConfig, DIM, metric, &base)?;
    let params = SearchParams::new(10, metric);
    let cfg = LatencyConfig { warmup: 32 };

    let r = latency(&idx, &queries, &params, &cfg)?;

    println!("latency over {} queries (microseconds)", r.query_count);
    println!("  mean : {:.2}", r.mean_us);
    println!("  p50  : {:.2}", r.p50_us);
    println!("  p95  : {:.2}", r.p95_us);
    println!("  p99  : {:.2}", r.p99_us);
    println!("  max  : {:.2}", r.max_us);
    println!("  qps  : {:.0} (single thread)", r.qps);
    Ok(())
}
