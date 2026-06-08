//! Property-based invariants the harness must hold across arbitrary
//! inputs.
//!
//! These properties are cheap by design (small corpus, small k, small
//! query set) so the `cargo test` default loop stays fast. The
//! correctness guarantees they exercise:
//!
//! - `recall_at_k_vs_oracle(flat, flat, …).mean_recall == 1.0` (a flat
//!   index is exact; recall against itself is one).
//! - `mean_recall`, `min_recall`, `max_recall` ∈ `[0.0, 1.0]`, ordered
//!   `min <= mean <= max`.
//! - `LatencyReport` percentiles ordered
//!   `min <= p50 <= p95 <= p99 <= max`.
//! - Empty query set → `EvalError::EmptyInput`.
//! - `k > corpus_size` → `EvalError::KExceedsCorpus`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use iqdb_eval::{
    EvalError, LatencyConfig, build_index_from_base, latency, recall_at_k, recall_at_k_vs_oracle,
};
use iqdb_flat::{FlatConfig, FlatIndex};
use iqdb_types::{DistanceMetric, SearchParams};
use proptest::prelude::*;

const DIM: usize = 4;

fn row_strategy() -> impl Strategy<Value = Vec<f32>> {
    proptest::collection::vec(-1.0_f32..1.0_f32, DIM)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 24, .. ProptestConfig::default() })]

    /// Self-recall: a flat index's recall against itself is always 1.0,
    /// regardless of corpus/query content.
    #[test]
    fn self_recall_is_one(
        base in proptest::collection::vec(row_strategy(), 4..16),
        queries in proptest::collection::vec(row_strategy(), 1..8),
    ) {
        let target: FlatIndex =
            build_index_from_base(FlatConfig, DIM, DistanceMetric::Euclidean, &base).unwrap();
        let oracle: FlatIndex =
            build_index_from_base(FlatConfig, DIM, DistanceMetric::Euclidean, &base).unwrap();
        let k = 3usize.min(base.len());
        let params = SearchParams::new(k, DistanceMetric::Euclidean);
        let report = recall_at_k_vs_oracle(&target, &oracle, &queries, &params).unwrap();
        prop_assert_eq!(report.mean_recall, 1.0);
        prop_assert_eq!(report.min_recall, 1.0);
        prop_assert_eq!(report.max_recall, 1.0);
    }

    /// Recall summary fields always lie in [0,1] and are ordered.
    #[test]
    fn recall_summary_bounds_and_order(
        base in proptest::collection::vec(row_strategy(), 4..16),
        queries in proptest::collection::vec(row_strategy(), 1..8),
    ) {
        let target: FlatIndex =
            build_index_from_base(FlatConfig, DIM, DistanceMetric::Euclidean, &base).unwrap();
        let oracle: FlatIndex =
            build_index_from_base(FlatConfig, DIM, DistanceMetric::Euclidean, &base).unwrap();
        let k = 3usize.min(base.len());
        let params = SearchParams::new(k, DistanceMetric::Euclidean);
        let r = recall_at_k_vs_oracle(&target, &oracle, &queries, &params).unwrap();
        prop_assert!(r.min_recall >= 0.0 && r.min_recall <= 1.0);
        prop_assert!(r.max_recall >= 0.0 && r.max_recall <= 1.0);
        prop_assert!(r.mean_recall >= 0.0 && r.mean_recall <= 1.0);
        prop_assert!(r.min_recall <= r.mean_recall);
        prop_assert!(r.mean_recall <= r.max_recall);
    }

    /// Latency report orders percentiles within `[min, max]`.
    #[test]
    fn latency_percentiles_ordered(
        base in proptest::collection::vec(row_strategy(), 4..16),
        queries in proptest::collection::vec(row_strategy(), 2..16),
    ) {
        let idx: FlatIndex =
            build_index_from_base(FlatConfig, DIM, DistanceMetric::Euclidean, &base).unwrap();
        let params = SearchParams::new(1, DistanceMetric::Euclidean);
        let report = latency(&idx, &queries, &params, &LatencyConfig::default()).unwrap();
        prop_assert!(report.min_us <= report.p50_us);
        prop_assert!(report.p50_us <= report.p95_us);
        prop_assert!(report.p95_us <= report.p99_us);
        prop_assert!(report.p99_us <= report.max_us);
        prop_assert!(report.min_us <= report.mean_us);
        prop_assert!(report.mean_us <= report.max_us);
        prop_assert!(report.qps > 0.0);
    }
}

#[test]
fn empty_queries_recall_at_k_errors() {
    let base: Vec<Vec<f32>> = vec![vec![0.0; DIM]];
    let oracle: FlatIndex =
        build_index_from_base(FlatConfig, DIM, DistanceMetric::Euclidean, &base).unwrap();
    let target: FlatIndex =
        build_index_from_base(FlatConfig, DIM, DistanceMetric::Euclidean, &base).unwrap();
    let queries: Vec<Vec<f32>> = vec![];
    let params = SearchParams::new(1, DistanceMetric::Euclidean);

    let err = recall_at_k_vs_oracle(&target, &oracle, &queries, &params).unwrap_err();
    assert!(matches!(err, EvalError::EmptyInput { kind } if kind == "queries"));
}

#[test]
fn empty_queries_latency_errors() {
    let base: Vec<Vec<f32>> = vec![vec![0.0; DIM]];
    let idx: FlatIndex =
        build_index_from_base(FlatConfig, DIM, DistanceMetric::Euclidean, &base).unwrap();
    let queries: Vec<Vec<f32>> = vec![];
    let params = SearchParams::new(1, DistanceMetric::Euclidean);

    let err = latency(&idx, &queries, &params, &LatencyConfig::default()).unwrap_err();
    assert!(matches!(err, EvalError::EmptyInput { kind } if kind == "queries"));
}

#[test]
fn k_exceeds_corpus_errors_on_recall_at_k() {
    let base: Vec<Vec<f32>> = vec![vec![0.0; DIM], vec![1.0; DIM]];
    let idx: FlatIndex =
        build_index_from_base(FlatConfig, DIM, DistanceMetric::Euclidean, &base).unwrap();
    let queries: Vec<Vec<f32>> = vec![vec![0.0; DIM]];
    let ground_truth: Vec<Vec<u32>> = vec![vec![0, 1, 2, 3, 4]];
    let params = SearchParams::new(5, DistanceMetric::Euclidean);

    let err = recall_at_k(&idx, &queries, &ground_truth, &params).unwrap_err();
    assert!(matches!(
        err,
        EvalError::KExceedsCorpus {
            k: 5,
            corpus_size: 2,
        },
    ));
}

#[test]
fn dimension_mismatch_errors() {
    let base: Vec<Vec<f32>> = vec![vec![0.0; DIM], vec![1.0; DIM]];
    let idx: FlatIndex =
        build_index_from_base(FlatConfig, DIM, DistanceMetric::Euclidean, &base).unwrap();
    let queries: Vec<Vec<f32>> = vec![vec![0.0; DIM + 1]];
    let params = SearchParams::new(1, DistanceMetric::Euclidean);
    let err = latency(&idx, &queries, &params, &LatencyConfig::default()).unwrap_err();
    assert!(matches!(
        err,
        EvalError::DimensionMismatch {
            expected,
            found,
        } if expected == DIM && found == DIM + 1
    ));
}
