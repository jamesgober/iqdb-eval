# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added

### Changed

### Fixed

### Security

---

## [1.0.0] - 2026-06-07

First stable release. The evaluation harness is feature-complete and its public
surface is committed under the SemVer 1.x guarantee — no breaking changes until
2.0. Every measurement is generic over the `iqdb-index` `Index` / `IndexCore`
traits and validated against the exact `iqdb-flat` oracle.

### Added

- **recall@k** — `recall_at_k` (against a supplied `.ivecs`-shaped ground truth),
  `compute_ground_truth` (true top-k from an exact oracle), and the
  `recall_at_k_vs_oracle` convenience wrapper. Recall is the true top-k overlap,
  never approximated.
- **Latency** — `latency` reports mean / min / max and nearest-rank p50 / p95 /
  p99 in microseconds plus single-thread QPS, with build cost excluded by
  construction and a configurable `LatencyConfig { warmup }`.
- **Index construction** — `build_index_from_base`, generic over
  `iqdb_index::Index`, inserting each base row at `VectorId::U64(row_index)` so
  ids align with `.ivecs` ground truth.
- **Dataset loaders** — `read_fvecs`, `read_ivecs`, and `load_sift_dataset` for
  the TEXMEX SIFT family (`SIFT1M`, `GIST1M`, `siftsmall`), returning a
  `SiftDataset`; zero new parsing dependencies.
- **Report types** — `RecallReport` and `LatencyReport`, with optional
  `serde::Serialize` / `Deserialize` behind the `serde` feature.
- **Errors** — `EvalError` (`#[non_exhaustive]`, `error_forge::ForgeError`) and
  the `Result<T>` alias; `From<IqdbError>` for `?` on downstream index calls.
- **Quality** — unit, `proptest` invariant, differential (flat-vs-flat), and
  opt-in real-corpus integration tests; `criterion` benches for the recall and
  latency hot paths; three runnable examples; complete `docs/API.md`.

### Changed

- Crate is now `std`-only: the placeholder `std` feature was removed (the
  measurement surface requires `std` unconditionally).
- Dependencies pinned to the published iQDB family at `1.0.0` (`iqdb-types`,
  `iqdb-index`, `iqdb-flat`).

---

## [0.1.0] - 2026-05-30

Initial scaffold and repository bootstrap. No domain logic yet &mdash; this release establishes the structure, tooling, and quality gates the implementation will be built on.

### Added

- `Cargo.toml` with crate metadata, Rust 2024 edition, MSRV 1.87.
- Dual `Apache-2.0 OR MIT` license files.
- `README.md`, `CHANGELOG.md`, and a documentation skeleton.
- `REPS.md` compliance baseline.
- `.github/workflows/ci.yml` CI matrix; `deny.toml`, `clippy.toml`, `rustfmt.toml`.
- `dev/DIRECTIVES.md` and `dev/ROADMAP.md` (committed engineering standards + plan).
[Unreleased]: https://github.com/jamesgober/iqdb-eval/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/jamesgober/iqdb-eval/compare/v0.1.0...v1.0.0
[0.1.0]: https://github.com/jamesgober/iqdb-eval/releases/tag/v0.1.0
