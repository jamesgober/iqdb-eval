//! Tier-1 quickstart: measure recall@k for an index against an exact oracle.
//!
//! Build two indexes from the same synthetic base set — the *index under test*
//! and an exact `iqdb-flat` *oracle* — then ask the harness for recall@k. A
//! flat index is exact, so flat-vs-flat recall is `1.0`; swap the target for an
//! approximate index (`iqdb-hnsw`, `iqdb-ivf`) and the number tells you how much
//! recall that index trades for speed.
//!
//! ```sh
//! cargo run --example recall_quickstart
//! ```

use iqdb_eval::{build_index_from_base, recall_at_k_vs_oracle};
use iqdb_flat::{FlatConfig, FlatIndex};
use iqdb_types::{DistanceMetric, SearchParams};

fn main() -> Result<(), iqdb_eval::EvalError> {
    let base: Vec<Vec<f32>> = vec![
        vec![0.0, 0.0],
        vec![3.0, 4.0],
        vec![1.0, 1.0],
        vec![9.0, 9.0],
        vec![-2.0, 0.5],
    ];
    let queries: Vec<Vec<f32>> = vec![vec![0.5, 0.5], vec![8.0, 8.5]];

    let metric = DistanceMetric::Euclidean;
    let target: FlatIndex = build_index_from_base(FlatConfig, 2, metric, &base)?;
    let oracle: FlatIndex = build_index_from_base(FlatConfig, 2, metric, &base)?;

    let params = SearchParams::new(3, metric);
    let report = recall_at_k_vs_oracle(&target, &oracle, &queries, &params)?;

    println!("recall@{}", report.k);
    println!("  queries : {}", report.query_count);
    println!("  mean    : {:.4}", report.mean_recall);
    println!(
        "  min/max : {:.4} / {:.4}",
        report.min_recall, report.max_recall
    );

    assert_eq!(report.mean_recall, 1.0, "flat-vs-flat must be exact");
    Ok(())
}
