# Fixture Provenance

These fixtures are vendored from the GS1 reference implementation test corpus.

- Upstream repository: `https://github.com/gs1/gs1-syntax-engine`
- Upstream commit: `84a35e1c3ff322b062e56c000140f0f22303d037`
- License: Apache-2.0

Source files used:

- `src/c-lib/ai.c`
  - Macro cases from `test_parseAIdata(...)`
- `src/c-lib/scandata.c`
  - Macro cases from `test_testProcessScanData(...)`
- `src/c-lib/dl.c`
  - Macro cases from `test_parseDLuri(...)`

Generated files:

- `ai_parse.jsonl`
- `scandata_process.jsonl`
- `dl_parse.jsonl`

Regeneration:

- `python3 crates/aidc-gs1/tests/fixtures/extract_reference_fixtures.py`

The extractor expects a local checkout at `./gs1-reference` (repo root).
