//! Smoke test for the `serde` feature: build a sample of each report
//! type, serialize to JSON, deserialize, and assert equality.
//!
//! This test compiles to an empty binary when the `serde` feature is
//! off, so it's safe to leave in `tests/` regardless of the active
//! feature set.

#![allow(clippy::unwrap_used)]

#[cfg(feature = "serde")]
mod with_serde {
    use iqdb_eval::{LatencyReport, RecallReport};

    #[test]
    fn recall_report_round_trips() {
        let r = RecallReport {
            k: 10,
            query_count: 100,
            mean_recall: 0.97,
            min_recall: 0.80,
            max_recall: 1.00,
        };
        let json = serde_json::to_string(&r).unwrap();
        let r2: RecallReport = serde_json::from_str(&json).unwrap();
        assert_eq!(r, r2);
    }

    #[test]
    fn latency_report_round_trips() {
        let r = LatencyReport {
            query_count: 1_000,
            mean_us: 250.0,
            min_us: 100.0,
            max_us: 900.0,
            p50_us: 220.0,
            p95_us: 600.0,
            p99_us: 850.0,
            qps: 4_000.0,
        };
        let json = serde_json::to_string(&r).unwrap();
        let r2: LatencyReport = serde_json::from_str(&json).unwrap();
        assert_eq!(r, r2);
    }
}
