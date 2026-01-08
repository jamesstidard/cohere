use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::schema::{ReferencedByRule, ReferencesRule, Schema, UniqueAcrossRule};

/// Result of validation
pub type ValidationResult = Result<(), Vec<ValidationError>>;

/// A single validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
    pub path: Option<String>,
    pub value: Option<String>,
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
