# iqdb-eval &mdash; API Reference

> Complete reference for every public item in `iqdb-eval` v1.0.0, with
> descriptions, parameters, errors, and runnable examples.

`iqdb-eval` is the evaluation harness of the iQDB vector database. It measures
**recall@k**, **latency percentiles**, and **throughput** for any index behind
the `iqdb-index` `Index` / `IndexCore` traits, and uses `iqdb-flat` as the exact
oracle for ground truth.

---

## Table of Contents

- **[Installation](#installation)**
- **[Tiered API](#tiered-api)**
- **[Quick Start](#quick-start)**
- **[Public APIs](#public-apis)**
  - [`build_index_from_base`](#build_index_from_base)
  - [`compute_ground_truth`](#compute_ground_truth)
  - [`recall_at_k`](#recall_at_k)
  - [`recall_at_k_vs_oracle`](#recall_at_k_vs_oracle)
  - [`latency`](#latency)
  - [`LatencyConfig`](#latencyconfig)
  - [`RecallReport`](#recallreport)
  - [`LatencyReport`](#latencyreport)
  - [`read_fvecs`](#read_fvecs)
  - [`read_ivecs`](#read_ivecs)
  - [`load_sift_dataset`](#load_sift_dataset)
  - [`SiftDataset`](#siftdataset)
  - [`VERSION`](#version)
- **[Errors](#errors)**
- **[Conventions and invariants](#conventions-and-invariants)**
- **[Performance notes](#performance-notes)**
- **[Feature flags](#feature-flags)**

---

## Installation

```toml
[dependencies]
iqdb-eval = "1.0"
```

`iqdb-eval` re-exports nothing from its dependencies; bring the vocabulary in
directly. A typical consumer depends on the harness, the trait crate, the type
crate, and the oracle:

```toml
[dependencies]
iqdb-eval  = "1.0"
iqdb-flat  = "1.0"
iqdb-index = "1.0"
iqdb-types = "1.0"
```

MSRV is Rust **1.87** (edition 2024). The crate is `std`-only. The optional
`serde` feature derives `Serialize` / `Deserialize` on the report types.

---

## Tiered API

- **Tier 1 — the lazy path.** [`build_index_from_base`](#build_index_from_base)
  + [`recall_at_k_vs_oracle`](#recall_at_k_vs_oracle) + [`latency`](#latency)
  cover the whole common case in three calls.
- **Tier 2 — the configured path.** Precompute ground truth once with
  [`compute_ground_truth`](#compute_ground_truth) and reuse it across
  [`recall_at_k`](#recall_at_k); tune the timing loop with
  [`LatencyConfig`](#latencyconfig); load standard corpora with
  [`load_sift_dataset`](#load_sift_dataset) / [`read_fvecs`](#read_fvecs) /
  [`read_ivecs`](#read_ivecs).
- **Tier 3 — the trait seam.** Every measurement is generic over
  `iqdb_index::IndexCore` (and `Index` for construction), so any custom backend
  behind those traits is measurable with no extra wiring.

---

## Quick Start

```rust
use iqdb_eval::{build_index_from_base, latency, recall_at_k_vs_oracle, LatencyConfig};
use iqdb_flat::{FlatConfig, FlatIndex};
use iqdb_types::{DistanceMetric, SearchParams};

# fn main() -> Result<(), iqdb_eval::EvalError> {
let base: Vec<Vec<f32>> = vec![vec![0.0, 0.0], vec![3.0, 4.0], vec![1.0, 1.0]];
let queries: Vec<Vec<f32>> = vec![vec![0.5, 0.5]];
let metric = DistanceMetric::Euclidean;

let target: FlatIndex = build_index_from_base(FlatConfig, 2, metric, &base)?;
let oracle: FlatIndex = build_index_from_base(FlatConfig, 2, metric, &base)?;
let params = SearchParams::new(2, metric);

let recall = recall_at_k_vs_oracle(&target, &oracle, &queries, &params)?;
assert_eq!(recall.mean_recall, 1.0);

let lat = latency(&target, &queries, &params, &LatencyConfig::default())?;
assert!(lat.p50_us <= lat.p95_us);
# Ok(())
# }
```

---

## Public APIs

### `build_index_from_base`

```rust,ignore
pub fn build_index_from_base<I: Index>(
    config: I::Config,
    dim: usize,
    metric: DistanceMetric,
    base: &[Vec<f32>],
) -> Result<I>
```

Builds a fresh index from a base set, inserting each row at
`VectorId::U64(row_index)`. This is the canonical way to construct **both** the
index under test and the oracle, so the ids `search` returns line up with the
row indices stored in `.ivecs` ground-truth files. Generic over
[`iqdb_index::Index`], so any backend (flat, HNSW, IVF, …) works.

**Parameters**

| Name | Type | Meaning |
|------|------|---------|
| `config` | `I::Config` | the backend's configuration value (e.g. `FlatConfig`, `HnswConfig::default()`) |
| `dim` | `usize` | the dimensionality every base row must have |
| `metric` | `DistanceMetric` | the distance metric the index is built with |
| `base` | `&[Vec<f32>]` | the base vectors; row `i` is inserted at `VectorId::U64(i)` |

**Errors** — [`EvalError::EmptyInput`] when `base` is empty;
[`EvalError::DimensionMismatch`] when a row's length differs from `dim`;
[`EvalError::Search`] when the backend's `new` or `insert` fails.

```rust
use iqdb_eval::build_index_from_base;
use iqdb_flat::{FlatConfig, FlatIndex};
use iqdb_index::IndexCore;
use iqdb_types::DistanceMetric;

# fn main() -> Result<(), iqdb_eval::EvalError> {
let base = vec![vec![0.0, 0.0], vec![3.0, 4.0]];
let idx: FlatIndex = build_index_from_base(FlatConfig, 2, DistanceMetric::Euclidean, &base)?;
assert_eq!(idx.len(), 2);
# Ok(())
# }
```

---

### `compute_ground_truth`

```rust,ignore
pub fn compute_ground_truth<O: IndexCore>(
    oracle: &O,
    queries: &[Vec<f32>],
    k: usize,
) -> Result<Vec<Vec<u32>>>
```

Computes per-query top-`k` ground truth by running an exact search on `oracle`
(typically an [`iqdb_flat::FlatIndex`]). Returns a `Vec<Vec<u32>>` shaped exactly
like the contents of a `.ivecs` file: one row per query, each holding the `k`
nearest neighbour ids, best first. The metric is read from `oracle.metric()` —
no metric parameter is accepted, because mismatching it would silently corrupt
the ground truth.

**Parameters** — `oracle`: an exact index built with `VectorId::U64` ids;
`queries`: the query set; `k`: the number of neighbours per query.

**Errors** — [`EvalError::EmptyInput`] when `queries` is empty or `k == 0`;
[`EvalError::KExceedsCorpus`] when `k > oracle.len()`;
[`EvalError::DimensionMismatch`] when a query has the wrong dim;
[`EvalError::UnsupportedVectorId`] when the oracle returns a non-`U64` id;
[`EvalError::Search`] when the oracle's `search` fails.

```rust
use std::sync::Arc;
use iqdb_eval::compute_ground_truth;
use iqdb_flat::{FlatConfig, FlatIndex};
use iqdb_index::{Index, IndexCore};
use iqdb_types::{DistanceMetric, VectorId};

# fn main() -> Result<(), iqdb_eval::EvalError> {
let mut oracle = FlatIndex::new(2, DistanceMetric::Euclidean, FlatConfig)?;
oracle.insert(VectorId::from(0u64), Arc::<[f32]>::from(&[0.0, 0.0][..]), None)?;
oracle.insert(VectorId::from(1u64), Arc::<[f32]>::from(&[3.0, 4.0][..]), None)?;

let gt = compute_ground_truth(&oracle, &[vec![0.0, 0.0]], 1)?;
assert_eq!(gt, vec![vec![0u32]]);
# Ok(())
# }
```

---

### `recall_at_k`

```rust,ignore
pub fn recall_at_k<I: IndexCore>(
    index: &I,
    queries: &[Vec<f32>],
    ground_truth: &[Vec<u32>],
    params: &SearchParams,
) -> Result<RecallReport>
```

Measures recall@k for `index` against an externally-supplied `ground_truth`
(typically loaded from a `.ivecs` file or produced by
[`compute_ground_truth`](#compute_ground_truth)). Per-query recall is
`|retrieved_topk ∩ true_topk| / k`, where `true_topk` is the first `params.k`
ids of the matching ground-truth row. Returns the mean / min / max in a
[`RecallReport`](#recallreport). Hits whose id is not `VectorId::U64` count as a
miss.

**Parameters** — `index`: the index under test; `queries`: the query set;
`ground_truth`: one row of true neighbour ids per query (each at least `params.k`
long); `params`: search parameters (`params.k` is the recall `k`).

**Errors** — [`EvalError::EmptyInput`] (`queries`, `ground_truth`, or
`params.k == 0`); [`EvalError::LengthMismatch`] (`queries.len() !=
ground_truth.len()`, or a ground-truth row shorter than `params.k`);
[`EvalError::DimensionMismatch`]; [`EvalError::KExceedsCorpus`]
(`params.k > index.len()`); [`EvalError::Search`].

```rust
use std::sync::Arc;
use iqdb_eval::recall_at_k;
use iqdb_flat::{FlatConfig, FlatIndex};
use iqdb_index::{Index, IndexCore};
use iqdb_types::{DistanceMetric, SearchParams, VectorId};

# fn main() -> Result<(), iqdb_eval::EvalError> {
let mut idx = FlatIndex::new(2, DistanceMetric::Euclidean, FlatConfig)?;
idx.insert(VectorId::from(0u64), Arc::<[f32]>::from(&[0.0, 0.0][..]), None)?;
idx.insert(VectorId::from(1u64), Arc::<[f32]>::from(&[3.0, 4.0][..]), None)?;

let queries = vec![vec![0.0, 0.0]];
let ground_truth = vec![vec![0u32]];
let params = SearchParams::new(1, DistanceMetric::Euclidean);

let report = recall_at_k(&idx, &queries, &ground_truth, &params)?;
assert_eq!(report.mean_recall, 1.0);
# Ok(())
# }
```

---

### `recall_at_k_vs_oracle`

```rust,ignore
pub fn recall_at_k_vs_oracle<I, O>(
    index: &I,
    oracle: &O,
    queries: &[Vec<f32>],
    params: &SearchParams,
) -> Result<RecallReport>
where
    I: IndexCore,
    O: IndexCore,
```

Convenience wrapper: computes ground truth from `oracle`, then measures `index`
against it. Equivalent to `compute_ground_truth(oracle, queries, params.k)`
followed by `recall_at_k(index, queries, &gt, params)`. Both `index` and `oracle`
must be built with `VectorId::U64` row-index ids (see
[`build_index_from_base`](#build_index_from_base)).

**Errors** — the union of [`compute_ground_truth`](#compute_ground_truth) and
[`recall_at_k`](#recall_at_k).

```rust
use iqdb_eval::{build_index_from_base, recall_at_k_vs_oracle};
use iqdb_flat::{FlatConfig, FlatIndex};
use iqdb_types::{DistanceMetric, SearchParams};

# fn main() -> Result<(), iqdb_eval::EvalError> {
let base = vec![vec![0.0, 0.0], vec![3.0, 4.0]];
let queries = vec![vec![0.0, 0.0]];

let target: FlatIndex = build_index_from_base(FlatConfig, 2, DistanceMetric::Euclidean, &base)?;
let oracle: FlatIndex = build_index_from_base(FlatConfig, 2, DistanceMetric::Euclidean, &base)?;
let params = SearchParams::new(1, DistanceMetric::Euclidean);

let report = recall_at_k_vs_oracle(&target, &oracle, &queries, &params)?;
assert_eq!(report.mean_recall, 1.0);
# Ok(())
# }
```

---

### `latency`

```rust,ignore
pub fn latency<I: IndexCore>(
    index: &I,
    queries: &[Vec<f32>],
    params: &SearchParams,
    config: &LatencyConfig,
) -> Result<LatencyReport>
```

Measures per-query latency over `queries` and returns a
[`LatencyReport`](#latencyreport). Each query is timed with `Instant::now` /
`elapsed` around a single `search` call. Build cost is excluded by construction:
`index` is borrowed, already built. Percentiles are nearest-rank; single-thread
QPS is `query_count / sum_of_latencies_seconds`. The `config.warmup` queries run
first with their timings discarded.

**Errors** — [`EvalError::EmptyInput`] when `queries` is empty;
[`EvalError::DimensionMismatch`]; [`EvalError::Search`].

```rust
use std::sync::Arc;
use iqdb_eval::{latency, LatencyConfig};
use iqdb_flat::{FlatConfig, FlatIndex};
use iqdb_index::{Index, IndexCore};
use iqdb_types::{DistanceMetric, SearchParams, VectorId};

# fn main() -> Result<(), iqdb_eval::EvalError> {
let mut idx = FlatIndex::new(2, DistanceMetric::Euclidean, FlatConfig)?;
idx.insert(VectorId::from(0u64), Arc::<[f32]>::from(&[0.0, 0.0][..]), None)?;
idx.insert(VectorId::from(1u64), Arc::<[f32]>::from(&[3.0, 4.0][..]), None)?;

let queries = vec![vec![0.0, 0.0], vec![3.0, 4.0]];
let params = SearchParams::new(1, DistanceMetric::Euclidean);
let report = latency(&idx, &queries, &params, &LatencyConfig { warmup: 1 })?;
assert_eq!(report.query_count, 2);
assert!(report.p95_us <= report.p99_us);
# Ok(())
# }
```

---

### `LatencyConfig`

```rust,ignore
#[derive(Debug, Clone, Copy, Default)]
pub struct LatencyConfig {
    pub warmup: usize,
}
```

Controls how [`latency`](#latency) runs. `warmup` is the number of queries to run
with timing discarded before the measured loop begins; it cycles through
`queries` modulo `queries.len()`. Default `0` (no warm-up). A few hundred is
typical at SIFT scale.

```rust
use iqdb_eval::LatencyConfig;
let cfg = LatencyConfig { warmup: 128 };
assert_eq!(cfg.warmup, 128);
let default = LatencyConfig::default();
assert_eq!(default.warmup, 0);
```

---

### `RecallReport`

```rust,ignore
#[derive(Debug, Clone, PartialEq)]
pub struct RecallReport {
    pub k: usize,
    pub query_count: usize,
    pub mean_recall: f64,
    pub min_recall: f64,
    pub max_recall: f64,
}
```

Summary of a recall@k run. Each per-query recall lies in `[0.0, 1.0]` and the
fields are ordered `min_recall <= mean_recall <= max_recall`. Per-query values
are not retained (they grow `O(n_queries)`). Derives `serde` traits under the
`serde` feature.

```rust
use iqdb_eval::RecallReport;
let r = RecallReport { k: 10, query_count: 100, mean_recall: 0.97, min_recall: 0.80, max_recall: 1.0 };
assert!(r.min_recall <= r.mean_recall && r.mean_recall <= r.max_recall);
```

---

### `LatencyReport`

```rust,ignore
#[derive(Debug, Clone, PartialEq)]
pub struct LatencyReport {
    pub query_count: usize,
    pub mean_us: f64,
    pub min_us: f64,
    pub max_us: f64,
    pub p50_us: f64,
    pub p95_us: f64,
    pub p99_us: f64,
    pub qps: f64,
}
```

Summary of a latency run. All latencies are **microseconds**. Percentiles are
nearest-rank — for a sorted ascending sample of `n` values, `p_q` is the value at
index `clamp(ceil(q × n) − 1, 0, n − 1)`, never an interpolation. `qps` is
single-threaded throughput; warm-up samples are excluded from every field.
Derives `serde` traits under the `serde` feature.

```rust
use iqdb_eval::LatencyReport;
let r = LatencyReport {
    query_count: 1_000, mean_us: 250.0, min_us: 100.0, max_us: 900.0,
    p50_us: 220.0, p95_us: 600.0, p99_us: 850.0, qps: 4_000.0,
};
assert!(r.p50_us <= r.p95_us && r.p95_us <= r.p99_us);
```

---

### `read_fvecs`

```rust,ignore
pub fn read_fvecs(path: impl AsRef<Path>) -> Result<Vec<Vec<f32>>>
```

Reads a TEXMEX `.fvecs` file into one `Vec<f32>` per record. Each on-disk record
is a little-endian `u32 dim` header followed by `dim` little-endian `f32` values.
The per-record dimension comes from an untrusted file, so it is bounded: a header
claiming a dimension above `2^20` is rejected as corruption *before* any payload
is allocated, capping a single record's scratch buffer at 4 MiB.

**Errors** — [`EvalError::Io`] on open/read failure; [`EvalError::Parse`] on a
truncated trailing record or a dimension header above the bound.

```rust,no_run
use iqdb_eval::read_fvecs;
# fn run() -> Result<(), iqdb_eval::EvalError> {
let rows = read_fvecs(".bench-data/siftsmall/siftsmall_base.fvecs")?;
assert!(!rows.is_empty());
# Ok(())
# }
```

---

### `read_ivecs`

```rust,ignore
pub fn read_ivecs(path: impl AsRef<Path>) -> Result<Vec<Vec<u32>>>
```

Reads a TEXMEX `.ivecs` file into one `Vec<u32>` per record. Identical layout to
[`read_fvecs`](#read_fvecs) but with little-endian `i32` payloads (SIFT ids are
non-negative row indices, so `u32` is the natural fit). The same `2^20`
dimension bound applies.

**Errors** — [`EvalError::Io`]; [`EvalError::Parse`] (truncated record or a
dimension header above the bound).

```rust,no_run
use iqdb_eval::read_ivecs;
# fn run() -> Result<(), iqdb_eval::EvalError> {
let gt = read_ivecs(".bench-data/siftsmall/siftsmall_groundtruth.ivecs")?;
assert!(!gt.is_empty());
# Ok(())
# }
```

---

### `load_sift_dataset`

```rust,ignore
pub fn load_sift_dataset(root: impl AsRef<Path>, prefix: &str) -> Result<SiftDataset>
```

Loads a full SIFT-family dataset rooted at `root` and named by `prefix`. Resolves
`{prefix}_base.fvecs`, `{prefix}_query.fvecs`, and `{prefix}_groundtruth.ivecs`
directly under `root`, then validates: every set non-empty; uniform
dimensionality across base and queries; `queries.len() == ground_truth.len()`.

**Errors** — [`EvalError::Io`] / [`EvalError::Parse`] from the readers;
[`EvalError::EmptyInput`]; [`EvalError::DimensionMismatch`];
[`EvalError::LengthMismatch`].

```rust,no_run
use iqdb_eval::load_sift_dataset;
# fn run() -> Result<(), iqdb_eval::EvalError> {
let data = load_sift_dataset(".bench-data/siftsmall", "siftsmall")?;
assert_eq!(data.queries.len(), data.ground_truth.len());
# Ok(())
# }
```

---

### `SiftDataset`

```rust,ignore
#[derive(Debug, Clone)]
pub struct SiftDataset {
    pub base: Vec<Vec<f32>>,
    pub queries: Vec<Vec<f32>>,
    pub ground_truth: Vec<Vec<u32>>,
    pub dim: usize,
}
```

One full SIFT-family dataset. `base[i]` is the `i`-th base vector; `i` is also the
row-index id used by [`build_index_from_base`](#build_index_from_base) and stored
in the `.ivecs` ground-truth entries. Feed `base` to `build_index_from_base`,
then pass `queries` and `ground_truth` to [`recall_at_k`](#recall_at_k).

---

### `VERSION`

```rust,ignore
pub const VERSION: &str;
```

The crate version, taken from `Cargo.toml` at compile time.

```rust
let v = iqdb_eval::VERSION;
assert_eq!(v.split('.').count(), 3);
```

---

## Errors

All fallible functions return [`Result<T>`](#) — an alias for
`core::result::Result<T, EvalError>`. `EvalError` is `#[non_exhaustive]`, built on
`error_forge::ForgeError` (`kind()` / `caption()`), and implements
`std::error::Error` (the `Io` and `Search` variants expose a `source`). A `match`
on it must carry a wildcard arm.

| Variant | Raised when |
|---------|-------------|
| `Io { path, source }` | an OS-level read/open of a dataset file failed |
| `Parse { path, reason }` | a dataset file opened but could not be parsed (truncated record, bad header) |
| `DimensionMismatch { expected, found }` | a vector's length did not match the required dim |
| `LengthMismatch { kind, expected, found }` | two collections that must share a length did not |
| `KExceedsCorpus { k, corpus_size }` | `k` exceeds the number of vectors in the index |
| `EmptyInput { kind }` | a required input collection (or `k`) was empty |
| `Search(IqdbError)` | a downstream `IndexCore` operation returned an error |
| `UnsupportedVectorId { found }` | ground truth needed a `VectorId::U64` but got another shape |

`IqdbError` converts into `EvalError::Search` via `From`, so `?` works directly
on downstream index calls.

```rust
use iqdb_eval::EvalError;
let err = EvalError::DimensionMismatch { expected: 128, found: 64 };
assert_eq!(err.to_string(), "vector dimension mismatch: expected 128, found 64");
```

---

## Conventions and invariants

- **Row-index ↔ `VectorId::U64`.** [`build_index_from_base`](#build_index_from_base)
  inserts base row `i` at `VectorId::U64(i)`. Build the oracle and the index under
  test the same way, or `.ivecs` ids will not match the ids `search` returns.
- **recall@k is exact.** Ground truth is the *true* top-k from an exact oracle (or
  a loaded `.ivecs` set); it is never approximated.
- **Latency excludes build cost.** [`latency`](#latency) borrows an already-built
  index, so construction is never timed.
- **Percentiles are nearest-rank.** Every reported percentile is an observed
  sample — no interpolation.
- **The metric comes from the oracle.** [`compute_ground_truth`](#compute_ground_truth)
  reads `oracle.metric()`, so a mismatched metric cannot silently corrupt ground
  truth.

---

## Performance notes

- The harness is thin: a run's cost is dominated by the index `search` calls it
  drives. recall@k adds an `O(k)` `HashSet<u64>` membership check per query;
  latency adds one sort of the sample buffer.
- `latency` records into a pre-sized buffer and borrows the index — no allocation
  inside the timing window.
- Benchmarks for the recall and latency loops live in
  [`benches/eval_bench.rs`](../benches/eval_bench.rs); run with `cargo bench`.

---

## Feature flags

| Feature | Default | Description |
|---------|---------|-------------|
| `serde` | no | Derive `serde::Serialize` / `Deserialize` on [`RecallReport`](#recallreport) and [`LatencyReport`](#latencyreport). |

The crate is `std`-only; there is no `no_std` build.

---

<sub>Copyright &copy; 2026 <strong>James Gober</strong>.</sub>
