mod analysis;
mod protocol;

use analysis::run_all_analyzers;
use protocol::{AnalyzeRequest, AnalyzeResponse};
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
// Static output buffer — avoids complex struct-return ABI issues with WASM.
// JS calls: analyze(input_ptr, input_len) → output_len (negative = error)
// JS reads: get_output_ptr() → pointer to output bytes inside WASM memory
// ---------------------------------------------------------------------------

static mut OUTPUT_BUF: Vec<u8> = Vec::new();

/// Runs analysis on the JSON input at `ptr`/`len`.
/// Returns the byte length of the JSON output (positive = ok, negative = error).
/// The output bytes are available via `get_output_ptr()` until the next call.
#[no_mangle]
pub extern "C" fn analyze(ptr: *const u8, len: usize) -> i32 {
    if ptr.is_null() || len == 0 {
        let msg = b"empty input";
        unsafe {
            OUTPUT_BUF = msg.to_vec();
        }
        return -(msg.len() as i32);
    }

    let input = unsafe { slice::from_raw_parts(ptr, len) };
    let input_str = match str::from_utf8(input) {
        Ok(s) => s,
        Err(_) => {
            let msg = b"invalid UTF-8";
            unsafe {
                OUTPUT_BUF = msg.to_vec();
            }
            return -(msg.len() as i32);
        }
    };

    match analyze_json(input_str) {
        Ok(json) => {
            let bytes = json.into_bytes();
            let out_len = bytes.len() as i32;
            unsafe {
                OUTPUT_BUF = bytes;
            }
            out_len
        }
        Err(err) => {
            let bytes = err.into_bytes();
            unsafe {
                OUTPUT_BUF = bytes.clone();
            }
            -(bytes.len() as i32)
        }
    }
}

/// Returns a pointer to the output buffer written by the last `analyze()` call.
#[no_mangle]
pub extern "C" fn get_output_ptr() -> *const u8 {
    unsafe { OUTPUT_BUF.as_ptr() }
}
