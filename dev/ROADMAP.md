# iqdb-eval -- Roadmap

> Path from scaffold to a stable 1.0. Hard parts are front-loaded; each phase has hard exit criteria.
>
> **Anti-deferral rule:** no listed hard task moves to a later phase unless this file records the move and the reason.

---

## v0.1.0 -- Scaffold (DONE)

Compiles, CI green, structure correct, no domain logic.

- [x] Manifest, README, CHANGELOG, REPS, license, CI, lints in place.
- [x] API surface sketched in `docs/API.md`.

---

## v0.2.0 -- recall@k against flat ground truth + report types (THE HARD PART, NOT DEFERRED) (DONE)

Exit criteria:
- [x] Every public item has rustdoc + a runnable example.
- [x] Core invariants property-tested.

---

## v0.3.0 -- latency (p50/p95/p99) + throughput harness (DONE)

Exit criteria:
- [x] New surface tested and benchmarked where it is a hot path.

---

## v0.4.0 -- standard dataset loaders (SIFT1M/GIST1M) with caching + feature freeze (DONE)

Exit criteria:
- [x] No `todo!`/`unimplemented!`. Feature freeze declared.

> **Scope note (anti-deferral rule).** GloVe was dropped from the loader set: it
> ships in a different (text) format than the TEXMEX `.fvecs`/`.ivecs` corpus and
> would add surface for no extra coverage of the harness itself. The loaders cover
> the TEXMEX SIFT family (`SIFT1M`, `GIST1M`, `siftsmall`). Datasets are read from
> local files; downloading/caching is left to the caller so the crate pulls in no
> network dependency. `read_fvecs`/`read_ivecs` are exposed for non-standard
> layouts.

---

## v0.5.0 -- reproducibility + API freeze (DONE)

Exit criteria:
- [x] Public API frozen (recorded below). `cargo audit` + `cargo deny` clean.

---

## v0.6.0 -> v0.9.x -- Alpha / Beta -> RC (consolidated into 1.0.0)

The full surface landed in one verified step on top of the already-stable (1.0.0)
iQDB spine, and the harness had already been exercised as the cross-crate
validation home for `iqdb-flat`/`iqdb-hnsw`/`iqdb-ivf`. With every Definition-of-
Done criterion met and all dependencies frozen at 1.0.0, there was nothing left
for a separate alpha/beta/rc cycle to de-risk, so 0.6.x-0.9.x are consolidated
directly into 1.0.0 (matching the rest of the family).

---

## v1.0.0 -- Stable (DONE)

- [x] Definition of Done (DIRECTIVES section 7) satisfied.
- [x] Public API frozen until 2.0.
- [x] Release note written (`docs/release/v1.0.0.md`). Publish + tag handled by maintainer.

### Frozen public API (1.x)

- Functions: `build_index_from_base`, `compute_ground_truth`, `recall_at_k`,
  `recall_at_k_vs_oracle`, `latency`.
- Types: `LatencyConfig`, `RecallReport`, `LatencyReport`, `SiftDataset`,
  `EvalError`, `Result`.
- Loaders: `read_fvecs`, `read_ivecs`, `load_sift_dataset`.
- Const: `VERSION`.
- Feature: `serde` (additive, derives on the report types).

---

## Out of scope for 1.0

- Being a production index -- evaluation only.
- Third-party comparative harness -- revisit post-1.0.
