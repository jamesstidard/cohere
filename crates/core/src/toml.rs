use serde_json::Value as JsonValue;
use toml_span::value::{Value as TomlValue, ValueInner};

use crate::source_map::{build_line_offsets, make_span, SourceMap};

/// Error from TOML parsing or conversion
#[derive(Debug, thiserror::Error)]
pub enum TomlError {
    #[error("TOML parse error: {0}")]
    Parse(String),
}

/// Parse a TOML string into a JSON value and a source map that maps
/// JSON paths back to byte offsets (and line/column) in the TOML source.
pub fn toml_to_json(toml_src: &str) -> Result<(JsonValue, SourceMap), TomlError> {
    let toml_value =
        toml_span::parse(toml_src).map_err(|e| TomlError::Parse(format!("{}", e)))?;

    let line_offsets = build_line_offsets(toml_src);
    let mut source_map = SourceMap::new();

    let json = convert_value(&toml_value, "", &line_offsets, &mut source_map);

    Ok((json, source_map))
}

/// Recursively convert a toml_span Value to serde_json Value,
/// recording source spans in the source map.
fn convert_value(
    value: &TomlValue<'_>,
    path: &str,
    line_offsets: &[usize],
    source_map: &mut SourceMap,
) -> JsonValue {
    if !path.is_empty() {
        source_map.insert(
            path.to_string(),
            make_span(value.span.start, value.span.end, line_offsets),
        );
    }

    match value.as_ref() {
        ValueInner::String(s) => JsonValue::String(s.to_string()),
        ValueInner::Integer(i) => serde_json::json!(*i),
        ValueInner::Float(f) => serde_json::json!(*f),
        ValueInner::Boolean(b) => JsonValue::Bool(*b),
        ValueInner::Array(arr) => {
            let items: Vec<JsonValue> = arr
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let child_path = format!("{}[{}]", path, i);
                    convert_value(v, &child_path, line_offsets, source_map)
                })
                .collect();
            JsonValue::Array(items)
        }
        ValueInner::Table(table) => {
            let mut map = serde_json::Map::new();
            for (key, val) in table {
                let child_path = if path.is_empty() {
                    key.name.to_string()
                } else {
                    format!("{}.{}", path, key.name)
                };
                map.insert(
                    key.name.to_string(),
                    convert_value(val, &child_path, line_offsets, source_map),
                );
            }
            JsonValue::Object(map)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_toml_to_json() {
        let toml = r#"
name = "alice"
age = 30
"#;
        let (json, source_map) = toml_to_json(toml).unwrap();
        assert_eq!(json["name"], "alice");
        assert_eq!(json["age"], 30);

        assert!(source_map.contains_key("name"));
        assert!(source_map.contains_key("age"));
    }

    #[test]
    fn test_nested_toml_to_json() {
        let toml = r#"
[[users]]
name = "alice"

[[users]]
name = "bob"

[[organisations]]
name = "acme"
members = ["alice"]
"#;
        let (json, source_map) = toml_to_json(toml).unwrap();

        assert_eq!(json["users"][0]["name"], "alice");
        assert_eq!(json["users"][1]["name"], "bob");
        assert_eq!(json["organisations"][0]["members"][0], "alice");

        assert!(source_map.contains_key("users[0].name"));
        assert!(source_map.contains_key("users[1].name"));
        assert!(source_map.contains_key("organisations[0].members[0]"));

        // Verify line numbers make sense
        let span = &source_map["users[0].name"];
        assert!(span.line > 1); // not on the first (empty) line
    }

    #[test]
    fn test_source_span_line_col() {
        let toml = "name = \"hello\"\nage = 42\n";
        let (_, source_map) = toml_to_json(toml).unwrap();

        let name_span = &source_map["name"];
        assert_eq!(name_span.line, 1);

        let age_span = &source_map["age"];
        assert_eq!(age_span.line, 2);
    }

    #[test]
    fn test_invalid_toml() {
        let result = toml_to_json("= invalid");
        assert!(result.is_err());
    }
}
