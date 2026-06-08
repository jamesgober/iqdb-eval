//! # iqdb-eval
//!
//! Index-agnostic evaluation harness for the HiveDB **iqdb** vector-database
//! spine. Measures **recall@k** and **per-query latency percentiles** for
//! any type that implements [`iqdb_index::IndexCore`].
//!
//! ## Surface
//!
//! All measurements are top-level free functions generic over the index
//! under test, so a single harness call works against
//! [`iqdb_flat::FlatIndex`], an HNSW index, or any future index that
//! implements the same trait:
//!
//! - [`recall_at_k`] — recall@k for an index against an externally
//!   supplied `Vec<Vec<u32>>` ground truth (typically loaded from a SIFT
//!   `.ivecs` file via [`read_ivecs`]).
//! - [`recall_at_k_vs_oracle`] — convenience wrapper that takes a second
//!   `IndexCore` (typically [`iqdb_flat::FlatIndex`]) as the oracle and
//!   computes ground truth on the fly.
//! - [`compute_ground_truth`] — the oracle-only half: returns the per-query
//!   ground-truth ids as `Vec<Vec<u32>>`, matching the `.ivecs` shape.
//! - [`latency`] — collect per-query wall-clock samples and report
//!   mean / min / max / p50 / p95 / p99 (nearest-rank) and single-thread QPS.
//! - [`build_index_from_base`] — build a fresh index from a `&[Vec<f32>]`
//!   base set, inserting each row at `VectorId::U64(row_index)` so the
//!   resulting ids align with `.ivecs` ground-truth files.
//! - [`read_fvecs`] / [`read_ivecs`] / [`load_sift_dataset`] — TEXMEX
//!   SIFT-family loaders.
//!
//! ## Correctness invariants
//!
//! - **Row-index ↔ `VectorId::U64`.** [`build_index_from_base`] inserts each
//!   row of the base set at `VectorId::U64(row_index as u64)`. Callers that
//!   build oracles or indexes by hand must do the same; otherwise ids in
//!   `.ivecs` ground-truth cannot match the ids returned by `search`.
//! - **Latency excludes build cost.** [`latency`] takes a borrowed
//!   `&I`, so the index is constructed (and therefore paid for) before
//!   timing begins.
//! - **Percentiles are nearest-rank.** No interpolation; every reported
//!   percentile is an observed sample. See [`LatencyReport`].
//! - **Metric is read from the oracle.** [`compute_ground_truth`] derives
//!   the metric from `oracle.metric()` so a mismatched metric cannot
//!   silently corrupt the ground-truth set.
//!
//! ## Example
//!
//! ```
//! use iqdb_eval::{
//!     build_index_from_base, latency, recall_at_k_vs_oracle, LatencyConfig,
//! };
//! use iqdb_flat::{FlatConfig, FlatIndex};
//! use iqdb_types::{DistanceMetric, SearchParams};
//!
//! let base: Vec<Vec<f32>> = vec![
//!     vec![0.0, 0.0],
//!     vec![3.0, 4.0],
//!     vec![1.0, 1.0],
//! ];
//! let queries: Vec<Vec<f32>> = vec![vec![0.5, 0.5]];
//!
//! let target: FlatIndex =
//!     build_index_from_base(FlatConfig, 2, DistanceMetric::Euclidean, &base)?;
//! let oracle: FlatIndex =
//!     build_index_from_base(FlatConfig, 2, DistanceMetric::Euclidean, &base)?;
//! let params = SearchParams::new(2, DistanceMetric::Euclidean);
//!
//! let recall = recall_at_k_vs_oracle(&target, &oracle, &queries, &params)?;
//! assert_eq!(recall.mean_recall, 1.0);
//!
//! let lat = latency(&target, &queries, &params, &LatencyConfig::default())?;
//! assert!(lat.p50_us <= lat.p95_us);
//! # Ok::<(), iqdb_eval::EvalError>(())
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![deny(warnings)]
#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_must_use)]
#![deny(unused_results)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::print_stdout)]
#![deny(clippy::print_stderr)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::unreachable)]
#![deny(clippy::undocumented_unsafe_blocks)]

mod dataset;
mod error;
mod latency;
mod recall;
mod report;

use std::sync::Arc;

use iqdb_index::Index;
use iqdb_types::{DistanceMetric, VectorId};

pub use crate::dataset::{SiftDataset, load_sift_dataset, read_fvecs, read_ivecs};
pub use crate::error::{EvalError, Result};
pub use crate::latency::{LatencyConfig, latency};
pub use crate::recall::{compute_ground_truth, recall_at_k, recall_at_k_vs_oracle};
pub use crate::report::{LatencyReport, RecallReport};

/// The version of this crate, taken from `Cargo.toml` at compile time.
///
/// # Examples
///
/// ```
/// let v = iqdb_eval::VERSION;
/// assert_eq!(v.split('.').count(), 3);
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build a fresh index from a `&[Vec<f32>]` base set, inserting each row
/// at `VectorId::U64(row_index)`.
///
/// This is the harness's canonical way to construct both **the index under
/// test** and **the oracle** so the ids returned by `search` align with
/// row indices stored in `.ivecs` ground-truth files. The function is
/// generic over [`Index`], so any concrete index that implements the trait
/// (flat, HNSW, …) works.
///
/// Each base row is cloned into a fresh `Arc<[f32]>` so it can be handed to
/// [`iqdb_index::IndexCore::insert`]; that allocation is **O(N · dim)** and
/// is unavoidable given `insert`'s `Arc<[f32]>` signature.
///
/// # Errors
///
/// - [`EvalError::EmptyInput`] when `base` is empty.
/// - [`EvalError::DimensionMismatch`] when any row's `len()` differs from
///   `dim`.
/// - [`EvalError::Search`] when [`Index::new`] or
///   [`iqdb_index::IndexCore::insert`] returns an [`iqdb_types::IqdbError`].
///
/// # Examples
///
/// ```
/// use iqdb_eval::build_index_from_base;
/// use iqdb_flat::{FlatConfig, FlatIndex};
/// use iqdb_index::IndexCore;
/// use iqdb_types::DistanceMetric;
///
/// let base: Vec<Vec<f32>> = vec![vec![0.0, 0.0], vec![3.0, 4.0]];
/// let idx: FlatIndex =
///     build_index_from_base(FlatConfig, 2, DistanceMetric::Euclidean, &base)?;
/// assert_eq!(idx.len(), 2);
/// # Ok::<(), iqdb_eval::EvalError>(())
/// ```
pub fn build_index_from_base<I: Index>(
    config: I::Config,
    dim: usize,
    metric: DistanceMetric,
    base: &[Vec<f32>],
) -> Result<I> {
    if base.is_empty() {
        return Err(EvalError::EmptyInput { kind: "base" });
    }
    for row in base {
        if row.len() != dim {
            return Err(EvalError::DimensionMismatch {
                expected: dim,
                found: row.len(),
            });
        }
    }

    let span = tracing::info_span!("eval.build_index_from_base", n_base = base.len(), dim = dim,);
    let _enter = span.enter();

    let mut idx = I::new(dim, metric, config)?;
    for (i, row) in base.iter().enumerate() {
        let id = VectorId::U64(i as u64);
        let vector: Arc<[f32]> = Arc::from(row.as_slice());
        iqdb_index::IndexCore::insert(&mut idx, id, vector, None)?;
    }
    Ok(idx)
}
