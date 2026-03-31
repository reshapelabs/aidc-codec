use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use aidc_core::{ScanInput, TransportCodec};
use aidc_gs1::process_scan_data;
use aidc_gs1::Gs1Codec;
use libloading::{Library, Symbol};
use proptest::prelude::*;
use proptest::strategy::Strategy;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct ScanDataProcessCase {
    scan_data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RefOutcome {
    ok: bool,
    sym_name: String,
    data_str: String,
    ai_data_bracketed: Option<String>,
    hri_compact: Option<String>,
}

struct RefApi {
    _lib: Library,
    init: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    free: unsafe extern "C" fn(*mut c_void),
    set_scan_data: unsafe extern "C" fn(*mut c_void, *const c_char) -> bool,
    get_data_str: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    get_sym: unsafe extern "C" fn(*mut c_void) -> c_int,
    get_ai_data_str: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    get_hri: unsafe extern "C" fn(*mut c_void, *mut *const *const c_char) -> c_int,
}

impl RefApi {
    fn load(path: &str) -> Result<Self, String> {
        let lib = unsafe { Library::new(path) }
            .map_err(|e| format!("failed to load reference lib {path}: {e}"))?;
        let init = unsafe {
            load_sym::<unsafe extern "C" fn(*mut c_void) -> *mut c_void>(
                &lib,
                b"gs1_encoder_init\0",
            )?
        };
        let free =
            unsafe { load_sym::<unsafe extern "C" fn(*mut c_void)>(&lib, b"gs1_encoder_free\0")? };
        let set_scan_data = unsafe {
            load_sym::<unsafe extern "C" fn(*mut c_void, *const c_char) -> bool>(
                &lib,
                b"gs1_encoder_setScanData\0",
            )?
        };
        let get_data_str = unsafe {
            load_sym::<unsafe extern "C" fn(*mut c_void) -> *const c_char>(
                &lib,
                b"gs1_encoder_getDataStr\0",
            )?
        };
        let get_sym = unsafe {
            load_sym::<unsafe extern "C" fn(*mut c_void) -> c_int>(&lib, b"gs1_encoder_getSym\0")?
        };
        let get_ai_data_str = unsafe {
            load_sym::<unsafe extern "C" fn(*mut c_void) -> *const c_char>(
                &lib,
                b"gs1_encoder_getAIdataStr\0",
            )?
        };
        let get_hri = unsafe {
            load_sym::<unsafe extern "C" fn(*mut c_void, *mut *const *const c_char) -> c_int>(
                &lib,
                b"gs1_encoder_getHRI\0",
            )?
        };
        Ok(Self {
            _lib: lib,
            init,
            free,
            set_scan_data,
            get_data_str,
            get_sym,
            get_ai_data_str,
            get_hri,
        })
    }
}

unsafe fn load_sym<T: Copy>(lib: &Library, name: &'static [u8]) -> Result<T, String> {
    let sym: Symbol<'_, T> = lib.get(name).map_err(|e| {
        format!(
            "failed to load symbol {}: {e}",
            String::from_utf8_lossy(name)
        )
    })?;
    Ok(*sym)
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn read_scandata_cases() -> Vec<ScanDataProcessCase> {
    let path = fixtures_dir().join("scandata_process.jsonl");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            serde_json::from_str::<ScanDataProcessCase>(line)
                .unwrap_or_else(|e| panic!("bad jsonl line {line:?}: {e}"))
        })
        .collect()
}

fn sym_name(code: c_int) -> &'static str {
    match code {
        -1 => "NONE",
        0 => "DataBarOmni",
        1 => "DataBarTruncated",
        2 => "DataBarStacked",
        3 => "DataBarStackedOmni",
        4 => "DataBarLimited",
        5 => "DataBarExpanded",
        6 => "UPCA",
        7 => "UPCE",
        8 => "EAN13",
        9 => "EAN8",
        10 => "GS1_128_CCA",
        11 => "GS1_128_CCC",
        12 => "QR",
        13 => "DM",
        14 => "DotCode",
        _ => "UNKNOWN",
    }
}

fn run_ref(api: &RefApi, scan_data: &str) -> Result<RefOutcome, String> {
    let ctx = unsafe { (api.init)(std::ptr::null_mut()) };
    if ctx.is_null() {
        return Err("gs1_encoder_init returned null".to_owned());
    }

    let c_scan =
        CString::new(scan_data).map_err(|_| "scan_data contains interior NUL byte".to_owned())?;
    let ok = unsafe { (api.set_scan_data)(ctx, c_scan.as_ptr()) };
    let data_ptr = unsafe { (api.get_data_str)(ctx) };
    let sym = unsafe { (api.get_sym)(ctx) };
    let ai_ptr = unsafe { (api.get_ai_data_str)(ctx) };
    let ai_data_bracketed = if ai_ptr.is_null() {
        None
    } else {
        Some(
            unsafe { CStr::from_ptr(ai_ptr) }
                .to_string_lossy()
                .into_owned(),
        )
    };
    let mut hri_ptr: *const *const c_char = std::ptr::null();
    let hri_count = unsafe { (api.get_hri)(ctx, &mut hri_ptr as *mut *const *const c_char) };

    let data_str = if data_ptr.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(data_ptr) }
            .to_string_lossy()
            .into_owned()
    };
    let hri_compact = if hri_count <= 0 || hri_ptr.is_null() {
        None
    } else {
        let mut parts = Vec::with_capacity(hri_count as usize);
        for idx in 0..(hri_count as usize) {
            let item_ptr = unsafe { *hri_ptr.add(idx) };
            if item_ptr.is_null() {
                continue;
            }
            let item = unsafe { CStr::from_ptr(item_ptr) }
                .to_string_lossy()
                .into_owned();
            parts.push(item.replace(") ", ")"));
        }
        Some(parts.join(""))
    };
    unsafe { (api.free)(ctx) };

    Ok(RefOutcome {
        ok,
        sym_name: sym_name(sym).to_owned(),
        data_str,
        ai_data_bracketed,
        hri_compact,
    })
}

fn ref_cwd_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct CwdGuard {
    old_cwd: PathBuf,
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.old_cwd);
    }
}

fn with_ref_api<F>(f: F)
where
    F: FnOnce(&RefApi),
{
    let Some(lib_path) = std::env::var("AIDC_GS1_REF_LIB").ok() else {
        eprintln!(
            "skipping differential tests: set AIDC_GS1_REF_LIB=/abs/path/libgs1encoders.(dylib|so)"
        );
        return;
    };

    let _lock = ref_cwd_lock().lock().expect("failed to lock cwd guard");
    let old_cwd = std::env::current_dir().expect("failed to read cwd");
    let _cwd_guard = CwdGuard { old_cwd };

    let lib_parent = PathBuf::from(&lib_path)
        .parent()
        .unwrap_or_else(|| panic!("invalid AIDC_GS1_REF_LIB path: {lib_path}"))
        .to_path_buf();
    let c_lib_dir = lib_parent
        .parent()
        .unwrap_or_else(|| panic!("AIDC_GS1_REF_LIB should point inside c-lib/build: {lib_path}"))
        .to_path_buf();
    std::env::set_current_dir(&c_lib_dir)
        .unwrap_or_else(|e| panic!("failed to switch cwd to {}: {e}", c_lib_dir.display()));

    let api = RefApi::load(&lib_path).unwrap_or_else(|e| panic!("{e}"));
    f(&api);
}

fn assert_parity(api: &RefApi, scan_data: &str) {
    let rust = process_scan_data(scan_data);
    let reference = run_ref(api, scan_data)
        .unwrap_or_else(|e| panic!("reference call failed for {:?}: {e}", scan_data));

    match (rust, reference.ok) {
        (Ok(out), true) => {
            assert_eq!(
                out.sym_name, reference.sym_name,
                "sym mismatch for scan_data {:?}",
                scan_data
            );
            assert_eq!(
                out.data_str, reference.data_str,
                "data_str mismatch for scan_data {:?}",
                scan_data
            );
        }
        (Err(_), false) => {}
        (Ok(out), false) => panic!(
            "rust accepted but reference rejected for {:?}; rust=({}, {:?})",
            scan_data, out.sym_name, out.data_str
        ),
        (Err(err), true) => panic!(
            "reference accepted but rust rejected for {:?}; rust_err={}",
            scan_data, err
        ),
    }
}

fn rust_semantics(scan_data: &str) -> Result<Option<(String, String)>, String> {
    let scan = scan_data.as_bytes();
    let input = ScanInput::from_aim_scan(scan)
        .map_err(|e| format!("failed to parse AIM scan {:?}: {e}", scan_data))?;
    let decoded = Gs1Codec.decode(input).map_err(|e| e.to_string())?;
    let Some(elements) = decoded.parsed.ai_elements() else {
        return Ok(None);
    };
    let mut bracketed = String::new();
    for el in elements {
        bracketed.push('(');
        bracketed.push_str(el.ai.code());
        bracketed.push(')');
        bracketed.push_str(&el.value);
    }
    let hri = decoded.to_hri().unwrap_or_default();
    Ok(Some((bracketed, hri)))
}

fn assert_semantic_parity(api: &RefApi, scan_data: &str) {
    if !scan_data.starts_with("]d2")
        && !scan_data.starts_with("]Q3")
        && !scan_data.starts_with("]C1")
        && !scan_data.starts_with("]e0")
        && !scan_data.starts_with("]J1")
        && !scan_data.starts_with("]d1")
        && !scan_data.starts_with("]Q1")
    {
        return;
    }
    let reference = run_ref(api, scan_data)
        .unwrap_or_else(|e| panic!("reference call failed for {:?}: {e}", scan_data));
    let Some(reference_ai) = reference.ai_data_bracketed else {
        return;
    };
    let (rust_ai, rust_hri) = match rust_semantics(scan_data) {
        Ok(Some(v)) => v,
        Ok(None) => panic!(
            "rust semantic decode returned no AI elements for {:?}",
            scan_data
        ),
        Err(e) => panic!("rust semantic decode failed for {:?}: {e}", scan_data),
    };

    assert_eq!(
        rust_ai, reference_ai,
        "AI bracketed parity mismatch for scan_data {:?}",
        scan_data
    );
    if let Some(reference_hri) = reference.hri_compact {
        assert_eq!(
            rust_hri, reference_hri,
            "HRI parity mismatch for scan_data {:?}",
            scan_data
        );
    }
}

fn mod10_check_digit(base: &str) -> u32 {
    let mut sum = 0u32;
    for (idx, ch) in base.chars().rev().enumerate() {
        let d = u32::from((ch as u8) - b'0');
        sum += if idx % 2 == 0 { 3 * d } else { d };
    }
    (10 - (sum % 10)) % 10
}

fn gtin14_from_13(base13: &str) -> String {
    format!("{base13}{}", mod10_check_digit(base13))
}

fn fnc1_encoded_payload() -> impl Strategy<Value = String> {
    let ai01 = proptest::string::string_regex("[1-9][0-9]{12}")
        .expect("valid regex")
        .prop_map(|base| format!("01{}", gtin14_from_13(&base)));
    let ai10 = proptest::string::string_regex("[A-Z0-9]{1,12}")
        .expect("valid regex")
        .prop_map(|v| format!("10{v}"));
    let ai21 = proptest::string::string_regex("[A-Z0-9]{1,12}")
        .expect("valid regex")
        .prop_map(|v| format!("21{v}"));
    let ai17 =
        (0u8..100, 1u8..13, 1u8..29).prop_map(|(yy, mm, dd)| format!("17{yy:02}{mm:02}{dd:02}"));
    (
        ai01,
        prop::option::of(ai17),
        prop::option::of(ai10),
        prop::option::of(ai21),
    )
        .prop_map(|(head, a17, a10, a21)| {
            let mut payload = head;
            for t in [a17, a10, a21].into_iter().flatten() {
                payload.push('\u{001d}');
                payload.push_str(&t);
            }
            payload
        })
}

#[test]
fn differential_scandata_matches_reference() {
    let strict_semantic = std::env::var("AIDC_GS1_DIFF_STRICT_SEMANTIC").as_deref() == Ok("1");
    with_ref_api(|api| {
        let cases = read_scandata_cases();
        assert!(
            !cases.is_empty(),
            "no scan-data fixtures found for differential test"
        );

        for case in cases {
            assert_parity(api, &case.scan_data);
            if strict_semantic {
                assert_semantic_parity(api, &case.scan_data);
            }
        }
    });
}

#[test]
fn differential_scandata_fuzz_matches_reference() {
    if std::env::var("AIDC_GS1_DIFF_PROPTEST").as_deref() != Ok("1") {
        eprintln!(
            "skipping differential_scandata_fuzz_matches_reference: set AIDC_GS1_DIFF_PROPTEST=1"
        );
        return;
    }

    let cases = std::env::var("AIDC_GS1_DIFF_PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(256);

    with_ref_api(|api| {
        let strict_semantic = std::env::var("AIDC_GS1_DIFF_STRICT_SEMANTIC").as_deref() == Ok("1");
        let mut runner = TestRunner::new(ProptestConfig {
            cases,
            ..ProptestConfig::default()
        });
        let strategy = fnc1_encoded_payload().prop_map(|payload| format!("]d2{payload}"));
        runner
            .run(&strategy, |scan_data| {
                assert_parity(api, &scan_data);
                if strict_semantic {
                    assert_semantic_parity(api, &scan_data);
                }
                Ok(())
            })
            .unwrap_or_else(|e| panic!("differential proptest failed: {e}"));
    });
}
