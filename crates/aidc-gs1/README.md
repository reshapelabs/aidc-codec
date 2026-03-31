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

### Requirement Matrix (First 66 Entries)

| Requirement ID | GS1 Clause | Requirement Summary | Scope | Status | Evidence (tests/fixtures) | Implementation (file) | Notes |
|---|---|---|---|---|---|---|---|
| GS1-7.2.2-01 | 7.2.2 | Full string starts with AIM symbology identifier | In | PASS | `conformance_scandata_process_vectors`, `conformance_clause_scandata_vectors` | `src/conformance.rs` | |
| GS1-7.2.2-02 | 7.2.2 | Unknown/invalid symbology identifiers rejected | In | PASS | `conformance_scandata_process_vectors` | `src/conformance.rs` | |
| GS1-7.2.6-01 | 7.2.6 | ITF-14 requires exactly 14 digits | In | PASS | `i1_rejects_wrong_length`, `i1_decode_rejects_non_itf14_payload` | `src/normalize.rs` | |
| GS1-7.2.7-01 | 7.2.7 | ITF-14 check digit validated for `]I1` path | In | PASS | `i1_rejects_bad_check_digit` | `src/normalize.rs` | |
| GS1-7.2.7-02 | 7.2.7 | EAN/UPC check digit validated in scan processing | In | PASS | `conformance_scandata_process_vectors` | `src/conformance.rs` | |
| GS1-7.2.8-01 | 7.2.8 | Element strings moved to internal message field form | In | PASS | `conformance_scandata_process_vectors` | `src/conformance.rs` | |
| GS1-7.3-01 | 7.3 | Required AI associations enforced | In | PASS | `validates_required_ai_associations`, `encode_rejects_missing_required_association`, `decode_rejects_missing_required_association` | `src/ai.rs` | |
| GS1-7.3-02 | 7.3 | Exclusive AI associations enforced | In | PASS | `validates_exclusive_ai_associations` | `src/ai.rs` | |
| GS1-7.4-01 | 7.4 | AI-level value validity (charset/length/date/time/check) | In | PASS | `conformance_parse_ai_vectors`, `validates_ai_*`, `validates_mod10_for_gtin_and_gln`, `rejects_empty_ai_values`, `rejects_values_longer_than_90_chars`, `validates_ai17_zero_day_and_date_bounds` | `src/ai.rs` | Roll-up row backed by detailed 7.4 sub-rows (`7.4-02..10`). |
| GS1-7.8.1-01 | 7.8.1 | Multiple GS1 element strings parsed in one carrier payload | In | PASS | `parses_variable_ai_with_fnc1_separator`, `conformance_scandata_process_vectors` | `src/parser/gs1.rs`, `src/conformance.rs` | |
| GS1-7.8.2-01 | 7.8.2 | AI tokens recognized as numeric 2–4 length dictionary keys | In | PASS | `conformance_parse_ai_vectors`, `rejects_unknown_ai` | `src/parser/gs1.rs` | |
| GS1-7.8.3-01 | 7.8.3 | Predefined-length elements can chain without separator | In | PASS | `predefined_fixed_ai_can_chain_without_separator` | `src/parser/gs1.rs` | |
| GS1-7.8.3-02 | 7.8.3 | Non-predefined element followed by next requires separator | In | PASS | `non_predefined_fixed_ai_requires_separator_when_followed_by_more_data` | `src/parser/gs1.rs` | |
| GS1-7.8.3-03 | 7.8.3 | Last element does not end with separator (strict parser rejects trailing separator) | In | PASS | `rejects_single_trailing_fnc1_separator` | `src/parser/gs1.rs` | |
| GS1-7.8.4-01 | 7.8.4 | GS1-128 separator handling | In | PASS | `carrier_separator_legality_matrix` (`]C1` cases) | `src/conformance.rs`, `src/parser/gs1.rs` | |
| GS1-7.8.4-02 | 7.8.4 | GS1 DataMatrix and DotCode separator handling | In | PASS | `carrier_separator_legality_matrix` (`]d2`, `]J1` cases) | `src/conformance.rs`, `src/parser/gs1.rs` | |
| GS1-7.8.4-03 | 7.8.4 | GS1 QR separator handling (`<GS>` and `%`) | In | PASS | `q3_percent_and_gs_separator_are_semantically_equivalent`, `q3_maps_percent_to_fnc1_when_no_gs_present` | `src/normalize.rs`, `src/conformance.rs` | |
| GS1-7.8.4-04 | 7.8.4 | Decoded separator represented as `<GS>` (`0x1D`) in data string | In | PASS | `conformance_scandata_process_vectors`, `conformance_clause_scandata_vectors` | `src/conformance.rs` | |
| GS1-7.8.5-01 | 7.8.5 | Basic GS1 barcode structure processed by symbology path | In | PASS | `conformance_scandata_process_vectors` | `src/identify.rs`, `src/conformance.rs` | |
| GS1-7.8.6.1-01 | 7.8.6.1 | Concatenation with predefined-length elements | In | PASS | `predefined_fixed_ai_can_chain_without_separator`, `clause_mixed_predefined_and_variable_ordering_cases` | `src/parser/gs1.rs` | |
| GS1-7.8.6.2-01 | 7.8.6.2 | Non-predefined followed by another element uses separator | In | PASS | `clause_mixed_predefined_and_variable_ordering_cases` | `src/parser/gs1.rs` | |
| GS1-7.8.6.2-02 | 7.8.6.2 | Non-predefined as last element omits separator | In | PASS | `parses_variable_ai_with_fnc1_separator` | `src/parser/gs1.rs` | |
| GS1-7.8.6.3-01 | 7.8.6.3 | Trailing separator is rejected in strict parser mode | In | PASS | `rejects_single_trailing_fnc1_separator` | `src/parser/gs1.rs` | |
| GS1-7.8.6.3-02 | 7.8.6.3 | Double separator / empty field rejected in parser | In | PASS | `fnc1_double_separator_is_rejected`, `clause_mixed_predefined_and_variable_ordering_cases` | `src/parser/gs1.rs` | |
| GS1-7.8.6.3-03 | 7.8.6.3 | Variable→fixed without separator is rejected as missing required separator | In | PASS | `rejects_variable_then_fixed_without_separator`, `clause_mixed_predefined_and_variable_ordering_cases` | `src/parser/gs1.rs` | |
| GS1-7.9.1-01 | 7.9.1 | Standard mod10 check digit calculations for GS1 IDs | In | PASS | `validates_mod10_for_gtin_and_gln`, `check_digit_vectors` | `src/ai.rs`, `src/check.rs` | |
| GS1-7.9.3-01 | 7.9.3 | Four-digit price-field check digit calculation | In | PASS | `price_or_weight_check_digit_vectors` | `src/check.rs` | |
| GS1-7.9.4-01 | 7.9.4 | Five-digit price-field check digit calculation | In | PASS | `price_or_weight_check_digit_vectors` | `src/check.rs` | |
| GS1-7.9.5-01 | 7.9.5 | Alphanumeric check-character pair calculation | In | PASS | `check_character_pair_vectors` | `src/check.rs` | |
| GS1-DL-ENC-001 | GS1 Digital Link Std | GS1 Digital Link encode from canonical payload | In | PASS | `encode::tests::dl_encode_builds_canonical_uri_with_qualifiers_and_sorted_query`, `gs1_roundtrip::dl_encode_then_decode_preserves_gs1_semantics` | `src/encode.rs` | Encodes canonical URI at `https://id.gs1.org` with path primary/qualifiers and sorted query attributes. |
| GS1-7.3-03 | 7.3 | Required AI pattern associations enforced | In | PASS | `validates_required_pattern_associations` | `src/ai.rs` | |
| GS1-7.4-02 | 7.4 | AI `01` fixed numeric length and charset enforcement | In | PASS | `validates_ai_01_numeric_fixed_length` | `src/ai.rs` | |
| GS1-7.4-03 | 7.4 | AI `10` variable-length AI82 charset enforcement | In | PASS | `validates_ai_10_variable_x_charset` | `src/ai.rs` | |
| GS1-7.4-04 | 7.4 | AI82 character-class boundary validation | In | PASS | `validates_ai82_boundary_vectors` | `src/ai.rs` | |
| GS1-7.4-05 | 7.4 | AI39 character-class boundary validation | In | PASS | `validates_ai39_boundary_vectors` | `src/ai.rs` | |
| GS1-7.4-06 | 7.4 | YYMMDD date component validation (month/day bounds) | In | PASS | `validates_ai_17_fixed_numeric_date_shape` | `src/ai.rs` | |
| GS1-7.4-07 | 7.4 | Multi-component AI segment constraints validated (AI `253`) | In | PASS | `validates_ai_253_multipart_constraints` | `src/ai.rs` | |
| GS1-7.4-08 | 7.4 | Base64url value constraints validated (AI `8030`) | In | PASS | `validates_ai_8030_base64url_charset` | `src/ai.rs` | |
| GS1-7.4-09 | 7.4 | Time component validation (HHMI) where specified | In | PASS | `validates_hhmi_time_component` | `src/ai.rs` | |
| GS1-7.4-10 | 7.4 | Empty AI values rejected during parse/validation | In | PASS | `conformance_parse_ai_vectors`, `parse_ai_elements` | `src/conformance.rs`, `src/parser/gs1.rs` | |
| GS1-7.8.2-02 | 7.8.2 | Unknown AI codes rejected in element parser | In | PASS | `rejects_unknown_ai` | `src/parser/gs1.rs` | |
| GS1-7.8.2-03 | 7.8.2 | Truncated fixed-length AI values rejected | In | PASS | `rejects_truncated_fixed_value` | `src/parser/gs1.rs` | |
| GS1-7.8.5-02 | 7.8.5 | Parsed GS1 element strings render deterministic HRI output | In | PASS | `parse_result_hri_formats_ai_elements`, `hri_is_deterministic_for_parsed_element_strings` | `src/parser/gs1.rs` | |
| GS1-7.8.5-03 | 7.8.5 | Non-GS1 payloads do not produce HRI output | In | PASS | `non_gs1_payload_has_no_hri` | `src/parser/gs1.rs` | |
| GS1-7.8.4-05 | 7.8.4 | QR `%` and `<GS>` separators treated equivalently for normalization path | In | PASS | `q3_percent_and_gs_separator_are_semantically_equivalent` | `src/normalize.rs`, `src/conformance.rs` | |
| GS1-7.8.4-06 | 7.8.4 | `%` retained when explicit `<GS>` already exists in QR payload | In | PASS | `q3_keeps_percent_when_gs_already_present` | `src/normalize.rs` | |
| GS1-DL-001 | GS1 Digital Link Std | DL URI corpus passes parse conformance fixtures | In | PASS | `conformance_dl_parse_vectors` | `src/conformance.rs` | |
| GS1-DL-002 | GS1 Digital Link Std | DL URI accepts only supported schemes (`http`/`https`) | In | PASS | `conformance_dl_parse_vectors` | `src/conformance.rs` | |
| GS1-DL-003 | GS1 Digital Link Std | Illegal DL URI characters are rejected | In | PASS | `conformance_dl_parse_vectors` | `src/conformance.rs` | |
| GS1-DL-004 | GS1 Digital Link Std | DL URI authority/path structural validation enforced | In | PASS | `conformance_dl_parse_vectors` | `src/conformance.rs` | |
| GS1-DL-005 | GS1 Digital Link Std | Primary key AI required in DL path | In | PASS | `conformance_dl_parse_vectors` | `src/conformance.rs` | |
| GS1-DL-006 | GS1 Digital Link Std | Path qualifier validity enforced per primary AI | In | PASS | `conformance_dl_parse_vectors` | `src/conformance.rs` | |
| GS1-DL-007 | GS1 Digital Link Std | Percent-decoding supports values and rejects NUL escapes | In | PASS | `conformance_dl_parse_vectors` | `src/conformance.rs` | |
| GS1-DL-008 | GS1 Digital Link Std | Duplicate AI across path/query rejected | In | PASS | `dl_query_duplicate_ai_is_rejected`, `dl_query_path_duplicate_ai_is_rejected` | `src/conformance.rs` | |
| GS1-DL-009 | GS1 Digital Link Std | Query AI ordering preserved in internal representation | In | PASS | `dl_query_order_is_preserved_in_internal_representation` | `src/conformance.rs` | |
| GS1-DL-010 | GS1 Digital Link Std | Variable path fields followed by query insert separators correctly | In | PASS | `dl_path_then_query_variable_fields_insert_separator` | `src/conformance.rs` | |
| GS1-DL-011 | GS1 Digital Link Std | Unknown numeric query AI policy is option-controlled | In | PASS | `conformance_dl_parse_vectors`, `dl_unknown_numeric_query_ai_rejected_when_not_permitted`, `dl_unknown_numeric_query_ai_allowed_when_permitted` | `src/conformance.rs` | |
| GS1-DL-012 | GS1 Digital Link Std | Convenience alpha key mapping is option-controlled | In | PASS | `conformance_dl_parse_vectors`, `dl_convenience_alpha_key_rejected_when_option_disabled`, `dl_convenience_alpha_key_mapping_enabled_by_option` | `src/conformance.rs` | |
| GS1-DL-013 | GS1 Digital Link Std | Zero-suppressed GTIN handling is option-controlled | In | PASS | `conformance_dl_parse_vectors` | `src/conformance.rs` | |
| GS1-DL-014 | GS1 Digital Link Std | Parsed DL payload converts into typed AI element list | In | PASS | `parses_digital_link_into_ai_elements` | `src/parser/gs1.rs` | |
| GS1-7.8-01 | 7.8 | Bracketed AI payloads parse into internal `^` representation | In | PASS | `conformance_parse_ai_vectors` | `src/conformance.rs` | |
| GS1-7.8-02 | 7.8 | Escaped literal `(` is preserved in bracketed AI values | In | PASS | `conformance_parse_ai_vectors` | `src/conformance.rs` | |
| GS1-CMP-001 | Composite Transfer | Composite packet requires `|]e0` separator between linear and CC components | In | PASS | `parses_composite_packet_with_valid_separator`, `rejects_composite_packet_without_separator` | `src/parser/gs1.rs` | |
| GS1-CMP-002 | Composite Transfer | Composite packet rejects empty linear or CC component | In | PASS | `rejects_composite_packet_with_empty_cc_component` | `src/parser/gs1.rs` | |
| GS1-CMP-003 | Composite Transfer | Composite packet parsing yields structured AI element semantics | In | PASS | `parses_composite_packet_with_valid_separator`, `ean8_composite_injects_primary_ai01`, `non_ean_composite_does_not_inject_primary_ai01`, `ean13_composite_decode_parses_cc_ai_semantics` | `src/parser/gs1.rs`, `src/parser/mod.rs` | EAN/UPC composite decode injects validated AI `01` primary from linear component; CC AIs remain structured and HRI-stable. |
| GS1-CMP-ENC-001 | Composite Transfer | Composite packet encode from canonical payload | In | PARTIAL | `composite_encode_builds_ean13_packet`, `composite_encode_builds_ean8_packet`, `ean13_composite_encode_then_decode_preserves_gs1_semantics`, `ean8_composite_encode_then_decode_preserves_gs1_semantics` | `src/encode.rs` | Implemented for `]E0`/`]E4` with AI `01` primary + CC elements; other composite symbologies remain unsupported. |
| GS1-DL-015 | GS1 Digital Link Std | DL encode semantic parity with GS1 C reference parser | In | PASS | `differential_dl_encode_semantics_matches_reference` | `tests/differential_ref.rs` | |
| GS1-DL-016 | GS1 Digital Link Std | DL encode rejects repeated AI keys | In | PASS | `dl_encode_rejects_repeated_ai_keys` | `src/encode.rs` | |

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
