use cohere_core::{validate_json, validate_toml as core_validate_toml, Schema};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
struct WasmValidationResult {
    valid: bool,
    errors: Vec<WasmValidationError>,
}

#[derive(Serialize)]
struct WasmValidationError {
    message: String,
    path: Option<String>,
    value: Option<String>,
    line: Option<usize>,
    column: Option<usize>,
}

fn make_result(
    result: Result<(), Vec<cohere_core::ValidationError>>,
) -> Result<JsValue, JsValue> {
    let wasm_result = match result {
        Ok(()) => WasmValidationResult {
            valid: true,
            errors: vec![],
        },
        Err(errors) => WasmValidationResult {
            valid: false,
            errors: errors
                .into_iter()
                .map(|e| WasmValidationError {
                    message: e.message,
                    path: e.path,
                    value: e.value,
                    line: e.line,
                    column: e.column,
                })
                .collect(),
        },
    };
    serde_wasm_bindgen::to_value(&wasm_result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// A compiled schema that can validate JSON and TOML data.
#[wasm_bindgen(js_name = Schema)]
pub struct WasmSchema {
    inner: Schema,
}

#[wasm_bindgen(js_class = Schema)]
impl WasmSchema {
    /// Create a new Schema from a JSON string or object.
    #[wasm_bindgen(constructor)]
    pub fn new(schema: JsValue) -> Result<WasmSchema, JsValue> {
        let json_str: String = if schema.is_string() {
            schema.as_string().unwrap()
        } else {
            js_sys::JSON::stringify(&schema)
                .map_err(|_| JsValue::from_str("Schema must be a string or object"))?
                .into()
        };

        let schema_value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| JsValue::from_str(&format!("Invalid schema JSON: {}", e)))?;

        let inner = Schema::parse(schema_value)
            .map_err(|e| JsValue::from_str(&format!("Invalid schema: {}", e)))?;

        Ok(WasmSchema { inner })
    }

    /// Validate a JSON string against this schema.
    #[wasm_bindgen(js_name = validateJson)]
    pub fn validate_json(&self, data_json: &str) -> Result<JsValue, JsValue> {
        let result = validate_json(&self.inner, data_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid JSON: {}", e)))?;
        make_result(result)
    }

    /// Validate a TOML string against this schema.
    #[wasm_bindgen(js_name = validateToml)]
    pub fn validate_toml(&self, data_toml: &str) -> Result<JsValue, JsValue> {
        let result = core_validate_toml(&self.inner, data_toml)
            .map_err(|e| JsValue::from_str(&format!("Invalid TOML: {}", e)))?;
        make_result(result)
    }
}
