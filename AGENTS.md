# Repository Guidelines

## Project Structure & Module Organization
This is a Rust workspace focused on AIDC payload decoding and encoding.

- `crates/aidc-core`: shared interfaces and error types (`TransportCodec`, scan I/O models, canonical encode models).
- `crates/aidc-gs1`: GS1 implementation (transport identify/normalize, parse + encode, conformance helpers, AI dictionary generation).
- `crates/aidc-gs1/tests`: fixture-driven integration tests.
- `crates/aidc-gs1/proptest-regressions`: persisted failing seeds from property tests.
- `crates/aidc-gs1/data/gs1-syntax-dictionary.txt`: source for generated AI metadata.

Prefer adding new specs as new crates (new codec implementation) that conform to `aidc-core` for both decode and encode paths.

## Build, Test, and Development Commands
- `cargo check`: fast compile validation for all crates.
- `cargo test`: run full workspace tests.
- `cargo test -p aidc-gs1 --features gs1-dl`: test GS1 Digital Link feature path.
- `cargo test -p aidc-gs1 --lib`: quick inner-loop for GS1 unit/property tests.

Use `mise` for toolchain management (`mise.toml` pins Rust toolchain).

## Coding Style & Naming Conventions
- Rust 2021 edition; keep code idiomatic and explicit.
- Use `snake_case` for functions/modules, `CamelCase` for types/traits.
- Keep parser logic small and composable; avoid large monolithic functions.
- Avoid comments for obvious code; add comments only for non-trivial logic.

## Testing Guidelines
- Unit tests live near implementation; integration tests in `crates/*/tests`.
- `proptest` is crucial here: use it for parser invariants, boundary handling, and malformed input behavior.
- Keep `proptest-regressions` files committed when they capture real failures.
- Add deterministic fixture tests for all critical behavior changes.

When adding a new codec/spec crate, prioritize borrowing/adapting authoritative external conformance tests and vectors (reference repos/spec suites) before expanding custom tests.

## Commit & Pull Request Guidelines
- Follow existing history style: short, imperative commit subjects (e.g., `Extract shared GS1 AI rule module`).
- Keep commits focused (one logical change).
- PRs should include:
  - what changed and why,
  - affected crates/features,
  - test commands run and outcomes,
  - links to external specs/reference vectors used.
