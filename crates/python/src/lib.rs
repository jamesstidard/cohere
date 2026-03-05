use cohere_core::{validate_json, validate_toml as core_validate_toml, Schema, ValidationError};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

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

fn make_result(
    result: Result<(), Vec<ValidationError>>,
) -> ValidationResult {
    match result {
        Ok(()) => ValidationResult {
            valid: true,
            errors: vec![],
        },
        Err(errors) => ValidationResult {
            valid: false,
            errors: errors.into_iter().map(PyValidationError::from).collect(),
        },
    }
}

#[pyclass(name = "Schema")]
struct PySchema {
    inner: Schema,
}

#[pymethods]
impl PySchema {
    #[new]
    fn new(py: Python<'_>, schema: PyObject) -> PyResult<Self> {
        let json_str = if let Ok(dict) = schema.downcast::<PyDict>(py) {
            let json_module = py.import("json")?;
            let s = json_module.call_method1("dumps", (dict,))?;
            s.extract::<String>()?
        } else if let Ok(s) = schema.extract::<String>(py) {
            s
        } else {
            return Err(PyValueError::new_err(
                "Schema must be a dict or a JSON string",
            ));
        };

        let schema_value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyValueError::new_err(format!("Invalid schema JSON: {}", e)))?;

        let inner = Schema::parse(schema_value)
            .map_err(|e| PyValueError::new_err(format!("Invalid schema: {}", e)))?;

        Ok(Self { inner })
    }

    fn validate_json(&self, data_json: &str) -> PyResult<ValidationResult> {
        let result = validate_json(&self.inner, data_json)
            .map_err(|e| PyValueError::new_err(format!("Invalid JSON: {}", e)))?;
        Ok(make_result(result))
    }

    fn validate_toml(&self, data_toml: &str) -> PyResult<ValidationResult> {
        let result = core_validate_toml(&self.inner, data_toml)
            .map_err(|e| PyValueError::new_err(format!("Invalid TOML: {}", e)))?;
        Ok(make_result(result))
    }

    fn __repr__(&self) -> String {
        "Schema(...)".to_string()
    }
}

#[pymodule]
fn cohere(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PySchema>()?;
    m.add_class::<ValidationResult>()?;
    m.add_class::<PyValidationError>()?;
    Ok(())
}
