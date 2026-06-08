//! Criterion benchmarks for the `iqdb-eval` measurement hot paths.
//!
//! The harness itself is thin — the cost of a measurement run is dominated by
//! the index `search` calls it drives. These benches isolate the harness
//! overhead on top of an exact `iqdb-flat` index so a regression in the
//! aggregation loops (recall set intersection, percentile sort) is visible
//! against a stable baseline.
//!
//! Run with:
//!
//! ```sh
//! cargo bench -p iqdb-eval
//! ```

#![allow(clippy::unwrap_used, clippy::expect_used)]

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use iqdb_eval::{
    LatencyConfig, build_index_from_base, compute_ground_truth, latency, recall_at_k,
    recall_at_k_vs_oracle,
};
use iqdb_flat::{FlatConfig, FlatIndex};
use iqdb_types::{DistanceMetric, SearchParams};

const DIM: usize = 16;

/// Deterministic SplitMix64 so benchmark inputs are identical across runs.
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
    fn unit(&mut self) -> f32 {
        let bits = (self.next_u64() >> 40) as u32;
        (bits as f32) / ((1_u32 << 24) as f32) * 2.0 - 1.0
    }
}

fn rows(seed: u64, count: usize) -> Vec<Vec<f32>> {
    let mut rng = Rng::new(seed);
    (0..count)
        .map(|_| (0..DIM).map(|_| rng.unit()).collect())
        .collect()
}

fn bench_recall(c: &mut Criterion) {
    let mut group = c.benchmark_group("recall_at_k");
    for &n in &[256usize, 1024] {
        let base = rows(0xA1, n);
        let queries = rows(0xB2, 64);
        let idx: FlatIndex =
            build_index_from_base(FlatConfig, DIM, DistanceMetric::Euclidean, &base).unwrap();
        let oracle: FlatIndex =
            build_index_from_base(FlatConfig, DIM, DistanceMetric::Euclidean, &base).unwrap();
        let params = SearchParams::new(10, DistanceMetric::Euclidean);
        let gt = compute_ground_truth(&oracle, &queries, 10).unwrap();

        group.bench_with_input(BenchmarkId::new("precomputed_gt", n), &n, |b, _| {
            b.iter(|| recall_at_k(black_box(&idx), &queries, &gt, &params).unwrap());
        });
        group.bench_with_input(BenchmarkId::new("vs_oracle", n), &n, |b, _| {
            b.iter(|| {
                recall_at_k_vs_oracle(black_box(&idx), black_box(&oracle), &queries, &params)
                    .unwrap()
            });
        });
    }
    group.finish();
}

fn bench_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency");
    let base = rows(0xC3, 1024);
    let queries = rows(0xD4, 128);
    let idx: FlatIndex =
        build_index_from_base(FlatConfig, DIM, DistanceMetric::Euclidean, &base).unwrap();
    let params = SearchParams::new(10, DistanceMetric::Euclidean);
    let cfg = LatencyConfig { warmup: 8 };
    group.bench_function("flat_1024", |b| {
        b.iter(|| latency(black_box(&idx), &queries, &params, &cfg).unwrap());
    });
    group.finish();
}

criterion_group!(benches, bench_recall, bench_latency);
criterion_main!(benches);
