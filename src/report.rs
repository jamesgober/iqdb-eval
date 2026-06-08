//! Report types returned by the eval functions.
//!
//! The two report types ([`RecallReport`] and [`LatencyReport`]) are plain
//! data: numeric summaries of one measurement run. Both derive
//! `serde::Serialize` and `serde::Deserialize` when the crate's `serde`
//! feature is enabled, mirroring the gating pattern in
//! [`iqdb_types::Hit`].

/// Summary of a recall@k measurement against a known or computed
/// ground-truth set.
///
/// `mean_recall`, `min_recall`, and `max_recall` are aggregated across the
/// query set; each per-query recall is `|retrieved_topk ∩ true_topk| / k`
/// and lies in `[0.0, 1.0]`. Per-query values are not retained because
/// they grow O(n_queries) and the use cases at this version only need the
/// summary.
///
/// # Examples
///
/// ```
/// use iqdb_eval::RecallReport;
///
/// let r = RecallReport {
///     k: 10,
///     query_count: 100,
///     mean_recall: 0.97,
///     min_recall: 0.80,
///     max_recall: 1.00,
/// };
/// assert!(r.min_recall <= r.mean_recall && r.mean_recall <= r.max_recall);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RecallReport {
    /// The `k` used for the top-`k` queries that produced this report.
    pub k: usize,
    /// The number of queries the report was aggregated over.
    pub query_count: usize,
    /// Arithmetic mean of per-query recall across the query set.
    pub mean_recall: f64,
    /// Smallest per-query recall observed in the query set.
    pub min_recall: f64,
    /// Largest per-query recall observed in the query set.
    pub max_recall: f64,
}

/// Summary of a per-query latency measurement.
///
/// All latency values are reported in **microseconds**. Percentiles use
/// the **nearest-rank** method: for a sorted (ascending) sample of `n`
/// values, `p_q` is the value at index `clamp(ceil(q × n) − 1, 0, n − 1)`.
/// This matches Criterion and `hdrhistogram` defaults — every reported
/// percentile is an observed latency, never an interpolation.
///
/// `qps` is single-threaded throughput derived as
/// `query_count / sum_of_latencies_seconds`. Warm-up samples are excluded
/// from every field.
///
/// # Examples
///
/// ```
/// use iqdb_eval::LatencyReport;
///
/// let r = LatencyReport {
///     query_count: 1_000,
///     mean_us: 250.0, min_us: 100.0, max_us: 900.0,
///     p50_us: 220.0, p95_us: 600.0, p99_us: 850.0,
///     qps: 4_000.0,
/// };
/// assert!(r.p50_us <= r.p95_us);
/// assert!(r.p95_us <= r.p99_us);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LatencyReport {
    /// The number of queries the report was aggregated over (warm-up
    /// queries are not included).
    pub query_count: usize,
    /// Arithmetic mean of per-query latency, in microseconds.
    pub mean_us: f64,
    /// Smallest per-query latency observed, in microseconds.
    pub min_us: f64,
    /// Largest per-query latency observed, in microseconds.
    pub max_us: f64,
    /// 50th-percentile (median) per-query latency, in microseconds.
    pub p50_us: f64,
    /// 95th-percentile per-query latency, in microseconds.
    pub p95_us: f64,
    /// 99th-percentile per-query latency, in microseconds.
    pub p99_us: f64,
    /// Single-threaded throughput: `query_count / total_latency_seconds`.
    pub qps: f64,
}
