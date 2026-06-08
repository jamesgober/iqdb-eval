//! Tier-2: compute exact ground truth once, then evaluate recall at several
//! values of `k` — and across more than one index — without recomputing it.
//!
//! `compute_ground_truth` runs the exact oracle once at the largest `k` you
//! need; `recall_at_k` then reads only the first `params.k` ids of each row, so
//! one ground-truth set serves every smaller `k`. This is how you sweep the
//! recall/`k` curve cheaply.
//!
//! ```sh
//! cargo run --example precomputed_ground_truth
//! ```

use iqdb_eval::{build_index_from_base, compute_ground_truth, recall_at_k};
use iqdb_flat::{FlatConfig, FlatIndex};
use iqdb_types::{DistanceMetric, SearchParams};

const DIM: usize = 8;

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
    let base = rows(0x5EED, 512);
    let queries = rows(0xF00D, 64);

    let target: FlatIndex = build_index_from_base(FlatConfig, DIM, metric, &base)?;
    let oracle: FlatIndex = build_index_from_base(FlatConfig, DIM, metric, &base)?;

    // Compute exact ground truth ONCE, at the largest k we plan to evaluate.
    let max_k = 20;
    let gt = compute_ground_truth(&oracle, &queries, max_k)?;

    println!(" k   recall");
    println!("---  ------");
    for &k in &[1usize, 5, 10, 20] {
        let params = SearchParams::new(k, metric);
        let report = recall_at_k(&target, &queries, &gt, &params)?;
        println!("{k:>3}  {:.4}", report.mean_recall);
    }
    Ok(())
}
