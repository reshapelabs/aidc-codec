use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::PathBuf;

use aidc_gs1::process_scan_data;
use libloading::{Library, Symbol};
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
}

struct RefApi {
    _lib: Library,
    init: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    free: unsafe extern "C" fn(*mut c_void),
    set_scan_data: unsafe extern "C" fn(*mut c_void, *const c_char) -> bool,
    get_data_str: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    get_sym: unsafe extern "C" fn(*mut c_void) -> c_int,
    get_err_msg: unsafe extern "C" fn(*mut c_void) -> *const c_char,
}

impl RefApi {
    fn load(path: &str) -> Result<Self, String> {
        let lib = unsafe { Library::new(path) }
            .map_err(|e| format!("failed to load reference lib {path}: {e}"))?;
        let init =
            unsafe { load_sym::<unsafe extern "C" fn(*mut c_void) -> *mut c_void>(&lib, b"gs1_encoder_init\0")? };
        let free = unsafe { load_sym::<unsafe extern "C" fn(*mut c_void)>(&lib, b"gs1_encoder_free\0")? };
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
        let get_sym =
            unsafe { load_sym::<unsafe extern "C" fn(*mut c_void) -> c_int>(&lib, b"gs1_encoder_getSym\0")? };
        let get_err_msg = unsafe {
            load_sym::<unsafe extern "C" fn(*mut c_void) -> *const c_char>(
                &lib,
                b"gs1_encoder_getErrMsg\0",
            )?
        };
        Ok(Self {
            _lib: lib,
            init,
            free,
            set_scan_data,
            get_data_str,
            get_sym,
            get_err_msg,
        })
    }
}

unsafe fn load_sym<T: Copy>(lib: &Library, name: &'static [u8]) -> Result<T, String> {
    let sym: Symbol<'_, T> = lib
        .get(name)
        .map_err(|e| format!("failed to load symbol {}: {e}", String::from_utf8_lossy(name)))?;
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

    let c_scan = CString::new(scan_data)
        .map_err(|_| "scan_data contains interior NUL byte".to_owned())?;
    let ok = unsafe { (api.set_scan_data)(ctx, c_scan.as_ptr()) };
    let data_ptr = unsafe { (api.get_data_str)(ctx) };
    let sym = unsafe { (api.get_sym)(ctx) };
    let _err_ptr = unsafe { (api.get_err_msg)(ctx) };

    let data_str = if data_ptr.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(data_ptr) }
            .to_string_lossy()
            .into_owned()
    };
    unsafe { (api.free)(ctx) };

    Ok(RefOutcome {
        ok,
        sym_name: sym_name(sym).to_owned(),
        data_str,
    })
}

#[test]
fn differential_scandata_matches_reference() {
    let Some(lib_path) = std::env::var("AIDC_GS1_REF_LIB").ok() else {
        eprintln!(
            "skipping differential_scandata_matches_reference: set AIDC_GS1_REF_LIB=/abs/path/libgs1encoders.(dylib|so)"
        );
        return;
    };

    let old_cwd = std::env::current_dir().expect("failed to read cwd");
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
    let cases = read_scandata_cases();
    assert!(
        !cases.is_empty(),
        "no scan-data fixtures found for differential test"
    );

    for case in cases {
        let rust = process_scan_data(&case.scan_data);
        let reference = run_ref(&api, &case.scan_data)
            .unwrap_or_else(|e| panic!("reference call failed for {:?}: {e}", case.scan_data));

        match (rust, reference.ok) {
            (Ok(out), true) => {
                assert_eq!(
                    out.sym_name, reference.sym_name,
                    "sym mismatch for scan_data {:?}",
                    case.scan_data
                );
                assert_eq!(
                    out.data_str, reference.data_str,
                    "data_str mismatch for scan_data {:?}",
                    case.scan_data
                );
            }
            (Err(_), false) => {}
            (Ok(out), false) => panic!(
                "rust accepted but reference rejected for {:?}; rust=({}, {:?})",
                case.scan_data, out.sym_name, out.data_str
            ),
            (Err(err), true) => panic!(
                "reference accepted but rust rejected for {:?}; rust_err={}",
                case.scan_data, err
            ),
        }
    }

    std::env::set_current_dir(old_cwd).expect("failed to restore cwd");
}
