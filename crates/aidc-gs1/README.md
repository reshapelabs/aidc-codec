# aidc-gs1

GS1 codec crate for transport identification, normalization, decoding, and encoding.

## Conformance

This crate tracks GS1 conformance using a requirement matrix.

- Matrix owner document: `crates/aidc-gs1/README.md` (this file)
- Clause-tagged scan-data vectors: `crates/aidc-gs1/tests/fixtures/clause_scandata_process.jsonl`
- Fixture runner: `crates/aidc-gs1/tests/reference_vectors.rs`
- Dictionary lock metadata: `crates/aidc-gs1/data/gs1-syntax-dictionary.lock.json`

### Current Dictionary Source

- Upstream repository: `https://github.com/gs1/gs1-syntax-dictionary`
- Upstream ref: `main`
- Upstream commit: `2dceb36b5a8aa0b413554fa886c470f34afdd43a`
- Dictionary path: `gs1-syntax-dictionary.txt`
- Source SHA-256: `3e0e1680a0bb5a5af13c863a4d8d6075f5766a09c463a78e0168d7330eecf779`

### Status Legend

- `PASS`: implemented and covered by deterministic tests
- `PARTIAL`: partially implemented or partially tested
- `GAP`: not implemented or not sufficiently tested
- `N/A`: out of scope (with rationale)

### Requirement Matrix Scaffold

| Requirement ID | GS1 Clause | Requirement Summary | Scope | Status | Evidence (tests/fixtures) | Implementation (file) | Notes |
|---|---|---|---|---|---|---|---|
| GS1-SCAN-001 | 7.2.2 | AIM scan data includes symbology identifier | In | PASS | `conformance_clause_scandata_vectors`, `clause_scandata_process.jsonl` | `src/conformance.rs` | |
| GS1-SCAN-002 | 7.8.4 | Separator representation handling by carrier | In | PARTIAL | `carrier_separator_legality_matrix`, `clause_scandata_process.jsonl` | `src/normalize.rs`, `src/conformance.rs` | Parse-layer vs process-layer behavior differs; keep explicitly documented. |
| GS1-SCAN-003 | 7.8.6.2 | Invalid separator sequences are rejected in element parsing | In | PASS | `clause_mixed_predefined_and_variable_ordering_cases`, `fnc1_double_separator_is_rejected` | `src/parser/gs1.rs` | |
| GS1-DL-001 | 6.x/8.x | DL URI parse to internal AI form | In | PASS | `conformance_dl_parse_vectors` | `src/conformance.rs` | Keep list of covered DL sub-clauses here. |
| GS1-DL-002 | 6.x/8.x | DL URI encode from canonical payload | In | GAP | N/A | `src/encode.rs` | Currently unsupported encode path. |
| GS1-CMP-001 | Composite | Composite packet semantic parse/validation | In | GAP | N/A | `src/parser/gs1.rs` | Currently pass-through parse only. |
| GS1-TRP-001 | Symbology mapping | Supported symbology mapping behavior | In | PARTIAL | `conformance_scandata_process_vectors` | `src/identify.rs` | `]J0` intentionally unsupported in codec transport mapping. |

### Update Protocol

For every GS1 behavior change:

1. Update matrix row status and notes.
2. Add/update deterministic tests and/or clause-tagged fixtures.
3. Link evidence in `Evidence` column.
4. If out of scope, mark `N/A` with rationale.

For every dictionary-source update:

1. Update `data/gs1-syntax-dictionary.txt`.
2. Update `data/gs1-syntax-dictionary.lock.json` (repo/ref/commit/path/checksum/time).
3. Run conformance gates and update this README source block.

### Minimal Release Gate (Conformance-Focused)

Run before release:

```bash
mise exec -- cargo clippy --workspace --all-targets --all-features -- -D warnings
mise exec -- cargo nextest run --workspace --all-features
mise run gs1-diff
```
