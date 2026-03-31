# aidc-codec

Rust workspace for AIDC codec implementations (encode/decode) behind a shared interface.

## Crates

- `aidc-core`: shared transport/parser traits and common error types
- `aidc-gs1`: GS1 transport identification, normalization, and parsing
  - conformance matrix scaffold: `crates/aidc-gs1/README.md`

## Status

- GS1 reference-vector tests are vendored under `crates/aidc-gs1/tests/fixtures`
- Current focus is conformance-first implementation for GS1 AI/Digital Link parsing

## Dev

```bash
cargo test -p aidc-gs1
cargo check
```
