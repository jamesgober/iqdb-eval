//! The evaluation-harness domain error.
//!
//! [`EvalError`] names every failure mode the harness can surface. It
//! mirrors [`iqdb_types::IqdbError`]'s shape (non-exhaustive enum, one
//! variant per failure, [`error_forge::ForgeError`] integration) so the two
//! errors compose into the same operator-facing structured-error events.
//!
//! Unlike `IqdbError`, this type is **not** `Copy` or `Clone`: the `Io`
//! variant wraps a `std::io::Error` (used by the `.fvecs` / `.ivecs`
//! loaders) and `std::io::Error` does not implement either trait.

use std::path::PathBuf;

use error_forge::ForgeError;
use iqdb_types::IqdbError;

/// An error from an `iqdb-eval` measurement or dataset-loading operation.
///
/// Each variant identifies one specific failure. The enum is
/// `#[non_exhaustive]`: future releases may add variants without it being
/// a breaking change, so a `match` on it must include a wildcard arm.
///
/// # Examples
///
/// ```
/// use iqdb_eval::EvalError;
///
/// let err = EvalError::DimensionMismatch { expected: 128, found: 64 };
/// assert_eq!(
///     err.to_string(),
///     "vector dimension mismatch: expected 128, found 64",
/// );
///
/// let err = EvalError::KExceedsCorpus { k: 100, corpus_size: 10 };
/// assert_eq!(
///     err.to_string(),
///     "k exceeds corpus size: k=100, corpus_size=10",
/// );
/// ```
#[non_exhaustive]
#[derive(Debug)]
pub enum EvalError {
    /// An OS-level I/O failure occurred while reading a dataset file.
    /// `path` is the file whose read failed; `source` is the underlying
    /// `std::io::Error` and is reachable via [`std::error::Error::source`].
    Io {
        /// The path whose read or open call failed.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// A dataset file was opened successfully but its contents could not
    /// be parsed (truncated record, invalid header, etc.). `reason` is a
    /// short static description of the parser check that failed.
    Parse {
        /// The path of the file whose contents could not be parsed.
        path: PathBuf,
        /// Short static description of which parser check failed.
        reason: &'static str,
    },
    /// A vector did not have the dimensionality the operation required.
    /// `expected` is what was required; `found` is what was supplied.
    DimensionMismatch {
        /// The dimensionality the operation required.
        expected: usize,
        /// The dimensionality that was actually supplied.
        found: usize,
    },
    /// Two collections that had to share a length did not. `kind` names
    /// the pair (e.g. `"queries vs ground_truth"`); `expected` is the
    /// first collection's length; `found` is the second's.
    LengthMismatch {
        /// Short, stable identifier for the collection pair.
        kind: &'static str,
        /// The expected length (typically the first collection's `len()`).
        expected: usize,
        /// The actual length (typically the second collection's `len()`).
        found: usize,
    },
    /// The requested `k` exceeds the corpus size, so a `k`-nearest result
    /// cannot be returned.
    KExceedsCorpus {
        /// The requested top-k count.
        k: usize,
        /// The number of vectors searchable in the index.
        corpus_size: usize,
    },
    /// A required input collection was empty. `kind` names the collection
    /// (e.g. `"queries"`, `"base"`, `"ground_truth"`).
    EmptyInput {
        /// Short, stable identifier for the empty input.
        kind: &'static str,
    },
    /// A nested [`IqdbError`] surfaced from a downstream `IndexCore`
    /// operation (typically [`iqdb_index::IndexCore::insert`] or
    /// [`iqdb_index::IndexCore::search`]).
    Search(IqdbError),
    /// A `VectorId` shape that the harness cannot project to a `u32`
    /// row-index was encountered while computing ground truth. The
    /// convention is documented on [`crate::build_index_from_base`]:
    /// callers must insert each base row at `VectorId::U64(row_index)`.
    /// `found` is a short static identifier for the variant that was
    /// actually returned (for example, `"VectorId::Bytes"`).
    UnsupportedVectorId {
        /// Short identifier for the `VectorId` variant that was returned.
        found: &'static str,
    },
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "I/O error reading {}: {source}", path.display())
            }
            Self::Parse { path, reason } => {
                write!(f, "parse error in {}: {reason}", path.display())
            }
            Self::DimensionMismatch { expected, found } => {
                write!(
                    f,
                    "vector dimension mismatch: expected {expected}, found {found}",
                )
            }
            Self::LengthMismatch {
                kind,
                expected,
                found,
            } => {
                write!(
                    f,
                    "length mismatch ({kind}): expected {expected}, found {found}",
                )
            }
            Self::KExceedsCorpus { k, corpus_size } => {
                write!(f, "k exceeds corpus size: k={k}, corpus_size={corpus_size}")
            }
            Self::EmptyInput { kind } => write!(f, "empty input: {kind}"),
            Self::Search(e) => write!(f, "search failed: {e}"),
            Self::UnsupportedVectorId { found } => {
                write!(f, "unsupported VectorId variant for ground truth: {found}")
            }
        }
    }
}

impl std::error::Error for EvalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Search(e) => Some(e),
            _ => None,
        }
    }
}

impl ForgeError for EvalError {
    fn kind(&self) -> &'static str {
        match self {
            Self::Io { .. } => "Io",
            Self::Parse { .. } => "Parse",
            Self::DimensionMismatch { .. } => "DimensionMismatch",
            Self::LengthMismatch { .. } => "LengthMismatch",
            Self::KExceedsCorpus { .. } => "KExceedsCorpus",
            Self::EmptyInput { .. } => "EmptyInput",
            Self::Search(_) => "Search",
            Self::UnsupportedVectorId { .. } => "UnsupportedVectorId",
        }
    }

    fn caption(&self) -> &'static str {
        match self {
            Self::Io { .. } => "OS-level I/O failure reading a dataset file",
            Self::Parse { .. } => "dataset file could not be parsed",
            Self::DimensionMismatch { .. } => "vector dimension does not match the index",
            Self::LengthMismatch { .. } => "two collections that must share a length did not",
            Self::KExceedsCorpus { .. } => "requested top-k exceeds the corpus size",
            Self::EmptyInput { .. } => "a required input collection was empty",
            Self::Search(_) => "a downstream index operation returned an error",
            Self::UnsupportedVectorId { .. } => "ground truth requires VectorId::U64-shaped ids",
        }
    }
}

impl From<IqdbError> for EvalError {
    fn from(value: IqdbError) -> Self {
        Self::Search(value)
    }
}

/// A specialized [`Result`](core::result::Result) whose error is [`EvalError`].
///
/// # Examples
///
/// ```
/// use iqdb_eval::{EvalError, Result};
///
/// fn require_non_empty<T>(kind: &'static str, items: &[T]) -> Result<()> {
///     if items.is_empty() {
///         return Err(EvalError::EmptyInput { kind });
///     }
///     Ok(())
/// }
///
/// assert!(require_non_empty::<u8>("queries", &[]).is_err());
/// assert!(require_non_empty("queries", &[1u8, 2]).is_ok());
/// ```
pub type Result<T> = core::result::Result<T, EvalError>;
