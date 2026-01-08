use graph_validator_core::{validate, Schema};
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
}

/// Validate JSON data against a schema
///
/// # Arguments
/// * `schema_json` - JSON string of the schema (with x- keywords)
/// * `data_json` - JSON string of the data to validate
///
/// # Returns
/// A JavaScript object with `valid: boolean` and `errors: Array`
#[wasm_bindgen]
pub fn validate_graph(schema_json: &str, data_json: &str) -> Result<JsValue, JsValue> {
    let schema_value: serde_json::Value = serde_json::from_str(schema_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid schema JSON: {}", e)))?;

    let data_value: serde_json::Value = serde_json::from_str(data_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid data JSON: {}", e)))?;

    let schema = Schema::parse(schema_value)
        .map_err(|e| JsValue::from_str(&format!("Invalid schema: {}", e)))?;

    let result = match validate(&schema, &data_value) {
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
                })
                .collect(),
        },
    };

    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Parse a schema and return any errors
#[wasm_bindgen]
pub fn parse_schema(schema_json: &str) -> Result<JsValue, JsValue> {
    let schema_value: serde_json::Value = serde_json::from_str(schema_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid JSON: {}", e)))?;

    Schema::parse(schema_value)
        .map_err(|e| JsValue::from_str(&format!("Invalid schema: {}", e)))?;

    Ok(JsValue::from_str("Schema is valid"))
}
