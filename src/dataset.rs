//! Dataset loaders for the TEXMEX SIFT family.
//!
//! The SIFT corpus (and its `siftsmall` and `GIST` siblings) is shipped as
//! a pair of `.fvecs` files (base vectors and query vectors) and one
//! `.ivecs` file (per-query top-100 ground-truth neighbour ids). All three
//! share the same record layout: a little-endian `u32 dim` header
//! followed by `dim` payload elements (`f32` for `.fvecs`, `i32` for
//! `.ivecs`).
//!
//! The readers and [`load_sift_dataset`] are minimal, hand-rolled, and
//! pull in no new external parsing dependencies. They generalize the
//! one-off versions that previously lived in `iqdb-hnsw/tests/sift_recall.rs`.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use crate::error::{EvalError, Result};

/// One full SIFT-family dataset: base vectors, query vectors, per-query
/// ground-truth neighbour ids, and the shared dimensionality.
///
/// `base[i]` is the `i`-th base vector — `i` is also the row-index ID
/// used in [`crate::build_index_from_base`] and the value stored in the
/// `.ivecs` ground-truth entries.
#[derive(Debug, Clone)]
pub struct SiftDataset {
    /// The base vectors used to build the index under test.
    pub base: Vec<Vec<f32>>,
    /// The query vectors against which recall and latency are measured.
    pub queries: Vec<Vec<f32>>,
    /// Per-query exact top-`k` neighbour ids (ids index into `base`).
    pub ground_truth: Vec<Vec<u32>>,
    /// The dimensionality every base and query vector shares.
    pub dim: usize,
}

/// Upper bound on a single record's dimensionality, enforced by the `.fvecs`
/// and `.ivecs` readers.
///
/// A record's `u32 dim` header comes from an untrusted file: a corrupt or
/// hostile file can claim any value up to `u32::MAX`, which without a cap would
/// drive a single ~16 GiB allocation (`4 * u32::MAX` bytes) before the read
/// even fails. The largest real TEXMEX vectors (GIST) are 960-D, so the cap of
/// `2^20` is orders of magnitude above any legitimate dataset while bounding a
/// single record's scratch buffer to 4 MiB. A header above this returns
/// [`EvalError::Parse`].
const MAX_RECORD_DIM: usize = 1 << 20;

/// Read a length-prefixed TEXMEX record stream into one `Vec<T>` per record,
/// decoding each little-endian 4-byte payload word with `decode`.
///
/// Shared by [`read_fvecs`] and [`read_ivecs`], which differ only in how the
/// 4-byte words are interpreted. Centralizes the bounds check on the untrusted
/// per-record dimension (see [`MAX_RECORD_DIM`]) and the truncated-record
/// handling so both readers stay identical and hardened.
fn read_vecs<T, F>(path: &Path, truncated_reason: &'static str, decode: F) -> Result<Vec<Vec<T>>>
where
    F: Fn([u8; 4]) -> T,
{
    let file = File::open(path).map_err(|source| EvalError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut r = BufReader::new(file);
    let mut out: Vec<Vec<T>> = Vec::new();
    let mut dim_buf = [0u8; 4];
    loop {
        match r.read_exact(&mut dim_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(source) => {
                return Err(EvalError::Io {
                    path: path.to_path_buf(),
                    source,
                });
            }
        }
        let dim = u32::from_le_bytes(dim_buf) as usize;
        if dim > MAX_RECORD_DIM {
            return Err(EvalError::Parse {
                path: path.to_path_buf(),
                reason: "record dimension exceeds the maximum supported (file likely corrupt)",
            });
        }
        // `dim <= MAX_RECORD_DIM` (2^20), so `dim * 4` cannot overflow `usize`.
        let mut payload = vec![0u8; dim * 4];
        r.read_exact(&mut payload).map_err(|source| {
            if source.kind() == std::io::ErrorKind::UnexpectedEof {
                EvalError::Parse {
                    path: path.to_path_buf(),
                    reason: truncated_reason,
                }
            } else {
                EvalError::Io {
                    path: path.to_path_buf(),
                    source,
                }
            }
        })?;
        let row: Vec<T> = payload
            .chunks_exact(4)
            .map(|c| decode([c[0], c[1], c[2], c[3]]))
            .collect();
        out.push(row);
    }
    Ok(out)
}

/// Read a `.fvecs` file (TEXMEX corpus format) into one `Vec<f32>` per
/// record.
///
/// Each on-disk record is a little-endian `u32 dim` followed by `dim`
/// little-endian `f32` payload values. A truncated trailing record returns
/// [`EvalError::Parse`]; a record whose header claims a dimension above the
/// internal cap of `2^20` (treated as corruption) also returns
/// [`EvalError::Parse`]; an open or read failure returns [`EvalError::Io`].
///
/// # Examples
///
/// ```no_run
/// use iqdb_eval::read_fvecs;
///
/// # fn run() -> Result<(), iqdb_eval::EvalError> {
/// let rows = read_fvecs(".bench-data/siftsmall/siftsmall_base.fvecs")?;
/// assert!(!rows.is_empty());
/// # Ok(())
/// # }
/// ```
pub fn read_fvecs(path: impl AsRef<Path>) -> Result<Vec<Vec<f32>>> {
    read_vecs(
        path.as_ref(),
        "truncated fvecs record payload",
        f32::from_le_bytes,
    )
}

/// Read an `.ivecs` file (TEXMEX corpus format) into one `Vec<u32>` per
/// record.
///
/// Identical on-disk layout to [`read_fvecs`], but the payload is
/// little-endian `i32`. SIFT ground-truth ids are always non-negative
/// row indices, so `u32` is the natural fit; this reader does not check
/// for negative values. The same `2^20` dimension bound and
/// truncated-record handling as [`read_fvecs`] apply.
///
/// # Examples
///
/// ```no_run
/// use iqdb_eval::read_ivecs;
///
/// # fn run() -> Result<(), iqdb_eval::EvalError> {
/// let gt = read_ivecs(".bench-data/siftsmall/siftsmall_groundtruth.ivecs")?;
/// assert!(!gt.is_empty());
/// # Ok(())
/// # }
/// ```
pub fn read_ivecs(path: impl AsRef<Path>) -> Result<Vec<Vec<u32>>> {
    read_vecs(
        path.as_ref(),
        "truncated ivecs record payload",
        u32::from_le_bytes,
    )
}

/// Load a SIFT-family dataset rooted at `root` and named by `prefix`.
///
/// Resolves the canonical TEXMEX file names: `{prefix}_base.fvecs`,
/// `{prefix}_query.fvecs`, and `{prefix}_groundtruth.ivecs` directly
/// under `root`. For example, `load_sift_dataset(".bench-data/siftsmall",
/// "siftsmall")` reads `.bench-data/siftsmall/siftsmall_base.fvecs` and
/// its siblings.
///
/// Validates: every set is non-empty; every row in `base` and `queries`
/// has the same dimensionality; `queries.len() == ground_truth.len()`.
/// Returns [`EvalError::EmptyInput`], [`EvalError::DimensionMismatch`],
/// or [`EvalError::LengthMismatch`] accordingly.
///
/// # Examples
///
/// ```no_run
/// use iqdb_eval::load_sift_dataset;
///
/// # fn run() -> Result<(), iqdb_eval::EvalError> {
/// let dataset = load_sift_dataset(".bench-data/siftsmall", "siftsmall")?;
/// assert_eq!(dataset.queries.len(), dataset.ground_truth.len());
/// # Ok(())
/// # }
/// ```
pub fn load_sift_dataset(root: impl AsRef<Path>, prefix: &str) -> Result<SiftDataset> {
    let root = root.as_ref();
    let base_path: PathBuf = root.join(format!("{prefix}_base.fvecs"));
    let query_path: PathBuf = root.join(format!("{prefix}_query.fvecs"));
    let gt_path: PathBuf = root.join(format!("{prefix}_groundtruth.ivecs"));

    let base = read_fvecs(&base_path)?;
    let queries = read_fvecs(&query_path)?;
    let ground_truth = read_ivecs(&gt_path)?;

    if base.is_empty() {
        return Err(EvalError::EmptyInput { kind: "base" });
    }
    if queries.is_empty() {
        return Err(EvalError::EmptyInput { kind: "queries" });
    }
    if ground_truth.is_empty() {
        return Err(EvalError::EmptyInput {
            kind: "ground_truth",
        });
    }

    let dim = base[0].len();
    if let Some(row) = base.iter().find(|r| r.len() != dim) {
        return Err(EvalError::DimensionMismatch {
            expected: dim,
            found: row.len(),
        });
    }
    if let Some(row) = queries.iter().find(|r| r.len() != dim) {
        return Err(EvalError::DimensionMismatch {
            expected: dim,
            found: row.len(),
        });
    }
    if queries.len() != ground_truth.len() {
        return Err(EvalError::LengthMismatch {
            kind: "queries vs ground_truth",
            expected: queries.len(),
            found: ground_truth.len(),
        });
    }

    Ok(SiftDataset {
        base,
        queries,
        ground_truth,
        dim,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use std::fs;

    /// Encode rows in TEXMEX `.fvecs` layout: per record, a little-endian
    /// `u32` dimension followed by `dim` little-endian `f32` payload words.
    fn encode_fvecs(rows: &[&[f32]]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for row in rows {
            bytes.extend_from_slice(&(row.len() as u32).to_le_bytes());
            for &x in *row {
                bytes.extend_from_slice(&x.to_le_bytes());
            }
        }
        bytes
    }

    /// Encode rows in TEXMEX `.ivecs` layout (same header, `u32`/`i32`
    /// payload words).
    fn encode_ivecs(rows: &[&[u32]]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for row in rows {
            bytes.extend_from_slice(&(row.len() as u32).to_le_bytes());
            for &x in *row {
                bytes.extend_from_slice(&x.to_le_bytes());
            }
        }
        bytes
    }

    /// A unique temp path per test name; removed on drop so failures do not
    /// leak files. No timestamp/random source is used — the name is enough to
    /// keep parallel tests from colliding.
    struct TempFile(PathBuf);
    impl TempFile {
        fn new(name: &str, bytes: &[u8]) -> Self {
            let path = std::env::temp_dir().join(format!("iqdb_eval_{name}"));
            fs::write(&path, bytes).unwrap();
            Self(path)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn fvecs_round_trips() {
        let rows: &[&[f32]] = &[&[1.0, 2.0, 3.0], &[-4.5, 0.0, 9.25]];
        let f = TempFile::new("rt.fvecs", &encode_fvecs(rows));
        let got = read_fvecs(f.path()).unwrap();
        assert_eq!(got, vec![vec![1.0, 2.0, 3.0], vec![-4.5, 0.0, 9.25]]);
    }

    #[test]
    fn ivecs_round_trips() {
        let rows: &[&[u32]] = &[&[0, 1, 2], &[7, 8, 9]];
        let f = TempFile::new("rt.ivecs", &encode_ivecs(rows));
        let got = read_ivecs(f.path()).unwrap();
        assert_eq!(got, vec![vec![0u32, 1, 2], vec![7, 8, 9]]);
    }

    #[test]
    fn empty_file_reads_empty() {
        let f = TempFile::new("empty.fvecs", &[]);
        assert!(read_fvecs(f.path()).unwrap().is_empty());
    }

    #[test]
    fn truncated_payload_is_parse_error() {
        // Header claims dim=3 but only two floats follow.
        let mut bytes = 3u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&1.0f32.to_le_bytes());
        bytes.extend_from_slice(&2.0f32.to_le_bytes());
        let f = TempFile::new("trunc.fvecs", &bytes);
        let err = read_fvecs(f.path()).unwrap_err();
        assert!(matches!(err, EvalError::Parse { .. }), "got {err:?}");
    }

    #[test]
    fn trailing_partial_header_stops_cleanly() {
        // One full record, then two stray bytes (an incomplete next header).
        let mut bytes = encode_fvecs(&[&[1.0, 2.0]]);
        bytes.extend_from_slice(&[0xAB, 0xCD]);
        let f = TempFile::new("partial.fvecs", &bytes);
        let got = read_fvecs(f.path()).unwrap();
        assert_eq!(got, vec![vec![1.0, 2.0]]);
    }

    #[test]
    fn oversized_dim_is_rejected_without_allocating() {
        // A hostile header claiming a dimension above the cap must error before
        // attempting the (here, ~16 GiB) payload allocation.
        let bytes = u32::MAX.to_le_bytes().to_vec();
        let f = TempFile::new("huge.fvecs", &bytes);
        let err = read_fvecs(f.path()).unwrap_err();
        match err {
            EvalError::Parse { reason, .. } => {
                assert!(reason.contains("dimension"), "unexpected reason: {reason}");
            }
            other => panic!("expected Parse, got {other:?}"),
        }
    }

    #[test]
    fn dim_exactly_at_cap_is_accepted_in_header() {
        // The cap itself is allowed by the bound check; the read then fails as
        // a truncated payload (we do not write 4 MiB), proving the boundary is
        // inclusive and that rejection is by truncation, not by the cap.
        let bytes = (MAX_RECORD_DIM as u32).to_le_bytes().to_vec();
        let f = TempFile::new("atcap.fvecs", &bytes);
        let err = read_fvecs(f.path()).unwrap_err();
        assert!(
            matches!(&err, EvalError::Parse { reason, .. } if reason.contains("truncated")),
            "expected truncated-payload parse error, got {err:?}",
        );
    }

    #[test]
    fn missing_file_is_io_error() {
        let path = std::env::temp_dir().join("iqdb_eval_does_not_exist_xyz.fvecs");
        let err = read_fvecs(&path).unwrap_err();
        assert!(matches!(err, EvalError::Io { .. }), "got {err:?}");
    }
}
