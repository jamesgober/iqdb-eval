<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br>
    <b>iqdb-eval</b>
    <br>
    <sub><sup>iQDB BENCHMARKING & EVALUATION</sup></sub>
</h1>

<div align="center">
    <a href="https://crates.io/crates/iqdb-eval"><img alt="Crates.io" src="https://img.shields.io/crates/v/iqdb-eval"></a>
    <a href="https://crates.io/crates/iqdb-eval"><img alt="Downloads" src="https://img.shields.io/crates/d/iqdb-eval?color=%230099ff"></a>
    <a href="https://docs.rs/iqdb-eval"><img alt="docs.rs" src="https://img.shields.io/docsrs/iqdb-eval"></a>
    <a href="https://github.com/jamesgober/iqdb-eval/actions"><img alt="CI" src="https://github.com/jamesgober/iqdb-eval/actions/workflows/ci.yml/badge.svg"></a>
    <a href="https://github.com/rust-lang/rfcs/blob/master/text/2495-min-rust-version.md"><img alt="MSRV" src="https://img.shields.io/badge/MSRV-1.87%2B-blue"></a>
</div>

<br>

<div align="left">
    <p>
        <strong>iqdb-eval</strong> is the evaluation harness of the iQDB vector database. It measures the numbers a vector index lives and dies by &mdash; <strong>recall@k</strong>, <strong>latency percentiles</strong>, and <strong>throughput</strong> &mdash; and makes them reproducible.
    </p>
    <p>
        It is generic over the <code>iqdb-index</code> <code>Index</code> / <code>IndexCore</code> traits, so one harness call works against <code>iqdb-flat</code>, <code>iqdb-hnsw</code>, <code>iqdb-ivf</code>, or any future index, and it uses <code>iqdb-flat</code> as the exact oracle to compute true top-k ground truth when none is supplied.
    </p>
    <br>
    <hr>
    <p>
        <strong>MSRV is 1.87+</strong> (Rust 2024 edition). Correct recall@k, nearest-rank latency percentiles, single-thread QPS, and TEXMEX SIFT-family dataset loaders.
    </p>
    <blockquote>
        <strong>Status: stable (1.0).</strong> The measurement surface &mdash; <code>recall@k</code> against an exact oracle, latency percentiles, and dataset loaders &mdash; is committed under the SemVer 1.x guarantee: no breaking changes until 2.0. See <a href="./CHANGELOG.md"><code>CHANGELOG.md</code></a>.
    </blockquote>
</div>

<hr>
<br>

<h2>What it does</h2>

- **recall@k, done correctly** &mdash; compares an index's results against the *true* top-k from an exact `iqdb-flat` oracle (or against a known `.ivecs` ground-truth set); never approximated
- **Latency percentiles** &mdash; mean / min / max and nearest-rank p50 / p95 / p99 in microseconds, with build cost excluded by construction
- **Throughput** &mdash; single-thread queries-per-second over the measured query set
- **Index-agnostic** &mdash; one generic surface measures any backend behind the `Index` / `IndexCore` traits
- **Standard datasets** &mdash; zero-dependency loaders for the TEXMEX SIFT family (`SIFT1M`, `GIST1M`, `siftsmall`) in `.fvecs` / `.ivecs` format
- **Reproducible** &mdash; deterministic aggregation and a documented `VectorId::U64` row-index convention, so numbers are comparable across runs

<br>

## Installation

```toml
[dependencies]
iqdb-eval = "1.0"
```

`iqdb-eval` takes its vocabulary &mdash; `VectorId`, `SearchParams`, `DistanceMetric`, `IqdbError` &mdash; from `iqdb-types`, the `Index` / `IndexCore` traits from `iqdb-index`, and the exact oracle from `iqdb-flat`. A typical consumer depends on all four:

```toml
[dependencies]
iqdb-eval  = "1.0"
iqdb-flat  = "1.0"   # the exact oracle (and a fine first index under test)
iqdb-index = "1.0"   # the Index / IndexCore traits
iqdb-types = "1.0"   # VectorId, SearchParams, DistanceMetric, ...
```

MSRV is Rust **1.87** (edition 2024). The crate is `std`-only; the optional `serde` feature derives `Serialize` / `Deserialize` on the report types.

<br>

## Quick Start

Build the index under test and an exact oracle from the same base set, then ask
the harness for recall@k and latency:

```rust
use iqdb_eval::{build_index_from_base, latency, recall_at_k_vs_oracle, LatencyConfig};
use iqdb_flat::{FlatConfig, FlatIndex};
use iqdb_types::{DistanceMetric, SearchParams};

fn main() -> Result<(), iqdb_eval::EvalError> {
    let base: Vec<Vec<f32>> = vec![vec![0.0, 0.0], vec![3.0, 4.0], vec![1.0, 1.0]];
    let queries: Vec<Vec<f32>> = vec![vec![0.5, 0.5]];
    let metric = DistanceMetric::Euclidean;

    // The index under test and an exact oracle, built identically.
    let target: FlatIndex = build_index_from_base(FlatConfig, 2, metric, &base)?;
    let oracle: FlatIndex = build_index_from_base(FlatConfig, 2, metric, &base)?;
    let params = SearchParams::new(2, metric);

    // recall@k against the oracle's true top-k.
    let recall = recall_at_k_vs_oracle(&target, &oracle, &queries, &params)?;
    assert_eq!(recall.mean_recall, 1.0); // flat is exact

    // Latency percentiles (build cost is excluded — `target` is borrowed).
    let lat = latency(&target, &queries, &params, &LatencyConfig::default())?;
    assert!(lat.p50_us <= lat.p95_us);
    Ok(())
}
```

The complete surface — every function, parameter, error, and more examples — is
in [`docs/API.md`](./docs/API.md).

<br>

## Measuring an approximate index

Swap the target for any backend behind the `Index` / `IndexCore` traits; the
oracle stays flat. recall@k now reports how much accuracy the approximate index
trades for its speed:

```rust,ignore
use iqdb_eval::{build_index_from_base, recall_at_k_vs_oracle};
use iqdb_flat::{FlatConfig, FlatIndex};
use iqdb_hnsw::{HnswConfig, HnswIndex};
use iqdb_types::{DistanceMetric, SearchParams};

let metric = DistanceMetric::Euclidean;
let target: HnswIndex = build_index_from_base(HnswConfig::default(), dim, metric, &base)?;
let oracle: FlatIndex = build_index_from_base(FlatConfig, dim, metric, &base)?;
let params = SearchParams::new(10, metric);

let report = recall_at_k_vs_oracle(&target, &oracle, &queries, &params)?;
println!("recall@10 = {:.4}", report.mean_recall);
```

> **The one rule:** build both indexes with `build_index_from_base` (or insert
> each base row at `VectorId::U64(row_index)` by hand). That convention is what
> lets `.ivecs` ground-truth ids line up with the ids `search` returns.

<br>

## Standard datasets

The loaders read the TEXMEX corpus layout — a little-endian `u32 dim` header
followed by `dim` payload values per record — shared by `SIFT1M`, `GIST1M`, and
`siftsmall`. Point `load_sift_dataset` at a directory and a prefix; it resolves
`{prefix}_base.fvecs`, `{prefix}_query.fvecs`, and `{prefix}_groundtruth.ivecs`,
validates dimensions and lengths, and returns a `SiftDataset`:

```rust,no_run
use iqdb_eval::load_sift_dataset;

# fn run() -> Result<(), iqdb_eval::EvalError> {
let data = load_sift_dataset(".bench-data/siftsmall", "siftsmall")?;
assert_eq!(data.queries.len(), data.ground_truth.len());
# Ok(())
# }
```

Datasets are read from local files; downloading and caching them is left to the
caller (so the crate pulls in no network dependency). `read_fvecs` and
`read_ivecs` are available directly for non-standard layouts.

<br>

## Tiered API

- **Tier 1 — the lazy path.** `build_index_from_base` + `recall_at_k_vs_oracle`
  + `latency` cover the whole common case in three calls.
- **Tier 2 — the configured path.** Precompute ground truth once with
  `compute_ground_truth` and reuse it across `recall_at_k`; tune the timing loop
  with `LatencyConfig { warmup }`; load standard corpora with `load_sift_dataset`
  / `read_fvecs` / `read_ivecs`.
- **Tier 3 — the trait seam.** Everything is generic over
  `iqdb_index::IndexCore` (and `Index` for construction), so any custom backend
  behind those traits is measurable with no extra wiring.

<br>

## Performance

- **The harness is thin.** A measurement run's cost is dominated by the index
  `search` calls it drives; `iqdb-eval` adds only an `O(k)` set-membership check
  per query for recall and a single sort for latency percentiles.
- **No allocation in the timing window.** `latency` records into a pre-sized
  sample buffer; the index is borrowed, so build cost is never timed.
- **Recall sets are hashed once.** Each query's true top-k is a `HashSet<u64>`
  membership test against the retrieved hits — linear in the result size.
- **Nearest-rank percentiles.** Every reported percentile is an observed sample
  (`clamp(ceil(q·n) − 1, 0, n − 1)`), never an interpolation.

Benchmarks live in [`benches/eval_bench.rs`](./benches/eval_bench.rs)
(`cargo bench`).

<br>

## Examples

Runnable end-to-end programs in [`examples/`](./examples):

| Example | Shows |
|---------|-------|
| [`recall_quickstart`](./examples/recall_quickstart.rs) | recall@k against the exact `iqdb-flat` oracle |
| [`latency_report`](./examples/latency_report.rs) | latency percentiles + single-thread QPS |
| [`precomputed_ground_truth`](./examples/precomputed_ground_truth.rs) | compute ground truth once, sweep recall across several `k` |
| [`multi_metric`](./examples/multi_metric.rs) | comparing latency across distance metrics on one corpus |
| [`serde_report`](./examples/serde_report.rs) | serializing reports to JSON (`--features serde`) |
| [`sift_eval`](./examples/sift_eval.rs) | loading a real SIFT dataset and evaluating it end to end |

```sh
cargo run --example recall_quickstart
```

<br>

## Status

`v1.0.0` is **stable**: recall@k against an exact oracle, latency percentiles
and throughput, and the TEXMEX SIFT-family loaders are committed under the SemVer
1.x guarantee — no breaking changes until 2.0. The surface is covered by unit,
property-based, differential (against the exact `iqdb-flat` oracle), and
real-corpus integration tests, plus a runnable
<a href="./examples"><code>examples/</code></a> suite, and is recorded in the
<a href="./dev/ROADMAP.md"><code>ROADMAP</code></a>. Only additive, non-breaking
changes are made within 1.x.

<hr>
<br>

## Where It Fits

`iqdb-eval` is a Phase-4 evaluation tool. It builds on:

- `iqdb-types` &mdash; core types
- `iqdb-index` &mdash; generic over any index via the `Index` / `IndexCore` traits
- `iqdb-flat` &mdash; exact ground-truth generation

<br>

## Standards

Built to the iQDB Rust standard. See <a href="./REPS.md"><code>REPS.md</code></a> (Rust Efficiency &amp; Performance Standards) and <a href="./dev/DIRECTIVES.md"><code>dev/DIRECTIVES.md</code></a> for the engineering law and the definition of done. Before a PR: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features` must be clean.

<br>

<div id="license">
    <h2>License</h2>
    <p>Licensed under either of</p>
    <ul>
        <li><b>Apache License, Version 2.0</b> &mdash; <a href="./LICENSE-APACHE">LICENSE-APACHE</a></li>
        <li><b>MIT License</b> &mdash; <a href="./LICENSE-MIT">LICENSE-MIT</a></li>
    </ul>
    <p>at your option.</p>
</div>

<div align="center">
  <h2></h2>
  <sup>COPYRIGHT <small>&copy;</small> 2026 <strong>JAMES GOBER.</strong></sup>
</div>
