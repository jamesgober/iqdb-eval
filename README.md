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
    <a href="https://github.com/rust-lang/rfcs/blob/master/text/2495-min-rust-version.md"><img alt="MSRV" src="https://img.shields.io/badge/MSRV-1.85%2B-blue"></a>
</div>

<br>

<div align="left">
    <p>
        <strong>iqdb-eval</strong> is the evaluation harness: standard datasets, correct recall@k, latency percentiles, and throughput. Vector databases live and die by benchmarks, and this crate makes iQDB's numbers reproducible.
    </p>
    <p>
        It is generic over the `Index` trait and uses `iqdb-flat` to generate ground truth when none is provided.
    </p>
    <br>
    <hr>
    <p>
        <strong>MSRV is 1.85+</strong> (Rust 2024 edition). recall@k, latency percentiles, throughput. Reproducible.
    </p>
    <blockquote>
        <strong>Status: pre-1.0, in active development.</strong> The public API is being designed across the 0.x series and frozen at <code>1.0.0</code>. See <a href="./CHANGELOG.md"><code>CHANGELOG.md</code></a>.
    </blockquote>
</div>

<hr>
<br>

<h2>What it does</h2>

- **Standard datasets** &mdash; loaders for SIFT1M, GIST1M, GloVe, Deep1B subsets
- **recall@k** &mdash; compare approximate results against flat ground truth, correctly
- **Latency** &mdash; p50/p95/p99 and max
- **Throughput** &mdash; queries per second under load
- **Reproducible** &mdash; comparable results so performance claims are verifiable


<br>

## Installation

```toml
[dependencies]
iqdb-eval = "0.1"
```

<br>

## Status

This is the <code>v0.1.0</code> scaffold: structure, tooling, and quality gates are in place; the implementation lands across the 0.x series per the <a href="./dev/ROADMAP.md"><code>ROADMAP</code></a> and <a href="./docs/API.md"><code>docs/API.md</code></a>.

<hr>
<br>

## Where It Fits

`iqdb-eval` is a Phase-4 evaluation tool. It builds on:

- `iqdb-types` &mdash; core types
- `iqdb-index` &mdash; generic over any index
- `iqdb-flat` &mdash; ground-truth generation

It is unblocked once index + flat exist.

<br>

## Contributing

See <a href="./dev/DIRECTIVES.md"><code>dev/DIRECTIVES.md</code></a> for engineering standards and the definition of done. Before a PR: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features` must be clean.

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
