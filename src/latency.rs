//! Per-query latency measurement and percentile reporting.
//!
//! [`latency`] runs every query through an already-built
//! [`IndexCore`](iqdb_index::IndexCore), records a per-query wall-clock
//! sample via [`std::time::Instant`], and reports the distribution as a
//! [`LatencyReport`]. Build cost is **not** part of any measurement: the
//! function takes `&I` (a borrowed, already-built index).

use std::time::Instant;

use iqdb_index::IndexCore;
use iqdb_types::SearchParams;

use crate::error::{EvalError, Result};
use crate::report::LatencyReport;

/// Controls how [`latency`] runs its measurement loop.
///
/// `warmup` is the number of queries to run before timing begins. Their
/// timings are discarded. This prevents one-shot first-call costs (cache
/// misses, page faults, JIT-like internal warm-up in some indexes) from
/// skewing the percentiles. A `warmup` of `8` is a reasonable default for
/// a small query set; for SIFT-scale runs a few hundred is typical.
#[derive(Debug, Clone, Copy, Default)]
pub struct LatencyConfig {
    /// The number of queries to run with timing discarded before the
    /// measured loop begins. The warm-up cycles through `queries` modulo
    /// `queries.len()` so a single query can warm an index repeatedly.
    /// Default: `0` (no warm-up).
    pub warmup: usize,
}

/// Measure per-query latency for `index` over `queries` and return a
/// [`LatencyReport`].
///
/// Each query is timed with [`Instant::now`] immediately before the call
/// to [`IndexCore::search`] and [`Instant::elapsed`] immediately after;
/// the resulting microsecond delta is recorded as one sample. Build cost
/// is excluded by construction — `index` is borrowed.
///
/// Percentiles use the nearest-rank method documented on
/// [`LatencyReport`]. Single-threaded QPS is computed as
/// `query_count / sum_of_latencies_seconds` and is therefore comparable
/// across runs with identical query counts.
///
/// # Errors
///
/// - [`EvalError::EmptyInput`] when `queries` is empty.
/// - [`EvalError::DimensionMismatch`] when any query has the wrong dim.
/// - [`EvalError::Search`] when the index's `search` returns an error.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
///
/// use iqdb_eval::{latency, LatencyConfig};
/// use iqdb_flat::{FlatConfig, FlatIndex};
/// use iqdb_index::{Index, IndexCore};
/// use iqdb_types::{DistanceMetric, SearchParams, VectorId};
///
/// let mut idx = FlatIndex::new(2, DistanceMetric::Euclidean, FlatConfig)?;
/// idx.insert(VectorId::from(0u64), Arc::<[f32]>::from(&[0.0, 0.0][..]), None)?;
/// idx.insert(VectorId::from(1u64), Arc::<[f32]>::from(&[3.0, 4.0][..]), None)?;
///
/// let queries = vec![vec![0.0, 0.0], vec![3.0, 4.0]];
/// let params = SearchParams::new(1, DistanceMetric::Euclidean);
/// let cfg = LatencyConfig { warmup: 1 };
///
/// let report = latency(&idx, &queries, &params, &cfg)?;
/// assert_eq!(report.query_count, 2);
/// assert!(report.p50_us <= report.p95_us);
/// assert!(report.p95_us <= report.p99_us);
/// # Ok::<(), iqdb_eval::EvalError>(())
/// ```
pub fn latency<I: IndexCore>(
    index: &I,
    queries: &[Vec<f32>],
    params: &SearchParams,
    config: &LatencyConfig,
) -> Result<LatencyReport> {
    if queries.is_empty() {
        return Err(EvalError::EmptyInput { kind: "queries" });
    }
    let dim = index.dim();
    for query in queries {
        if query.len() != dim {
            return Err(EvalError::DimensionMismatch {
                expected: dim,
                found: query.len(),
            });
        }
    }

    let span = tracing::info_span!(
        "eval.latency",
        n_queries = queries.len(),
        warmup = config.warmup,
    );
    let _enter = span.enter();

    for i in 0..config.warmup {
        let q = &queries[i % queries.len()];
        let _ = index.search(q, params)?;
    }

    let mut samples_us: Vec<f64> = Vec::with_capacity(queries.len());
    for query in queries {
        let t0 = Instant::now();
        let _hits = index.search(query, params)?;
        let elapsed_us = t0.elapsed().as_nanos() as f64 / 1_000.0;
        samples_us.push(elapsed_us);
    }

    Ok(summarize(&mut samples_us))
}

/// Sort `samples` in place and produce a [`LatencyReport`].
fn summarize(samples: &mut [f64]) -> LatencyReport {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n = samples.len();
    let sum: f64 = samples.iter().sum();
    let mean_us = sum / n as f64;
    let min_us = samples[0];
    let max_us = samples[n - 1];
    let p50_us = samples[percentile_index(0.50, n)];
    let p95_us = samples[percentile_index(0.95, n)];
    let p99_us = samples[percentile_index(0.99, n)];

    // QPS = n / total_seconds. `samples` are microseconds.
    let total_secs = sum / 1_000_000.0;
    let qps = if total_secs > 0.0 {
        n as f64 / total_secs
    } else {
        f64::INFINITY
    };

    LatencyReport {
        query_count: n,
        mean_us,
        min_us,
        max_us,
        p50_us,
        p95_us,
        p99_us,
        qps,
    }
}

/// Nearest-rank index for percentile `p` (in `[0.0, 1.0]`) over a sample
/// of size `n` (must be `>= 1`).
///
/// `idx = clamp(ceil(p × n) − 1, 0, n − 1)`. Returns an observed sample's
/// position; no interpolation is performed.
fn percentile_index(p: f64, n: usize) -> usize {
    debug_assert!(n >= 1, "percentile_index requires n >= 1");
    let raw = (p * n as f64).ceil() as isize - 1;
    if raw < 0 {
        0
    } else if (raw as usize) >= n {
        n - 1
    } else {
        raw as usize
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn percentile_index_typical_n100() {
        // ceil(0.5*100)-1 = 49, ceil(0.95*100)-1 = 94, ceil(0.99*100)-1 = 98.
        assert_eq!(percentile_index(0.50, 100), 49);
        assert_eq!(percentile_index(0.95, 100), 94);
        assert_eq!(percentile_index(0.99, 100), 98);
    }

    #[test]
    fn percentile_index_n1_returns_0() {
        assert_eq!(percentile_index(0.50, 1), 0);
        assert_eq!(percentile_index(0.99, 1), 0);
    }

    #[test]
    fn percentile_index_n10_p99_clamps_to_last() {
        // ceil(0.99*10)-1 = ceil(9.9)-1 = 9.
        assert_eq!(percentile_index(0.99, 10), 9);
    }

    #[test]
    fn summarize_orders_percentiles() {
        let mut samples = vec![5.0, 1.0, 4.0, 2.0, 3.0];
        let r = summarize(&mut samples);
        assert_eq!(r.query_count, 5);
        assert!(r.min_us <= r.p50_us);
        assert!(r.p50_us <= r.p95_us);
        assert!(r.p95_us <= r.p99_us);
        assert!(r.p99_us <= r.max_us);
        assert_eq!(r.min_us, 1.0);
        assert_eq!(r.max_us, 5.0);
    }

    #[test]
    fn summarize_single_sample_collapses_all_fields() {
        let mut samples = vec![42.0];
        let r = summarize(&mut samples);
        assert_eq!(r.query_count, 1);
        assert_eq!(r.min_us, 42.0);
        assert_eq!(r.max_us, 42.0);
        assert_eq!(r.p50_us, 42.0);
        assert_eq!(r.p95_us, 42.0);
        assert_eq!(r.p99_us, 42.0);
        assert!(r.qps > 0.0);
    }

    #[test]
    fn summarize_zero_total_time_yields_infinite_qps() {
        // A degenerate all-zero sample (e.g. a sub-nanosecond clock) must not
        // divide by zero — QPS saturates to infinity instead.
        let mut samples = vec![0.0, 0.0, 0.0];
        let r = summarize(&mut samples);
        assert!(r.qps.is_infinite());
    }

    mod with_index {
        #![allow(clippy::unwrap_used, clippy::expect_used)]

        use crate::{LatencyConfig, build_index_from_base, latency};
        use iqdb_flat::{FlatConfig, FlatIndex};
        use iqdb_types::{DistanceMetric, SearchParams};

        const M: DistanceMetric = DistanceMetric::Euclidean;

        fn index() -> FlatIndex {
            let base: Vec<Vec<f32>> = vec![vec![0.0], vec![1.0], vec![2.0]];
            build_index_from_base(FlatConfig, 1, M, &base).unwrap()
        }

        #[test]
        fn single_query_reports_one_sample() {
            let idx = index();
            let r = latency(
                &idx,
                &[vec![0.0]],
                &SearchParams::new(1, M),
                &LatencyConfig::default(),
            )
            .unwrap();
            assert_eq!(r.query_count, 1);
            assert_eq!(r.min_us, r.max_us);
            assert_eq!(r.p50_us, r.p99_us);
        }

        #[test]
        fn warmup_larger_than_query_set_cycles_and_excludes_itself() {
            let idx = index();
            // warmup of 10 over 2 queries cycles modulo the set; only the 2
            // measured queries are counted in query_count.
            let queries = vec![vec![0.0], vec![2.0]];
            let cfg = LatencyConfig { warmup: 10 };
            let r = latency(&idx, &queries, &SearchParams::new(1, M), &cfg).unwrap();
            assert_eq!(r.query_count, 2);
        }
    }
}
