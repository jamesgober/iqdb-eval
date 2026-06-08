//! End-to-end evaluation on a real SIFT-family dataset.
//!
//! Loads a TEXMEX SIFT dataset (base + query `.fvecs`, ground-truth `.ivecs`),
//! builds an exact `iqdb-flat` index, and reports recall@10 and latency against
//! the published ground truth. The example resolves the dataset under
//! `./.bench-data/siftsmall/` and prints a `SKIP` line (without failing) when
//! the data is not present, so it is always runnable.
//!
//! Fetch the small dataset first:
//!
//! ```sh
//! mkdir -p .bench-data && cd .bench-data
//! curl -O ftp://ftp.irisa.fr/local/texmex/corpus/siftsmall.tar.gz
//! tar -xzf siftsmall.tar.gz
//! cd .. && cargo run --example sift_eval --release
//! ```

use std::path::PathBuf;

use iqdb_eval::{LatencyConfig, build_index_from_base, latency, load_sift_dataset, recall_at_k};
use iqdb_flat::{FlatConfig, FlatIndex};
use iqdb_types::{DistanceMetric, SearchParams};

fn main() -> Result<(), iqdb_eval::EvalError> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".bench-data/siftsmall");
    if !root.exists() {
        println!("SKIP: dataset not found under {}", root.display());
        println!("Fetch siftsmall into .bench-data/ first (see the file header).");
        return Ok(());
    }

    let dataset = load_sift_dataset(&root, "siftsmall")?;
    println!(
        "loaded siftsmall: base={} queries={} dim={}",
        dataset.base.len(),
        dataset.queries.len(),
        dataset.dim,
    );

    let metric = DistanceMetric::Euclidean;
    let idx: FlatIndex = build_index_from_base(FlatConfig, dataset.dim, metric, &dataset.base)?;
    let params = SearchParams::new(10, metric);

    let recall = recall_at_k(&idx, &dataset.queries, &dataset.ground_truth, &params)?;
    println!(
        "recall@10 vs published ground truth: mean={:.4} (flat is exact, expect ~1.0)",
        recall.mean_recall,
    );

    let warmup = dataset.queries.len() / 10;
    let lat = latency(&idx, &dataset.queries, &params, &LatencyConfig { warmup })?;
    println!(
        "latency: p50={:.1}us p95={:.1}us p99={:.1}us qps={:.0}",
        lat.p50_us, lat.p95_us, lat.p99_us, lat.qps,
    );
    Ok(())
}
