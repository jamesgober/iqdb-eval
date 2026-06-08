//! Recall@k measurement against a known or computed ground-truth set.
//!
//! Three entry points:
//!
//! - [`recall_at_k`] — recall against an externally-supplied
//!   `Vec<Vec<u32>>` ground truth (e.g. loaded from a `.ivecs` file).
//! - [`compute_ground_truth`] — produce that ground truth by running a
//!   top-`k` search on an exact oracle (typically [`iqdb_flat::FlatIndex`]).
//! - [`recall_at_k_vs_oracle`] — convenience wrapper: compute ground
//!   truth from an oracle, then measure the index under test against it.
//!
//! Each entry point opens one `tracing::info_span!` so a calling harness
//! can correlate eval invocations with downstream `IndexCore` spans.

use std::collections::HashSet;

use iqdb_index::IndexCore;
use iqdb_types::{SearchParams, VectorId};

use crate::error::{EvalError, Result};
use crate::report::RecallReport;

/// Compute per-query top-`k` ground truth using `oracle`.
///
/// Returns a `Vec<Vec<u32>>` shaped exactly like the contents of a
/// TEXMEX `.ivecs` file: one row per query, each row containing the `k`
/// nearest neighbour ids (best first).
///
/// The metric is taken from [`IndexCore::metric`] on `oracle`; no metric
/// parameter is accepted because mismatching it would silently produce a
/// meaningless ground truth.
///
/// # Errors
///
/// - [`EvalError::EmptyInput`] when `queries` is empty or `k == 0`.
/// - [`EvalError::KExceedsCorpus`] when `k > oracle.len()`.
/// - [`EvalError::DimensionMismatch`] when any query has the wrong dim.
/// - [`EvalError::UnsupportedVectorId`] when the oracle returns a hit
///   whose id is not `VectorId::U64`.
/// - [`EvalError::Search`] when the oracle's `search` returns an error.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
///
/// use iqdb_eval::compute_ground_truth;
/// use iqdb_flat::{FlatConfig, FlatIndex};
/// use iqdb_index::{Index, IndexCore};
/// use iqdb_types::{DistanceMetric, VectorId};
///
/// let mut oracle = FlatIndex::new(2, DistanceMetric::Euclidean, FlatConfig)?;
/// oracle.insert(VectorId::from(0u64), Arc::<[f32]>::from(&[0.0, 0.0][..]), None)?;
/// oracle.insert(VectorId::from(1u64), Arc::<[f32]>::from(&[3.0, 4.0][..]), None)?;
///
/// let gt = compute_ground_truth(&oracle, &[vec![0.0, 0.0]], 1)?;
/// assert_eq!(gt, vec![vec![0u32]]);
/// # Ok::<(), iqdb_eval::EvalError>(())
/// ```
pub fn compute_ground_truth<O: IndexCore>(
    oracle: &O,
    queries: &[Vec<f32>],
    k: usize,
) -> Result<Vec<Vec<u32>>> {
    if queries.is_empty() {
        return Err(EvalError::EmptyInput { kind: "queries" });
    }
    if k == 0 {
        return Err(EvalError::EmptyInput { kind: "k" });
    }
    if k > oracle.len() {
        return Err(EvalError::KExceedsCorpus {
            k,
            corpus_size: oracle.len(),
        });
    }

    let span = tracing::info_span!(
        "eval.compute_ground_truth",
        k = k,
        n_queries = queries.len(),
    );
    let _enter = span.enter();

    let dim = oracle.dim();
    let params = SearchParams::new(k, oracle.metric());
    let mut out: Vec<Vec<u32>> = Vec::with_capacity(queries.len());
    for query in queries {
        if query.len() != dim {
            return Err(EvalError::DimensionMismatch {
                expected: dim,
                found: query.len(),
            });
        }
        let hits = oracle.search(query, &params)?;
        let mut row: Vec<u32> = Vec::with_capacity(hits.len());
        for h in hits {
            match h.id {
                VectorId::U64(u) => row.push(u as u32),
                _ => {
                    return Err(EvalError::UnsupportedVectorId {
                        found: "VectorId::Bytes",
                    });
                }
            }
        }
        out.push(row);
    }
    Ok(out)
}

/// Measure recall@k for `index` against an externally-supplied
/// `ground_truth`.
///
/// Per-query recall is `|retrieved_topk ∩ true_topk| / k`, where
/// `retrieved_topk` is `index.search(query, params)` truncated to
/// `params.k` and `true_topk` is the first `params.k` ids in the matching
/// `ground_truth` row. Returns the mean / min / max aggregated across the
/// query set in a [`RecallReport`].
///
/// Hits whose id is not `VectorId::U64` are treated as a miss (they
/// cannot match a `u32` ground-truth entry). The convention documented
/// on [`crate::build_index_from_base`] is that every base row is
/// inserted at `VectorId::U64(row_index)`.
///
/// # Errors
///
/// - [`EvalError::EmptyInput`] when `queries`, `ground_truth`, or
///   `params.k == 0`.
/// - [`EvalError::LengthMismatch`] when `queries.len() != ground_truth.len()`,
///   or when any ground-truth row holds fewer than `params.k` ids.
/// - [`EvalError::DimensionMismatch`] when any query has the wrong dim.
/// - [`EvalError::KExceedsCorpus`] when `params.k > index.len()`.
/// - [`EvalError::Search`] when the index's `search` returns an error.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
///
/// use iqdb_eval::recall_at_k;
/// use iqdb_flat::{FlatConfig, FlatIndex};
/// use iqdb_index::{Index, IndexCore};
/// use iqdb_types::{DistanceMetric, SearchParams, VectorId};
///
/// let mut idx = FlatIndex::new(2, DistanceMetric::Euclidean, FlatConfig)?;
/// idx.insert(VectorId::from(0u64), Arc::<[f32]>::from(&[0.0, 0.0][..]), None)?;
/// idx.insert(VectorId::from(1u64), Arc::<[f32]>::from(&[3.0, 4.0][..]), None)?;
///
/// let queries = vec![vec![0.0, 0.0]];
/// let ground_truth = vec![vec![0u32]];
/// let params = SearchParams::new(1, DistanceMetric::Euclidean);
///
/// let report = recall_at_k(&idx, &queries, &ground_truth, &params)?;
/// assert_eq!(report.mean_recall, 1.0);
/// # Ok::<(), iqdb_eval::EvalError>(())
/// ```
pub fn recall_at_k<I: IndexCore>(
    index: &I,
    queries: &[Vec<f32>],
    ground_truth: &[Vec<u32>],
    params: &SearchParams,
) -> Result<RecallReport> {
    if queries.is_empty() {
        return Err(EvalError::EmptyInput { kind: "queries" });
    }
    if ground_truth.is_empty() {
        return Err(EvalError::EmptyInput {
            kind: "ground_truth",
        });
    }
    if params.k == 0 {
        return Err(EvalError::EmptyInput { kind: "k" });
    }
    if queries.len() != ground_truth.len() {
        return Err(EvalError::LengthMismatch {
            kind: "queries vs ground_truth",
            expected: queries.len(),
            found: ground_truth.len(),
        });
    }
    if params.k > index.len() {
        return Err(EvalError::KExceedsCorpus {
            k: params.k,
            corpus_size: index.len(),
        });
    }

    let k = params.k;
    let span = tracing::info_span!("eval.recall_at_k", k = k, n_queries = queries.len());
    let _enter = span.enter();

    let dim = index.dim();
    let mut sum: f64 = 0.0;
    let mut min_recall: f64 = 1.0;
    let mut max_recall: f64 = 0.0;

    for (qi, query) in queries.iter().enumerate() {
        if query.len() != dim {
            return Err(EvalError::DimensionMismatch {
                expected: dim,
                found: query.len(),
            });
        }
        let gt_row = &ground_truth[qi];
        if gt_row.len() < k {
            return Err(EvalError::LengthMismatch {
                kind: "ground_truth row vs k",
                expected: k,
                found: gt_row.len(),
            });
        }
        let truth: HashSet<u64> = gt_row.iter().take(k).map(|&id| u64::from(id)).collect();
        let hits = index.search(query, params)?;
        let overlap = hits
            .iter()
            .filter(|h| matches!(&h.id, VectorId::U64(u) if truth.contains(u)))
            .count();
        let r = overlap as f64 / k as f64;
        sum += r;
        if r < min_recall {
            min_recall = r;
        }
        if r > max_recall {
            max_recall = r;
        }
    }

    let n = queries.len();
    Ok(RecallReport {
        k,
        query_count: n,
        mean_recall: sum / n as f64,
        min_recall,
        max_recall,
    })
}

/// Convenience wrapper: compute ground truth from `oracle`, then measure
/// `index` against it.
///
/// Equivalent to:
///
/// ```ignore
/// let gt = compute_ground_truth(oracle, queries, params.k)?;
/// recall_at_k(index, queries, &gt, params)?
/// ```
///
/// Both `index` and `oracle` must have been built with `VectorId::U64`
/// row-index ids (see [`crate::build_index_from_base`]); otherwise the
/// nested calls return [`EvalError::UnsupportedVectorId`] or report a
/// recall of zero.
///
/// # Examples
///
/// ```
/// use iqdb_eval::{build_index_from_base, recall_at_k_vs_oracle};
/// use iqdb_flat::{FlatConfig, FlatIndex};
/// use iqdb_types::{DistanceMetric, SearchParams};
///
/// let base: Vec<Vec<f32>> = vec![vec![0.0, 0.0], vec![3.0, 4.0]];
/// let queries: Vec<Vec<f32>> = vec![vec![0.0, 0.0]];
///
/// let target: FlatIndex =
///     build_index_from_base(FlatConfig, 2, DistanceMetric::Euclidean, &base)?;
/// let oracle: FlatIndex =
///     build_index_from_base(FlatConfig, 2, DistanceMetric::Euclidean, &base)?;
/// let params = SearchParams::new(1, DistanceMetric::Euclidean);
///
/// let report = recall_at_k_vs_oracle(&target, &oracle, &queries, &params)?;
/// assert_eq!(report.mean_recall, 1.0);
/// # Ok::<(), iqdb_eval::EvalError>(())
/// ```
pub fn recall_at_k_vs_oracle<I, O>(
    index: &I,
    oracle: &O,
    queries: &[Vec<f32>],
    params: &SearchParams,
) -> Result<RecallReport>
where
    I: IndexCore,
    O: IndexCore,
{
    let gt = compute_ground_truth(oracle, queries, params.k)?;
    recall_at_k(index, queries, &gt, params)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::build_index_from_base;
    use iqdb_flat::{FlatConfig, FlatIndex};
    use iqdb_types::DistanceMetric;

    const M: DistanceMetric = DistanceMetric::Euclidean;

    /// A small 1-D corpus whose exact ordering from the origin is unambiguous:
    /// ids 0,1,2,3 sit at distances 0,1,2,3.
    fn line_index() -> FlatIndex {
        let base: Vec<Vec<f32>> = vec![vec![0.0], vec![1.0], vec![2.0], vec![3.0]];
        build_index_from_base(FlatConfig, 1, M, &base).unwrap()
    }

    #[test]
    fn full_overlap_is_recall_one() {
        let idx = line_index();
        let queries = vec![vec![0.0]];
        // True top-3 from the origin are ids 0,1,2 — supply exactly those.
        let gt = vec![vec![0u32, 1, 2]];
        let report = recall_at_k(&idx, &queries, &gt, &SearchParams::new(3, M)).unwrap();
        assert_eq!(report.mean_recall, 1.0);
        assert_eq!(report.min_recall, 1.0);
        assert_eq!(report.max_recall, 1.0);
    }

    #[test]
    fn partial_overlap_is_fractional_recall() {
        let idx = line_index();
        let queries = vec![vec![0.0]];
        // Retrieved top-3 = {0,1,2}; supplied truth shares only {0,1} → 2/3.
        let gt = vec![vec![0u32, 1, 99]];
        let report = recall_at_k(&idx, &queries, &gt, &SearchParams::new(3, M)).unwrap();
        assert!((report.mean_recall - 2.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn disjoint_truth_is_zero_recall() {
        let idx = line_index();
        let queries = vec![vec![0.0]];
        let gt = vec![vec![97u32, 98, 99]];
        let report = recall_at_k(&idx, &queries, &gt, &SearchParams::new(3, M)).unwrap();
        assert_eq!(report.mean_recall, 0.0);
    }

    #[test]
    fn min_max_span_per_query_recall() {
        let idx = line_index();
        // Query A from origin retrieves {0,1,2}; truth {0,1,2} → 1.0.
        // Query B from origin retrieves {0,1,2}; truth {0,98,99} → 1/3.
        let queries = vec![vec![0.0], vec![0.0]];
        let gt = vec![vec![0u32, 1, 2], vec![0u32, 98, 99]];
        let report = recall_at_k(&idx, &queries, &gt, &SearchParams::new(3, M)).unwrap();
        assert_eq!(report.query_count, 2);
        assert_eq!(report.max_recall, 1.0);
        assert!((report.min_recall - 1.0 / 3.0).abs() < 1e-12);
        assert!((report.mean_recall - (1.0 + 1.0 / 3.0) / 2.0).abs() < 1e-12);
    }

    #[test]
    fn ground_truth_row_shorter_than_k_errors() {
        let idx = line_index();
        let queries = vec![vec![0.0]];
        let gt = vec![vec![0u32, 1]]; // only 2 ids, k = 3
        let err = recall_at_k(&idx, &queries, &gt, &SearchParams::new(3, M)).unwrap_err();
        assert!(matches!(
            err,
            EvalError::LengthMismatch {
                expected: 3,
                found: 2,
                ..
            }
        ));
    }

    #[test]
    fn compute_ground_truth_reads_metric_from_oracle() {
        let oracle = line_index();
        let gt = compute_ground_truth(&oracle, &[vec![0.0]], 2).unwrap();
        assert_eq!(gt, vec![vec![0u32, 1]]);
    }

    #[test]
    fn compute_ground_truth_rejects_k_zero() {
        let oracle = line_index();
        let err = compute_ground_truth(&oracle, &[vec![0.0]], 0).unwrap_err();
        assert!(matches!(err, EvalError::EmptyInput { kind } if kind == "k"));
    }
}
