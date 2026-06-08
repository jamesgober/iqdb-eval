//! Serialize a measurement report to JSON (requires the `serde` feature).
//!
//! With `--features serde`, `RecallReport` and `LatencyReport` derive
//! `serde::Serialize` / `Deserialize`, so a run's numbers can be persisted,
//! diffed across commits, or shipped to a dashboard.
//!
//! ```sh
//! cargo run --example serde_report --features serde
//! ```

#[cfg(feature = "serde")]
fn main() -> Result<(), iqdb_eval::EvalError> {
    use iqdb_eval::{LatencyConfig, build_index_from_base, latency, recall_at_k_vs_oracle};
    use iqdb_flat::{FlatConfig, FlatIndex};
    use iqdb_types::{DistanceMetric, SearchParams};

    let base: Vec<Vec<f32>> = vec![vec![0.0, 0.0], vec![3.0, 4.0], vec![1.0, 1.0]];
    let queries: Vec<Vec<f32>> = vec![vec![0.5, 0.5], vec![2.5, 3.5]];
    let metric = DistanceMetric::Euclidean;

    let target: FlatIndex = build_index_from_base(FlatConfig, 2, metric, &base)?;
    let oracle: FlatIndex = build_index_from_base(FlatConfig, 2, metric, &base)?;
    let params = SearchParams::new(2, metric);

    let recall = recall_at_k_vs_oracle(&target, &oracle, &queries, &params)?;
    let lat = latency(&target, &queries, &params, &LatencyConfig { warmup: 4 })?;

    // `expect` is fine in an example binary (not in the library).
    let recall_json = serde_json::to_string_pretty(&recall).expect("serialize recall");
    let lat_json = serde_json::to_string_pretty(&lat).expect("serialize latency");

    println!("recall report:\n{recall_json}\n");
    println!("latency report:\n{lat_json}");

    // Round-trip back to prove the reports are also `Deserialize`.
    let recall_back: iqdb_eval::RecallReport =
        serde_json::from_str(&recall_json).expect("deserialize recall");
    assert_eq!(recall_back, recall);
    Ok(())
}

#[cfg(not(feature = "serde"))]
fn main() {
    println!("This example needs the `serde` feature:");
    println!("    cargo run --example serde_report --features serde");
}
