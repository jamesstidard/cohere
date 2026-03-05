use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::schema::{ReferencedByRule, ReferencesRule, Schema, UniqueAcrossRule};
use crate::source_map::SourceMap;

/// Result of validation
pub type ValidationResult = Result<(), Vec<ValidationError>>;

/// A single validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
    pub path: Option<String>,
    pub value: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Validate a JSON value against a schema
pub fn validate(schema: &Schema, data: &Value) -> ValidationResult {
    let mut errors = Vec::new();

    // Standard JSON Schema validation first
    if let Some(ref validator) = schema.json_schema {
        errors.extend(validate_json_schema(validator, data));
    }

    // Custom x- keyword validation
    for rule in &schema.unique_across {
        errors.extend(validate_unique_across(rule, data));
    }

    for rule in &schema.references {
        errors.extend(validate_references(rule, data));
    }

    for rule in &schema.referenced_by {
        errors.extend(validate_referenced_by(rule, data));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate a JSON string against a schema, with error locations mapped
/// back to the JSON source.
///
/// Returns `Err(JsonError)` if the JSON cannot be parsed,
/// or `Ok(ValidationResult)` with enriched errors containing line/column info.
pub fn validate_json(schema: &Schema, json_src: &str) -> Result<ValidationResult, crate::json::JsonError> {
    let (data, source_map) = crate::json::json_with_source_map(json_src)?;

    match validate(schema, &data) {
        Ok(()) => Ok(Ok(())),
        Err(errors) => Ok(Err(enrich_errors(errors, &source_map))),
    }
}

/// Validate a TOML string against a schema, with error locations mapped
/// back to the TOML source.
///
/// Returns `Err(TomlError)` if the TOML cannot be parsed,
/// or `Ok(ValidationResult)` with enriched errors containing line/column info.
pub fn validate_toml(schema: &Schema, toml_src: &str) -> Result<ValidationResult, crate::toml::TomlError> {
    let (data, source_map) = crate::toml::toml_to_json(toml_src)?;

    match validate(schema, &data) {
        Ok(()) => Ok(Ok(())),
        Err(errors) => Ok(Err(enrich_errors(errors, &source_map))),
    }
}

/// Enrich validation errors with source location information from a source map.
pub fn enrich_errors(errors: Vec<ValidationError>, source_map: &SourceMap) -> Vec<ValidationError> {
    errors
        .into_iter()
        .map(|mut err| {
            if let Some(ref path) = err.path {
                if let Some(span) = source_map.get(path) {
                    err.line = Some(span.line);
                    err.column = Some(span.column);
                }
            }
            err
        })
        .collect()
}

fn validate_json_schema(
    validator: &jsonschema::Validator,
    data: &Value,
) -> Vec<ValidationError> {
    validator
        .iter_errors(data)
        .map(|e| convert_jsonschema_error(&e))
        .collect()
}

fn convert_jsonschema_error(error: &jsonschema::ValidationError) -> ValidationError {
    // Convert JSONPointer path (/foo/0/bar) to JSONPath (foo[0].bar)
    let path = jsonpointer_to_jsonpath(error.instance_path().as_str());

    // Get the failing value as string
    let value = value_to_string(error.instance());

    ValidationError {
        message: error.to_string(),
        path: if path.is_empty() { None } else { Some(path) },
        value: Some(value),
        line: None,
        column: None,
    }
}

fn jsonpointer_to_jsonpath(pointer: &str) -> String {
    if pointer.is_empty() || pointer == "/" {
        return String::new();
    }

    // JSONPointer: /nodes/0/name
    // JSONPath: nodes[0].name

    let segments: Vec<&str> = pointer.trim_start_matches('/').split('/').collect();
    let mut result = String::new();

    for (i, segment) in segments.iter().enumerate() {
        // Check if segment is a number (array index)
        if segment.parse::<usize>().is_ok() {
            result.push_str(&format!("[{}]", segment));
        } else {
            // Field name - add dot if result is not empty and doesn't end with a dot
            if !result.is_empty() && i > 0 {
                result.push('.');
            }
            result.push_str(segment);
        }
    }

    result
}

fn validate_unique_across(rule: &UniqueAcrossRule, data: &Value) -> Vec<ValidationError> {
    let mut seen: HashMap<&Value, String> = HashMap::new();
    let mut errors = Vec::new();

    for path in &rule.paths {
        for m in path.evaluate(data) {
            if let Some(first_path) = seen.get(m.value) {
                let message = rule
                    .message
                    .as_ref()
                    .map(|msg| interpolate_message(msg, m.value, &m.path, None))
                    .unwrap_or_else(|| {
                        format!(
                            "Duplicate value '{}' at '{}' (first seen at '{}')",
                            value_to_string(m.value),
                            m.path,
                            first_path
                        )
                    });

                errors.push(ValidationError {
                    message,
                    path: Some(m.path),
                    value: Some(value_to_string(m.value)),
                    line: None,
                    column: None,
                });
            } else {
                seen.insert(m.value, m.path);
            }
        }
    }

    errors
}

fn validate_references(rule: &ReferencesRule, data: &Value) -> Vec<ValidationError> {
    // Collect all valid target values
    let valid_targets: HashSet<&Value> = rule
        .to
        .iter()
        .flat_map(|path| path.evaluate(data))
        .map(|m| m.value)
        .collect();

    let mut errors = Vec::new();

    for m in rule.from.evaluate(data) {
        if !valid_targets.contains(m.value) {
            let message = rule
                .message
                .as_ref()
                .map(|msg| interpolate_message(msg, m.value, &m.path, None))
                .unwrap_or_else(|| {
                    format!(
                        "Invalid reference '{}' at '{}'",
                        value_to_string(m.value),
                        m.path
                    )
                });

            errors.push(ValidationError {
                message,
                path: Some(m.path),
                value: Some(value_to_string(m.value)),
                line: None,
                column: None,
            });
        }
    }

    errors
}

fn validate_referenced_by(rule: &ReferencedByRule, data: &Value) -> Vec<ValidationError> {
    // Collect all reference sources
    let references: Vec<&Value> = rule
        .from
        .iter()
        .flat_map(|path| path.evaluate(data))
        .map(|m| m.value)
        .collect();

    let mut errors = Vec::new();

    for m in rule.target.evaluate(data) {
        let count = references.iter().filter(|&&v| v == m.value).count();

        if count < rule.min {
            let message = rule
                .message
                .as_ref()
                .map(|msg| interpolate_message(msg, m.value, &m.path, None))
                .unwrap_or_else(|| {
                    format!(
                        "Value '{}' at '{}' is referenced {} time(s), minimum is {}",
                        value_to_string(m.value),
                        m.path,
                        count,
                        rule.min
                    )
                });

            errors.push(ValidationError {
                message,
                path: Some(m.path.clone()),
                value: Some(value_to_string(m.value)),
                line: None,
                column: None,
            });
        }

        if let Some(max) = rule.max {
            if count > max {
                let message = format!(
                    "Value '{}' at '{}' is referenced {} time(s), maximum is {}",
                    value_to_string(m.value),
                    m.path,
                    count,
                    max
                );

                errors.push(ValidationError {
                    message,
                    path: Some(m.path),
                    value: Some(value_to_string(m.value)),
                    line: None,
                    column: None,
                });
            }
        }
    }

    errors
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn interpolate_message(template: &str, value: &Value, path: &str, index: Option<usize>) -> String {
    let mut result = template.to_string();
    result = result.replace("{{value}}", &value_to_string(value));
    result = result.replace("{{path}}", path);
    if let Some(i) = index {
        result = result.replace("{{index}}", &i.to_string());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_jsonpointer_to_jsonpath() {
        assert_eq!(jsonpointer_to_jsonpath(""), "");
        assert_eq!(jsonpointer_to_jsonpath("/"), "");
        assert_eq!(jsonpointer_to_jsonpath("/nodes"), "nodes");
        assert_eq!(jsonpointer_to_jsonpath("/nodes/0"), "nodes[0]");
        assert_eq!(jsonpointer_to_jsonpath("/nodes/0/name"), "nodes[0].name");
        assert_eq!(
            jsonpointer_to_jsonpath("/users/0/addresses/1/city"),
            "users[0].addresses[1].city"
        );
    }

    #[test]
    fn test_standard_json_schema_valid() {
        let schema = Schema::parse(json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"]
        }))
        .unwrap();

        let valid = json!({ "name": "Alice" });
        assert!(validate(&schema, &valid).is_ok());
    }

    #[test]
    fn test_standard_json_schema_invalid_type() {
        let schema = Schema::parse(json!({
            "type": "object",
            "properties": {
                "age": { "type": "number" }
            }
        }))
        .unwrap();

        let invalid = json!({ "age": "not-a-number" });
        let result = validate(&schema, &invalid);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].path, Some("age".to_string()));
    }

    #[test]
    fn test_standard_json_schema_required_missing() {
        let schema = Schema::parse(json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"]
        }))
        .unwrap();

        let invalid = json!({});
        let result = validate(&schema, &invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_combined_validation_both_pass() {
        let schema = Schema::parse(json!({
            "type": "object",
            "properties": {
                "nodes": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" }
                        },
                        "required": ["name"]
                    }
                }
            },
            "x-uniqueAcross": [
                { "paths": ["nodes[*].name"] }
            ]
        }))
        .unwrap();

        let valid = json!({
            "nodes": [
                { "name": "a" },
                { "name": "b" }
            ]
        });

        assert!(validate(&schema, &valid).is_ok());
    }

    #[test]
    fn test_combined_validation_both_fail() {
        let schema = Schema::parse(json!({
            "type": "object",
            "properties": {
                "nodes": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" }
                        },
                        "required": ["name"]
                    }
                }
            },
            "x-uniqueAcross": [
                { "paths": ["nodes[*].name"] }
            ]
        }))
        .unwrap();

        let invalid = json!({
            "nodes": [
                { "name": "a" },
                { "name": "a" },  // duplicate (x-uniqueAcross violation)
                { "age": 30 }      // missing required "name" (JSON Schema)
            ]
        });

        let result = validate(&schema, &invalid);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.len() >= 2); // At least one from each validator
    }

    #[test]
    fn test_x_keywords_only_schema() {
        // Schema with ONLY x- keywords should work
        let schema = Schema::parse(json!({
            "x-uniqueAcross": [
                { "paths": ["items[*].id"] }
            ]
        }))
        .unwrap();

        assert!(schema.json_schema.is_none());

        let valid = json!({ "items": [{ "id": "a" }, { "id": "b" }] });
        assert!(validate(&schema, &valid).is_ok());
    }

    #[test]
    fn test_empty_schema() {
        // Empty schema should always be valid
        let schema = Schema::parse(json!({})).unwrap();
        assert!(schema.json_schema.is_none());

        let data = json!({ "anything": "goes" });
        assert!(validate(&schema, &data).is_ok());
    }

    #[test]
    fn test_validate_unique_across() {
        let schema = Schema::parse(json!({
            "x-uniqueAcross": [
                { "paths": ["nodes[*].name", "edges[*].name"] }
            ]
        }))
        .unwrap();

        // Valid: unique names
        let valid = json!({
            "nodes": [{"name": "a"}, {"name": "b"}],
            "edges": [{"name": "c"}]
        });
        assert!(validate(&schema, &valid).is_ok());

        // Invalid: duplicate names
        let invalid = json!({
            "nodes": [{"name": "a"}, {"name": "b"}],
            "edges": [{"name": "a"}]
        });
        assert!(validate(&schema, &invalid).is_err());
    }

    #[test]
    fn test_validate_references() {
        let schema = Schema::parse(json!({
            "x-references": [
                { "from": "edges[*].from", "to": ["nodes[*].name"] }
            ]
        }))
        .unwrap();

        // Valid: reference exists
        let valid = json!({
            "nodes": [{"name": "a"}],
            "edges": [{"from": "a"}]
        });
        assert!(validate(&schema, &valid).is_ok());

        // Invalid: reference doesn't exist
        let invalid = json!({
            "nodes": [{"name": "a"}],
            "edges": [{"from": "b"}]
        });
        assert!(validate(&schema, &invalid).is_err());
    }

    #[test]
    fn test_validate_referenced_by() {
        let schema = Schema::parse(json!({
            "x-referencedBy": [
                { "target": "nodes[*].name", "from": ["edges[*].from", "edges[*].to"], "min": 1 }
            ]
        }))
        .unwrap();

        // Valid: node is referenced
        let valid = json!({
            "nodes": [{"name": "a"}],
            "edges": [{"from": "a", "to": "a"}]
        });
        assert!(validate(&schema, &valid).is_ok());

        // Invalid: node is not referenced
        let invalid = json!({
            "nodes": [{"name": "a"}, {"name": "b"}],
            "edges": [{"from": "a", "to": "a"}]
        });
        assert!(validate(&schema, &invalid).is_err());
    }
}
