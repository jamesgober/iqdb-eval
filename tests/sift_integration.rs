//! Real-data harness validation on the TEXMEX SIFT corpus.
//!
//! This is the `#[ignore]`'d, opt-in integration test. It loads the
//! `siftsmall` dataset from `iqdb-eval/.bench-data/siftsmall/` (gitignored
//! at the workspace root), builds an `iqdb-hnsw` index under test plus an
//! `iqdb-flat` oracle, and runs `recall_at_k` and `latency` against the
//! published ground truth. Asserts the harness reproduces the recall
//! numbers we already measured in `iqdb-hnsw/tests/sift_recall.rs`.
//!
//! ## How to run
//!
//! 1. Fetch siftsmall into the gitignored bench-data directory:
//!
//!    ```sh
//!    mkdir -p iqdb-eval/.bench-data
//!    cd iqdb-eval/.bench-data
//!    curl -O ftp://ftp.irisa.fr/local/texmex/corpus/siftsmall.tar.gz
//!    tar -xzf siftsmall.tar.gz
//!    ```
//!
//! 2. Run from the workspace root:
//!
//!    ```sh
//!    cargo test -p iqdb-eval --test sift_integration \
//!        -- --include-ignored --nocapture
//!    ```
//!
//! If the dataset directory is missing the test prints a `SKIP` line and
//! returns; it does not fail. Mirrors the SKIP behaviour from
//! `iqdb-hnsw/tests/sift_recall.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use std::path::PathBuf;

use iqdb_eval::{LatencyConfig, build_index_from_base, latency, load_sift_dataset, recall_at_k};
use iqdb_flat::{FlatConfig, FlatIndex};
use iqdb_hnsw::{HnswConfig, HnswIndex};
use iqdb_types::{DistanceMetric, SearchParams};

const K: usize = 10;
const SIFTSMALL_RECALL_FLOOR: f64 = 0.95;

#[test]
#[ignore]
fn siftsmall_recall_and_latency_via_harness() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".bench-data/siftsmall");
    if !root.exists() {
        println!(
            "SKIP siftsmall: missing dataset under {}.\n  \
             Fetch with:\n    \
             mkdir -p iqdb-eval/.bench-data && cd iqdb-eval/.bench-data\n    \
             curl -O ftp://ftp.irisa.fr/local/texmex/corpus/siftsmall.tar.gz\n    \
             tar -xzf siftsmall.tar.gz",
            root.display(),
        );
        return;
    }

    let dataset = load_sift_dataset(&root, "siftsmall").unwrap();
    println!(
        "loaded siftsmall: base={} query={} gt={} dim={}",
        dataset.base.len(),
        dataset.queries.len(),
        dataset.ground_truth.len(),
        dataset.dim,
    );

    let metric = DistanceMetric::Euclidean;

    let oracle: FlatIndex =
        build_index_from_base(FlatConfig, dataset.dim, metric, &dataset.base).unwrap();
    let target: HnswIndex =
        build_index_from_base(HnswConfig::default(), dataset.dim, metric, &dataset.base).unwrap();

    let params = SearchParams::new(K, metric);

    // Recall vs the published .ivecs ground truth.
    let recall_loaded =
        recall_at_k(&target, &dataset.queries, &dataset.ground_truth, &params).unwrap();
    println!(
        "recall@10 (loaded gt):   mean={:.4}  min={:.4}  max={:.4}  n_queries={}",
        recall_loaded.mean_recall,
        recall_loaded.min_recall,
        recall_loaded.max_recall,
        recall_loaded.query_count,
    );
    assert!(
        recall_loaded.mean_recall >= SIFTSMALL_RECALL_FLOOR,
        "siftsmall recall@10 vs loaded gt = {:.4} < floor {:.4}",
        recall_loaded.mean_recall,
        SIFTSMALL_RECALL_FLOOR,
    );

    // Independent check: recall vs the flat oracle should produce a
    // similar number to the loaded ground truth (siftsmall ships the same
    // exact nearest neighbours flat would compute).
    let oracle_gt = iqdb_eval::compute_ground_truth(&oracle, &dataset.queries, K).unwrap();
    let recall_oracle = recall_at_k(&target, &dataset.queries, &oracle_gt, &params).unwrap();
    println!(
        "recall@10 (flat oracle): mean={:.4}  min={:.4}  max={:.4}",
        recall_oracle.mean_recall, recall_oracle.min_recall, recall_oracle.max_recall,
    );
    assert!(
        recall_oracle.mean_recall >= SIFTSMALL_RECALL_FLOOR,
        "siftsmall recall@10 vs flat oracle = {:.4} < floor {:.4}",
        recall_oracle.mean_recall,
        SIFTSMALL_RECALL_FLOOR,
    );

    // Latency: warm up with a small fraction of the query set, then
    // measure on the full set.
    let warmup = dataset.queries.len() / 10;
    let lat = latency(
        &target,
        &dataset.queries,
        &params,
        &LatencyConfig { warmup },
    )
    .unwrap();
    println!(
        "latency (us): mean={:.1}  p50={:.1}  p95={:.1}  p99={:.1}  qps={:.0}",
        lat.mean_us, lat.p50_us, lat.p95_us, lat.p99_us, lat.qps,
    );
    assert!(lat.p50_us <= lat.p95_us);
    assert!(lat.p95_us <= lat.p99_us);
    assert!(lat.qps > 0.0);
}
