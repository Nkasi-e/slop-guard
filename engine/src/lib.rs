mod analysis;
mod protocol;

use analysis::run_all_analyzers;
use protocol::{AnalyzeRequest, AnalyzeResponse};
use std::sync::Mutex;
use std::{slice, str};

pub fn analyze_json(input: &str) -> Result<String, String> {
    let req: AnalyzeRequest = serde_json::from_str(input).map_err(|e| e.to_string())?;
    let issues = run_all_analyzers(&req);
    let resp = AnalyzeResponse { issues };
    serde_json::to_string(&resp).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// WASM memory helpers
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[no_mangle]
pub extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    unsafe {
        let _ = Vec::<u8>::from_raw_parts(ptr, len, len);
    }
}

// ---------------------------------------------------------------------------
// Output buffer — avoids struct-return ABI issues with WASM.
// JS: analyze(ptr, len) → len; then get_output_ptr() → read bytes until next analyze().
// Pointer remains valid until the next analyze() replaces the buffer (single-threaded WASM).
// ---------------------------------------------------------------------------

static OUTPUT_BUF: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// Runs analysis on the JSON input at `ptr`/`len`.
/// Returns the byte length of the JSON output (positive = ok, negative = error).
/// The output bytes are available via `get_output_ptr()` until the next call.
#[no_mangle]
pub extern "C" fn analyze(ptr: *const u8, len: usize) -> i32 {
    let mut out = OUTPUT_BUF.lock().unwrap_or_else(|e| e.into_inner());

    if ptr.is_null() || len == 0 {
        *out = b"empty input".to_vec();
        return -(out.len() as i32);
    }

    let input = unsafe { slice::from_raw_parts(ptr, len) };
    let input_str = match str::from_utf8(input) {
        Ok(s) => s,
        Err(_) => {
            *out = b"invalid UTF-8".to_vec();
            return -(out.len() as i32);
        }
    };

    match analyze_json(input_str) {
        Ok(json) => {
            let bytes = json.into_bytes();
            let out_len = bytes.len() as i32;
            *out = bytes;
            out_len
        }
        Err(err) => {
            let bytes = err.into_bytes();
            let n = bytes.len() as i32;
            *out = bytes;
            -n
        }
    }
}

/// Returns a pointer to the output buffer written by the last `analyze()` call.
#[no_mangle]
pub extern "C" fn get_output_ptr() -> *const u8 {
    let guard = OUTPUT_BUF.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_empty() {
        return std::ptr::null();
    }
    guard.as_ptr()
}
