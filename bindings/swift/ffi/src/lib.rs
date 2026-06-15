//! C ABI for the Swift binding.
//!
//! The surface mirrors the WASM and Python bindings: a `Schema` is compiled
//! once from a JSON schema string, then used to validate JSON or TOML data.
//!
//! Everything crosses the boundary as UTF-8 C strings. Validation results are
//! returned as a JSON string with the shape:
//!
//! ```json
//! { "valid": false, "errors": [ { "message": "...", "path": "...", ... } ] }
//! ```
//!
//! Fallible operations report failure by writing an owned error string into an
//! out-parameter (`error_out`) and returning null / a sentinel. Callers own
//! every `*mut c_char` returned across the boundary and must release it with
//! [`cohere_string_free`]. Schemas must be released with [`cohere_schema_free`].

use std::ffi::{c_char, CStr, CString};
use std::ptr;

use cohere_core::{validate_json, validate_toml, Schema, ValidationError};
use serde::Serialize;

#[derive(Serialize)]
struct FfiValidationResult {
    valid: bool,
    errors: Vec<FfiValidationError>,
}

#[derive(Serialize)]
struct FfiValidationError {
    message: String,
    path: Option<String>,
    value: Option<String>,
    line: Option<usize>,
    column: Option<usize>,
}

impl From<ValidationError> for FfiValidationError {
    fn from(e: ValidationError) -> Self {
        Self {
            message: e.message,
            path: e.path,
            value: e.value,
            line: e.line,
            column: e.column,
        }
    }
}

/// Convert an owned Rust string into a heap-allocated C string.
///
/// Returns null only if `s` contains an interior NUL byte, which never happens
/// for the JSON/error text produced here.
fn into_c_string(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(c) => c.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Write an error message into `error_out` if it is non-null.
fn set_error(error_out: *mut *mut c_char, message: String) {
    if !error_out.is_null() {
        unsafe { *error_out = into_c_string(message) };
    }
}

/// Borrow a C string as `&str`, or return `Err` with a message on null / bad UTF-8.
///
/// # Safety
/// `ptr` must be null or a valid NUL-terminated C string.
unsafe fn cstr_to_str<'a>(ptr: *const c_char, what: &str) -> Result<&'a str, String> {
    if ptr.is_null() {
        return Err(format!("{} pointer is null", what));
    }
    CStr::from_ptr(ptr)
        .to_str()
        .map_err(|_| format!("{} is not valid UTF-8", what))
}

fn result_to_json(result: Result<(), Vec<ValidationError>>) -> String {
    let ffi = match result {
        Ok(()) => FfiValidationResult {
            valid: true,
            errors: vec![],
        },
        Err(errors) => FfiValidationResult {
            valid: false,
            errors: errors.into_iter().map(FfiValidationError::from).collect(),
        },
    };
    // Serializing this fixed, simple structure cannot fail.
    serde_json::to_string(&ffi).unwrap_or_else(|e| {
        format!("{{\"valid\":false,\"errors\":[{{\"message\":\"internal serialization error: {}\"}}]}}", e)
    })
}

/// Compile a schema from a JSON schema string.
///
/// On success returns an opaque, non-null `Schema` pointer that must be released
/// with [`cohere_schema_free`]. On failure returns null and, if `error_out` is
/// non-null, writes an owned error string to it (release with
/// [`cohere_string_free`]).
///
/// # Safety
/// `schema_json` must be a valid NUL-terminated C string. `error_out` must be
/// null or a valid pointer to a writable `*mut c_char`.
#[no_mangle]
pub unsafe extern "C" fn cohere_schema_new(
    schema_json: *const c_char,
    error_out: *mut *mut c_char,
) -> *mut Schema {
    if !error_out.is_null() {
        *error_out = ptr::null_mut();
    }

    let schema_str = match cstr_to_str(schema_json, "schema") {
        Ok(s) => s,
        Err(e) => {
            set_error(error_out, e);
            return ptr::null_mut();
        }
    };

    let schema_value: serde_json::Value = match serde_json::from_str(schema_str) {
        Ok(v) => v,
        Err(e) => {
            set_error(error_out, format!("Invalid schema JSON: {}", e));
            return ptr::null_mut();
        }
    };

    match Schema::parse(schema_value) {
        Ok(schema) => Box::into_raw(Box::new(schema)),
        Err(e) => {
            set_error(error_out, format!("Invalid schema: {}", e));
            ptr::null_mut()
        }
    }
}

/// Release a schema created by [`cohere_schema_new`]. Passing null is a no-op.
///
/// # Safety
/// `schema` must be null or a pointer returned by [`cohere_schema_new`] that has
/// not already been freed.
#[no_mangle]
pub unsafe extern "C" fn cohere_schema_free(schema: *mut Schema) {
    if !schema.is_null() {
        drop(Box::from_raw(schema));
    }
}

/// Validate a JSON data string against `schema`.
///
/// Returns an owned JSON result string (release with [`cohere_string_free`]).
/// Returns null and writes to `error_out` only when the input could not be
/// parsed at all (e.g. malformed JSON or a null schema); schema *violations* are
/// reported inside the returned result string, not via `error_out`.
///
/// # Safety
/// `schema` must be a valid pointer from [`cohere_schema_new`]. `data_json` must
/// be a valid NUL-terminated C string. `error_out` must be null or a valid
/// pointer to a writable `*mut c_char`.
#[no_mangle]
pub unsafe extern "C" fn cohere_validate_json(
    schema: *const Schema,
    data_json: *const c_char,
    error_out: *mut *mut c_char,
) -> *mut c_char {
    validate_with(schema, data_json, error_out, "JSON", |s, data| {
        validate_json(s, data)
    })
}

/// Validate a TOML data string against `schema`. See [`cohere_validate_json`].
///
/// # Safety
/// Same contract as [`cohere_validate_json`], with `data_toml` as a TOML string.
#[no_mangle]
pub unsafe extern "C" fn cohere_validate_toml(
    schema: *const Schema,
    data_toml: *const c_char,
    error_out: *mut *mut c_char,
) -> *mut c_char {
    validate_with(schema, data_toml, error_out, "TOML", |s, data| {
        validate_toml(s, data)
    })
}

/// Shared body for the validate entry points.
///
/// # Safety
/// See [`cohere_validate_json`].
unsafe fn validate_with<E: std::fmt::Display>(
    schema: *const Schema,
    data: *const c_char,
    error_out: *mut *mut c_char,
    kind: &str,
    f: impl Fn(&Schema, &str) -> Result<Result<(), Vec<ValidationError>>, E>,
) -> *mut c_char {
    if !error_out.is_null() {
        *error_out = ptr::null_mut();
    }

    if schema.is_null() {
        set_error(error_out, "schema pointer is null".to_string());
        return ptr::null_mut();
    }
    let schema = &*schema;

    let data_str = match cstr_to_str(data, "data") {
        Ok(s) => s,
        Err(e) => {
            set_error(error_out, e);
            return ptr::null_mut();
        }
    };

    match f(schema, data_str) {
        Ok(result) => into_c_string(result_to_json(result)),
        Err(e) => {
            set_error(error_out, format!("Invalid {}: {}", kind, e));
            ptr::null_mut()
        }
    }
}

/// Release a string returned by any of the `cohere_*` functions (validation
/// results or error strings). Passing null is a no-op.
///
/// # Safety
/// `s` must be null or a pointer returned by this library that has not already
/// been freed.
#[no_mangle]
pub unsafe extern "C" fn cohere_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(s: &str) -> CString {
        CString::new(s).unwrap()
    }

    unsafe fn take_string(ptr: *mut c_char) -> String {
        assert!(!ptr.is_null());
        let s = CStr::from_ptr(ptr).to_str().unwrap().to_owned();
        cohere_string_free(ptr);
        s
    }

    const SCHEMA: &str = r#"{
        "type": "object",
        "properties": { "name": { "type": "string" } },
        "required": ["name"]
    }"#;

    #[test]
    fn valid_json_round_trip() {
        unsafe {
            let mut err: *mut c_char = ptr::null_mut();
            let schema = cohere_schema_new(c(SCHEMA).as_ptr(), &mut err);
            assert!(!schema.is_null());
            assert!(err.is_null());

            let out = cohere_validate_json(schema, c(r#"{"name":"alice"}"#).as_ptr(), &mut err);
            let json = take_string(out);
            assert!(err.is_null());
            assert!(json.contains("\"valid\":true"));

            cohere_schema_free(schema);
        }
    }

    #[test]
    fn invalid_json_reports_errors_in_result() {
        unsafe {
            let mut err: *mut c_char = ptr::null_mut();
            let schema = cohere_schema_new(c(SCHEMA).as_ptr(), &mut err);

            let out = cohere_validate_json(schema, c(r#"{}"#).as_ptr(), &mut err);
            let json = take_string(out);
            assert!(err.is_null());
            assert!(json.contains("\"valid\":false"));
            assert!(json.contains("\"errors\""));

            cohere_schema_free(schema);
        }
    }

    #[test]
    fn malformed_json_sets_error_out() {
        unsafe {
            let mut err: *mut c_char = ptr::null_mut();
            let schema = cohere_schema_new(c(SCHEMA).as_ptr(), &mut err);

            let out = cohere_validate_json(schema, c("{not json").as_ptr(), &mut err);
            assert!(out.is_null());
            assert!(!err.is_null());
            let msg = take_string(err);
            assert!(msg.contains("Invalid JSON"));

            cohere_schema_free(schema);
        }
    }

    #[test]
    fn bad_schema_sets_error_out() {
        unsafe {
            let mut err: *mut c_char = ptr::null_mut();
            let schema = cohere_schema_new(c("{not json").as_ptr(), &mut err);
            assert!(schema.is_null());
            assert!(!err.is_null());
            let msg = take_string(err);
            assert!(msg.contains("Invalid schema JSON"));
        }
    }

    #[test]
    fn null_safety() {
        unsafe {
            // Freeing null is a no-op.
            cohere_schema_free(ptr::null_mut());
            cohere_string_free(ptr::null_mut());

            // Null schema on validate is reported, not a crash.
            let mut err: *mut c_char = ptr::null_mut();
            let out = cohere_validate_json(ptr::null(), c("{}").as_ptr(), &mut err);
            assert!(out.is_null());
            assert!(!err.is_null());
            cohere_string_free(err);
        }
    }
}
