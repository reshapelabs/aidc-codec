use std::fs;
use std::path::PathBuf;

use aidc_gs1::{parse_bracketed_ai, parse_dl_uri, process_scan_data, DlParseOptions};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct ParseAiCase {
    should_succeed: bool,
    input: String,
    expected: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ScanDataProcessCase {
    should_succeed: bool,
    scan_data: String,
    expected_sym: String,
    expected_data: String,
}

#[derive(Debug, Clone, Deserialize)]
struct DlParseCase {
    should_succeed: bool,
    input: String,
    expected: String,
    permit_convenience_alphas: bool,
    permit_zero_suppressed_gtin: bool,
    permit_unknown_ais: bool,
    validate_unknown_ai_not_dl_attr: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct ClauseScanDataProcessCase {
    clause_id: String,
    should_succeed: bool,
    scan_data: String,
    expected_sym: String,
    expected_data: String,
}

#[derive(Debug, Clone, Deserialize)]
struct DictionaryLock {
    upstream_repo: String,
    upstream_ref: String,
    upstream_commit: String,
    upstream_dictionary_path: String,
    source_sha256: String,
    fetched_at_utc: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ExtractionReport {
    upstream_repo: String,
    counts: ExtractionCounts,
    known_ids: ExtractionKnownIds,
}

#[derive(Debug, Clone, Deserialize)]
struct ExtractionCounts {
    ai_parse: usize,
    scandata_process: usize,
    dl_parse: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct ExtractionKnownIds {
    ai_parse_input: String,
    scandata_sample: String,
    dl_parse_input: String,
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn read_jsonl<T: for<'de> Deserialize<'de>>(name: &str) -> Vec<T> {
    let path = fixtures_dir().join(name);
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            serde_json::from_str::<T>(line)
                .unwrap_or_else(|e| panic!("bad jsonl line {line:?}: {e}"))
        })
        .collect()
}

fn load_parse_ai_cases() -> Vec<ParseAiCase> {
    read_jsonl("ai_parse.jsonl")
}

fn load_scandata_process_cases() -> Vec<ScanDataProcessCase> {
    read_jsonl("scandata_process.jsonl")
}

fn load_dl_parse_cases() -> Vec<DlParseCase> {
    read_jsonl("dl_parse.jsonl")
}

fn load_clause_scandata_process_cases() -> Vec<ClauseScanDataProcessCase> {
    read_jsonl("clause_scandata_process.jsonl")
}

fn load_dictionary_lock() -> DictionaryLock {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/gs1-syntax-dictionary.lock.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()))
}

fn load_extraction_report() -> ExtractionReport {
    let path = fixtures_dir().join("EXTRACTION_REPORT.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()))
}

#[test]
fn fixtures_sanity_parse_ai() {
    let cases = load_parse_ai_cases();
    assert!(
        cases.len() > 20,
        "too few parseAIdata cases: {}",
        cases.len()
    );

    let has_known = cases.iter().any(|c| {
        c.should_succeed && c.input == "(01)12345678901231" && c.expected == "^0112345678901231"
    });
    assert!(has_known, "missing known parseAIdata fixture");
}

#[test]
fn fixtures_sanity_scandata_process() {
    let cases = load_scandata_process_cases();
    assert!(
        cases.len() > 40,
        "too few scandata_process cases: {}",
        cases.len()
    );

    let has_d2 = cases.iter().any(|c| {
        c.should_succeed
            && c.scan_data.contains("]d2")
            && c.expected_sym == "DM"
            && c.expected_data.contains("^01")
    });
    assert!(has_d2, "missing known ]d2 process fixture");
}

#[test]
fn fixtures_sanity_dl_parse() {
    let cases = load_dl_parse_cases();
    assert!(cases.len() > 60, "too few dl_parse cases: {}", cases.len());

    let has_known = cases.iter().any(|c| {
        c.should_succeed
            && c.input == "https://a/01/12312312312333"
            && c.expected == "^0112312312312333"
    });
    assert!(has_known, "missing known parseDLuri fixture");
}

#[test]
fn fixtures_sanity_clause_scandata_process() {
    let cases = load_clause_scandata_process_cases();
    assert!(
        cases.len() >= 6,
        "too few clause scandata cases: {}",
        cases.len()
    );
    assert!(
        cases.iter().all(|c| !c.clause_id.trim().is_empty()),
        "all clause fixtures must include clause_id"
    );

    let unique_clause_count = cases
        .iter()
        .map(|c| c.clause_id.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len();
    assert!(
        unique_clause_count >= 3,
        "expected at least 3 unique clause ids, got {unique_clause_count}"
    );
}

#[test]
fn fixtures_sanity_dictionary_lock() {
    let lock = load_dictionary_lock();
    assert_eq!(
        lock.upstream_repo,
        "https://github.com/gs1/gs1-syntax-dictionary"
    );
    assert_eq!(lock.upstream_ref, "main");
    assert_eq!(lock.upstream_commit.len(), 40);
    assert!(lock.upstream_commit.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(lock.upstream_dictionary_path, "gs1-syntax-dictionary.txt");
    assert_eq!(lock.source_sha256.len(), 64);
    assert!(lock.source_sha256.chars().all(|c| c.is_ascii_hexdigit()));
    assert!(lock.fetched_at_utc.ends_with('Z'));
}

#[test]
fn fixtures_sanity_extraction_report_matches_fixture_sets() {
    let report = load_extraction_report();
    let ai = load_parse_ai_cases();
    let sc = load_scandata_process_cases();
    let dl = load_dl_parse_cases();

    assert_eq!(
        report.upstream_repo,
        "https://github.com/gs1/gs1-syntax-engine"
    );
    assert_eq!(report.counts.ai_parse, ai.len());
    assert_eq!(report.counts.scandata_process, sc.len());
    assert_eq!(report.counts.dl_parse, dl.len());

    assert!(
        ai.iter()
            .any(|c| c.input == report.known_ids.ai_parse_input),
        "known ai_parse input missing from fixtures"
    );
    let scandata_sample = report
        .known_ids
        .scandata_sample
        .replace("\\u001d", "\u{001d}");
    assert!(
        sc.iter().any(|c| c.scan_data == scandata_sample),
        "known scandata sample missing from fixtures"
    );
    assert!(
        dl.iter()
            .any(|c| c.input == report.known_ids.dl_parse_input),
        "known dl_parse input missing from fixtures"
    );
}

#[test]
fn conformance_parse_ai_vectors() {
    let cases = load_parse_ai_cases();
    assert!(!cases.is_empty(), "no parse AI fixtures found");

    for case in cases {
        let got = parse_bracketed_ai(&case.input);
        if case.should_succeed {
            let out = got.unwrap_or_else(|e| {
                panic!(
                    "expected success for input {:?}, got error: {e}",
                    case.input
                )
            });
            assert_eq!(
                out, case.expected,
                "parseAIdata mismatch for input {:?}",
                case.input
            );
        } else if got.is_ok() {
            panic!(
                "expected failure for input {:?}, got {:?}",
                case.input,
                got.ok()
            );
        }
    }
}

#[test]
fn conformance_scandata_process_vectors() {
    let cases = load_scandata_process_cases();
    assert!(!cases.is_empty(), "no scan-data fixtures found");

    for case in cases {
        let got = process_scan_data(&case.scan_data);
        if case.should_succeed {
            let out = got.unwrap_or_else(|e| {
                panic!(
                    "expected success for scan_data {:?}, got error: {e}",
                    case.scan_data
                )
            });
            assert_eq!(
                out.sym_name, case.expected_sym,
                "sym mismatch for scan_data {:?}",
                case.scan_data
            );
            assert_eq!(
                out.data_str, case.expected_data,
                "data_str mismatch for scan_data {:?}",
                case.scan_data
            );
        } else if got.is_ok() {
            panic!(
                "expected failure for scan_data {:?}, got {:?}",
                case.scan_data,
                got.ok()
            );
        }
    }
}

#[test]
fn conformance_dl_parse_vectors() {
    let cases = load_dl_parse_cases();
    assert!(!cases.is_empty(), "no DL parse fixtures found");

    for case in cases {
        let opts = DlParseOptions {
            permit_convenience_alphas: case.permit_convenience_alphas,
            permit_zero_suppressed_gtin: case.permit_zero_suppressed_gtin,
            permit_unknown_ais: case.permit_unknown_ais,
            validate_unknown_ai_not_dl_attr: case.validate_unknown_ai_not_dl_attr,
        };
        let got = parse_dl_uri(&case.input, opts);
        if case.should_succeed {
            let out = got.unwrap_or_else(|e| {
                panic!(
                    "expected DL success for input {:?}, got error: {e}",
                    case.input
                )
            });
            assert_eq!(
                out, case.expected,
                "parseDLuri mismatch for input {:?}",
                case.input
            );
        } else if got.is_ok() {
            panic!(
                "expected DL failure for input {:?}, got {:?}",
                case.input,
                got.ok()
            );
        }
    }
}

#[test]
fn conformance_clause_scandata_vectors() {
    let cases = load_clause_scandata_process_cases();
    assert!(!cases.is_empty(), "no clause scan-data fixtures found");

    for case in cases {
        let got = process_scan_data(&case.scan_data);
        if case.should_succeed {
            let out = got.unwrap_or_else(|e| {
                panic!(
                    "[{}] expected success for scan_data {:?}, got error: {e}",
                    case.clause_id, case.scan_data
                )
            });
            assert_eq!(
                out.sym_name, case.expected_sym,
                "[{}] sym mismatch for scan_data {:?}",
                case.clause_id, case.scan_data
            );
            assert_eq!(
                out.data_str, case.expected_data,
                "[{}] data_str mismatch for scan_data {:?}",
                case.clause_id, case.scan_data
            );
        } else if got.is_ok() {
            panic!(
                "[{}] expected failure for scan_data {:?}, got {:?}",
                case.clause_id,
                case.scan_data,
                got.ok()
            );
        }
    }
}
