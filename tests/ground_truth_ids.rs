//! Ground truth is 32-bit; ids past `u32::MAX` must not be truncated.

#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use iqdb_eval::{EvalError, compute_ground_truth};
use iqdb_flat::{FlatConfig, FlatIndex};
use iqdb_index::{Index, IndexCore};
use iqdb_types::{DistanceMetric, VectorId};

#[test]
fn id_above_u32_max_is_rejected_not_truncated() {
    let mut flat = FlatIndex::new(1, DistanceMetric::Euclidean, FlatConfig).unwrap();
    let big = (1u64 << 32) + 5;
    flat.insert(VectorId::from(big), Arc::from(&[0.0f32][..]), None)
        .unwrap();
    flat.insert(VectorId::from(5u64), Arc::from(&[10.0f32][..]), None)
        .unwrap();
    // Truncated, the nearest id would have come back as 5.
    let err = compute_ground_truth(&flat, &[vec![0.0]], 1).unwrap_err();
    assert!(
        matches!(err, EvalError::UnsupportedVectorId { .. }),
        "{err:?}"
    );
}

#[test]
fn id_at_u32_max_is_kept() {
    let mut flat = FlatIndex::new(1, DistanceMetric::Euclidean, FlatConfig).unwrap();
    flat.insert(
        VectorId::from(u64::from(u32::MAX)),
        Arc::from(&[0.0f32][..]),
        None,
    )
    .unwrap();
    let gt = compute_ground_truth(&flat, &[vec![0.0]], 1).unwrap();
    assert_eq!(gt, vec![vec![u32::MAX]]);
}
