//! End-to-end harness wiring test: build a `FlatIndex` from a tiny
//! synthetic base set, use it as both oracle AND index under test, and
//! verify that `recall_at_k_vs_oracle` reports `mean_recall == 1.0`.
//!
//! This is the harness's most basic correctness check: by construction,
//! a flat index is exact, so its recall against itself is one. If this
//! ever drops below 1.0, the bug is in the harness, not the index.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use iqdb_eval::{build_index_from_base, recall_at_k_vs_oracle};
use iqdb_flat::{FlatConfig, FlatIndex};
use iqdb_types::{DistanceMetric, SearchParams};

const DIM: usize = 8;
const N: usize = 32;
const QUERIES: usize = 16;
const K: usize = 5;

/// Tiny deterministic SplitMix64 — same constants as the existing
/// `iqdb-hnsw` recall tests; copied so the test stays self-contained.
struct Rng {
    state: u64,
}
impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn next_f32_unit(&mut self) -> f32 {
        let bits = (self.next_u64() >> 40) as u32;
        (bits as f32) / ((1_u32 << 24) as f32)
    }
}

fn make_rows(seed: u64, count: usize) -> Vec<Vec<f32>> {
    let mut rng = Rng::new(seed);
    (0..count)
        .map(|_| (0..DIM).map(|_| rng.next_f32_unit() * 2.0 - 1.0).collect())
        .collect()
}

#[test]
fn flat_vs_itself_recall_is_one() {
    let base = make_rows(0xC0DE_C0DE_C0DE_C0DE, N);
    let queries = make_rows(0xCAFE_BABE_DEAD_BEEF, QUERIES);

    let target: FlatIndex =
        build_index_from_base(FlatConfig, DIM, DistanceMetric::Euclidean, &base).unwrap();
    let oracle: FlatIndex =
        build_index_from_base(FlatConfig, DIM, DistanceMetric::Euclidean, &base).unwrap();
    let params = SearchParams::new(K, DistanceMetric::Euclidean);

    let report = recall_at_k_vs_oracle(&target, &oracle, &queries, &params).unwrap();
    assert_eq!(report.k, K);
    assert_eq!(report.query_count, QUERIES);
    assert_eq!(report.mean_recall, 1.0, "flat-vs-flat should be exact");
    assert_eq!(report.min_recall, 1.0);
    assert_eq!(report.max_recall, 1.0);
}

#[test]
fn flat_vs_itself_recall_across_all_metrics() {
    for metric in [
        DistanceMetric::Cosine,
        DistanceMetric::DotProduct,
        DistanceMetric::Euclidean,
        DistanceMetric::Manhattan,
    ] {
        let base = make_rows(0x1111_2222_3333_4444 ^ (metric as u64), N);
        let queries = make_rows(0x5555_6666_7777_8888 ^ (metric as u64), QUERIES);

        let target: FlatIndex = build_index_from_base(FlatConfig, DIM, metric, &base).unwrap();
        let oracle: FlatIndex = build_index_from_base(FlatConfig, DIM, metric, &base).unwrap();
        let params = SearchParams::new(K, metric);

        let report = recall_at_k_vs_oracle(&target, &oracle, &queries, &params).unwrap();
        assert_eq!(
            report.mean_recall, 1.0,
            "flat-vs-flat should be exact for {metric:?}",
        );
    }
}
