use graph_validator_core::{validate_json, validate_toml, Schema, ValidationError};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// A validation error returned to Python
#[pyclass]
#[derive(Clone)]
struct PyValidationError {
    #[pyo3(get)]
    message: String,
    #[pyo3(get)]
    path: Option<String>,
    #[pyo3(get)]
    value: Option<String>,
    #[pyo3(get)]
    line: Option<usize>,
    #[pyo3(get)]
    column: Option<usize>,
}

#[pymethods]
impl PyValidationError {
    fn __repr__(&self) -> String {
        format!("ValidationError(message={:?}, path={:?})", self.message, self.path)
    }

    fn __str__(&self) -> String {
        self.message.clone()
    }
}

impl From<ValidationError> for PyValidationError {
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

/// Result of validation
#[pyclass]
struct ValidationResult {
    #[pyo3(get)]
    valid: bool,
    #[pyo3(get)]
    errors: Vec<PyValidationError>,
}

#[pymethods]
impl ValidationResult {
    fn __repr__(&self) -> String {
        format!(
            "ValidationResult(valid={}, errors={})",
            self.valid,
            self.errors.len()
        )
    }

    fn __bool__(&self) -> bool {
        self.valid
    }
}

/// Validate JSON data against a schema with custom x- keywords.
///
/// Error locations (line/column) are mapped back to the JSON source.
///
/// Args:
///     schema_json: JSON string of the schema
///     data_json: JSON string of the data to validate
///
/// Returns:
///     ValidationResult with `valid` bool and `errors` list (with line/column)
#[pyfunction]
fn validate_graph(schema_json: &str, data_json: &str) -> PyResult<ValidationResult> {
    let schema_value: serde_json::Value = serde_json::from_str(schema_json)
        .map_err(|e| PyValueError::new_err(format!("Invalid schema JSON: {}", e)))?;

    let schema = Schema::parse(schema_value)
        .map_err(|e| PyValueError::new_err(format!("Invalid schema: {}", e)))?;

    let result = validate_json(&schema, data_json)
        .map_err(|e| PyValueError::new_err(format!("Invalid data JSON: {}", e)))?;

    match result {
        Ok(()) => Ok(ValidationResult {
            valid: true,
            errors: vec![],
        }),
        Err(errors) => Ok(ValidationResult {
            valid: false,
            errors: errors.into_iter().map(PyValidationError::from).collect(),
        }),
    }
}

/// Validate using Python dicts instead of JSON strings
#[pyfunction]
fn validate_graph_dict(py: Python<'_>, schema: &PyDict, data: &PyDict) -> PyResult<ValidationResult> {
    let schema_json = dict_to_json_string(py, schema)?;
    let data_json = dict_to_json_string(py, data)?;
    validate_graph(&schema_json, &data_json)
}

fn dict_to_json_string(py: Python<'_>, dict: &PyDict) -> PyResult<String> {
    let json_module = py.import("json")?;
    let json_str = json_module.call_method1("dumps", (dict,))?;
    json_str.extract()
}

/// Validate TOML data against a schema with custom x- keywords.
///
/// Error locations (line/column) are mapped back to the TOML source.
///
/// Args:
///     schema_json: JSON string of the schema
///     data_toml: TOML string of the data to validate
///
/// Returns:
///     ValidationResult with `valid` bool and `errors` list (with line/column)
#[pyfunction]
fn validate_graph_toml(schema_json: &str, data_toml: &str) -> PyResult<ValidationResult> {
    let schema_value: serde_json::Value = serde_json::from_str(schema_json)
        .map_err(|e| PyValueError::new_err(format!("Invalid schema JSON: {}", e)))?;

    let schema = Schema::parse(schema_value)
        .map_err(|e| PyValueError::new_err(format!("Invalid schema: {}", e)))?;

    let result = validate_toml(&schema, data_toml)
        .map_err(|e| PyValueError::new_err(format!("Invalid TOML: {}", e)))?;

    match result {
        Ok(()) => Ok(ValidationResult {
            valid: true,
            errors: vec![],
        }),
        Err(errors) => Ok(ValidationResult {
            valid: false,
            errors: errors.into_iter().map(PyValidationError::from).collect(),
        }),
    }
}

/// Parse a schema and check for errors
#[pyfunction]
fn parse_schema(schema_json: &str) -> PyResult<()> {
    let schema_value: serde_json::Value = serde_json::from_str(schema_json)
        .map_err(|e| PyValueError::new_err(format!("Invalid JSON: {}", e)))?;

    Schema::parse(schema_value)
        .map_err(|e| PyValueError::new_err(format!("Invalid schema: {}", e)))?;

    Ok(())
}

#[pymodule]
fn graph_validator(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(validate_graph, m)?)?;
    m.add_function(wrap_pyfunction!(validate_graph_dict, m)?)?;
    m.add_function(wrap_pyfunction!(validate_graph_toml, m)?)?;
    m.add_function(wrap_pyfunction!(parse_schema, m)?)?;
    m.add_class::<ValidationResult>()?;
    m.add_class::<PyValidationError>()?;
    Ok(())
}
