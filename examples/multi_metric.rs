//! Compare latency across distance metrics on one corpus.
//!
//! The metric is part of both the index and the `SearchParams`, so a fair
//! benchmark fixes the data and varies only the metric. This example builds the
//! same base set under each metric and prints a small latency table — the shape
//! of a real "which metric costs what" comparison.
//!
//! ```sh
//! cargo run --example multi_metric --release
//! ```

use iqdb_eval::{LatencyConfig, build_index_from_base, latency};
use iqdb_flat::{FlatConfig, FlatIndex};
use iqdb_types::{DistanceMetric, SearchParams};

const DIM: usize = 32;

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
    let base = rows(0xABCD, 2_048);
    let queries = rows(0x1234, 256);
    let cfg = LatencyConfig { warmup: 16 };

    println!("metric        p50(us)  p95(us)   qps");
    println!("-----------   -------  -------  ------");
    for metric in [
        DistanceMetric::Euclidean,
        DistanceMetric::Cosine,
        DistanceMetric::DotProduct,
        DistanceMetric::Manhattan,
    ] {
        let idx: FlatIndex = build_index_from_base(FlatConfig, DIM, metric, &base)?;
        let params = SearchParams::new(10, metric);
        let r = latency(&idx, &queries, &params, &cfg)?;
        println!(
            "{:<11}  {:>7.2}  {:>7.2}  {:>6.0}",
            format!("{metric:?}"),
            r.p50_us,
            r.p95_us,
            r.qps,
        );
    }
    Ok(())
}
