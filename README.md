# aidc-codec

Rust workspace for AIDC codec implementations (encode/decode) behind a shared interface.

## Crates

- `aidc-core`: shared transport/parser traits and common error types
- `aidc-gs1`: GS1 transport identification, normalization, and parsing
  - conformance matrix scaffold: `crates/aidc-gs1/README.md`

## Decode Input Expectations

`aidc-gs1` strict decode APIs expect AIM carrier input (`]xx...`).

Accepted inputs:
- `]d20109520123456788`
- `]Q3https://id.gs1.org/01/09520123456788`

Rejected by strict decode:
- `0109520123456788` (no AIM symbology identifier)
- `(01)09520123456788` (HRI/bracketed form)

Use:
- `decode_scan(&[u8])` for byte scans
- `decode_aim_str(&str)` for string scans

## Status

- GS1 reference-vector tests are vendored under `crates/aidc-gs1/tests/fixtures`
- Current focus is conformance-first implementation for GS1 AI/Digital Link parsing

## Dev

```bash
cargo test -p aidc-gs1
cargo check
```
