use serde_json::Value as JsonValue;

use crate::source_map::{build_line_offsets, make_span, SourceMap};

/// Error from JSON parsing
#[derive(Debug, thiserror::Error)]
pub enum JsonError {
    #[error("JSON parse error: {0}")]
    Parse(String),
}

/// Parse a JSON string into a serde_json Value and a source map that maps
/// JSON paths back to byte offsets (and line/column) in the JSON source.
pub fn json_with_source_map(json_src: &str) -> Result<(JsonValue, SourceMap), JsonError> {
    let value: JsonValue =
        serde_json::from_str(json_src).map_err(|e| JsonError::Parse(format!("{}", e)))?;

    let line_offsets = build_line_offsets(json_src);
    let mut source_map = SourceMap::new();

    map_value(&value, "", json_src, 0, &line_offsets, &mut source_map);

    Ok((value, source_map))
}

/// Walk the parsed JSON value and the source string in parallel to find
/// the byte offset of each value in the source.
fn map_value(
    value: &JsonValue,
    path: &str,
    src: &str,
    start_from: usize,
    line_offsets: &[usize],
    source_map: &mut SourceMap,
) -> usize {
    let bytes = src.as_bytes();
    let mut pos = skip_whitespace(bytes, start_from);

    match value {
        JsonValue::Object(map) => {
            // Record span for the object itself
            let obj_start = pos;
            if !path.is_empty() {
                // We'll set the end after we find closing brace
            }
            pos += 1; // skip '{'
            pos = skip_whitespace(bytes, pos);

            let mut first = true;
            // We need to iterate in the order they appear in source, not map order.
            // Parse keys from source to match ordering.
            while pos < bytes.len() && bytes[pos] != b'}' {
                if !first {
                    pos = expect_byte(bytes, pos, b',');
                    pos = skip_whitespace(bytes, pos);
                }
                first = false;

                // Parse key string from source
                let (key, key_end) = parse_string_raw(bytes, pos);
                pos = skip_whitespace(bytes, key_end);
                pos = expect_byte(bytes, pos, b':');
                pos = skip_whitespace(bytes, pos);

                // Find this key's value in the parsed map
                if let Some(child_value) = map.get(&key) {
                    let child_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };

                    let value_start = pos;
                    pos = map_value(child_value, &child_path, src, pos, line_offsets, source_map);

                    // Record span for this value
                    if !child_path.is_empty() {
                        source_map.insert(
                            child_path,
                            make_span(value_start, pos, line_offsets),
                        );
                    }
                } else {
                    // Key not in parsed map (shouldn't happen), skip the value
                    pos = skip_json_value(bytes, pos);
                }

                pos = skip_whitespace(bytes, pos);
            }

            if pos < bytes.len() {
                pos += 1; // skip '}'
            }

            if !path.is_empty() {
                source_map.insert(
                    path.to_string(),
                    make_span(obj_start, pos, line_offsets),
                );
            }

            pos
        }
        JsonValue::Array(arr) => {
            let arr_start = pos;
            pos += 1; // skip '['
            pos = skip_whitespace(bytes, pos);

            for (i, child_value) in arr.iter().enumerate() {
                if i > 0 {
                    pos = expect_byte(bytes, pos, b',');
                    pos = skip_whitespace(bytes, pos);
                }

                let child_path = format!("{}[{}]", path, i);
                let value_start = pos;
                pos = map_value(child_value, &child_path, src, pos, line_offsets, source_map);

                source_map.insert(
                    child_path,
                    make_span(value_start, pos, line_offsets),
                );

                pos = skip_whitespace(bytes, pos);
            }

            if pos < bytes.len() {
                pos += 1; // skip ']'
            }

            if !path.is_empty() {
                source_map.insert(
                    path.to_string(),
                    make_span(arr_start, pos, line_offsets),
                );
            }

            pos
        }
        JsonValue::String(_) => {
            // Skip the string literal in source
            let (_, end) = parse_string_raw(bytes, pos);
            end
        }
        JsonValue::Number(_) => skip_number(bytes, pos),
        JsonValue::Bool(b) => {
            if *b {
                pos + 4 // "true"
            } else {
                pos + 5 // "false"
            }
        }
        JsonValue::Null => pos + 4, // "null"
    }
}

fn skip_whitespace(bytes: &[u8], mut pos: usize) -> usize {
    while pos < bytes.len() && matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r') {
        pos += 1;
    }
    pos
}

fn expect_byte(bytes: &[u8], pos: usize, expected: u8) -> usize {
    if pos < bytes.len() && bytes[pos] == expected {
        pos + 1
    } else {
        pos
    }
}

/// Parse a JSON string starting at `pos` (which should point to the opening `"`).
/// Returns (the string content, byte position after the closing `"`).
fn parse_string_raw(bytes: &[u8], pos: usize) -> (String, usize) {
    let mut i = pos + 1; // skip opening "
    let mut s = String::new();
    while i < bytes.len() {
        match bytes[i] {
            b'"' => return (s, i + 1),
            b'\\' => {
                i += 1;
                if i < bytes.len() {
                    match bytes[i] {
                        b'"' => s.push('"'),
                        b'\\' => s.push('\\'),
                        b'/' => s.push('/'),
                        b'n' => s.push('\n'),
                        b'r' => s.push('\r'),
                        b't' => s.push('\t'),
                        b'b' => s.push('\u{0008}'),
                        b'f' => s.push('\u{000C}'),
                        b'u' => {
                            // Unicode escape: \uXXXX
                            if i + 4 < bytes.len() {
                                if let Ok(hex_str) = std::str::from_utf8(&bytes[i + 1..i + 5]) {
                                    if let Ok(code) = u32::from_str_radix(hex_str, 16) {
                                        if let Some(c) = char::from_u32(code) {
                                            s.push(c);
                                        }
                                    }
                                }
                                i += 4;
                            }
                        }
                        c => {
                            s.push('\\');
                            s.push(c as char);
                        }
                    }
                }
            }
            c => s.push(c as char),
        }
        i += 1;
    }
    (s, i)
}

fn skip_number(bytes: &[u8], mut pos: usize) -> usize {
    if pos < bytes.len() && bytes[pos] == b'-' {
        pos += 1;
    }
    while pos < bytes.len() && matches!(bytes[pos], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
    {
        pos += 1;
    }
    pos
}

/// Skip an entire JSON value (for keys we don't care about).
fn skip_json_value(bytes: &[u8], pos: usize) -> usize {
    let pos = skip_whitespace(bytes, pos);
    if pos >= bytes.len() {
        return pos;
    }
    match bytes[pos] {
        b'"' => parse_string_raw(bytes, pos).1,
        b'{' => skip_balanced(bytes, pos, b'{', b'}'),
        b'[' => skip_balanced(bytes, pos, b'[', b']'),
        b't' => pos + 4,
        b'f' => pos + 5,
        b'n' => pos + 4,
        _ => skip_number(bytes, pos),
    }
}

fn skip_balanced(bytes: &[u8], mut pos: usize, open: u8, close: u8) -> usize {
    let mut depth = 1;
    pos += 1;
    while pos < bytes.len() && depth > 0 {
        match bytes[pos] {
            b'"' => {
                pos = parse_string_raw(bytes, pos).1;
                continue;
            }
            b if b == open => depth += 1,
            b if b == close => depth -= 1,
            _ => {}
        }
        pos += 1;
    }
    pos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_json_source_map() {
        let json = r#"{"name": "alice", "age": 30}"#;
        let (value, source_map) = json_with_source_map(json).unwrap();

        assert_eq!(value["name"], "alice");
        assert_eq!(value["age"], 30);

        assert!(source_map.contains_key("name"));
        assert!(source_map.contains_key("age"));
    }

    #[test]
    fn test_nested_json_source_map() {
        let json = r#"{
  "users": [
    {"name": "alice"},
    {"name": "bob"}
  ]
}"#;
        let (value, source_map) = json_with_source_map(json).unwrap();

        assert_eq!(value["users"][0]["name"], "alice");
        assert_eq!(value["users"][1]["name"], "bob");

        assert!(source_map.contains_key("users[0].name"));
        assert!(source_map.contains_key("users[1].name"));

        let span = &source_map["users[0].name"];
        assert_eq!(span.line, 3);

        let span = &source_map["users[1].name"];
        assert_eq!(span.line, 4);
    }

    #[test]
    fn test_json_line_col() {
        let json = "{\n  \"name\": \"hello\",\n  \"age\": 42\n}";
        let (_, source_map) = json_with_source_map(json).unwrap();

        let name_span = &source_map["name"];
        assert_eq!(name_span.line, 2);

        let age_span = &source_map["age"];
        assert_eq!(age_span.line, 3);
    }

    #[test]
    fn test_invalid_json() {
        let result = json_with_source_map("{invalid}");
        assert!(result.is_err());
    }

    #[test]
    fn test_escaped_strings() {
        let json = r#"{"key": "hello \"world\""}"#;
        let (value, source_map) = json_with_source_map(json).unwrap();

        assert_eq!(value["key"], "hello \"world\"");
        assert!(source_map.contains_key("key"));
    }

    #[test]
    fn test_array_of_objects() {
        let json = r#"{
  "nodes": [{"id": "a"}, {"id": "b"}],
  "edges": [{"from": "a", "to": "b"}]
}"#;
        let (_, source_map) = json_with_source_map(json).unwrap();

        assert!(source_map.contains_key("nodes[0].id"));
        assert!(source_map.contains_key("nodes[1].id"));
        assert!(source_map.contains_key("edges[0].from"));
        assert!(source_map.contains_key("edges[0].to"));
    }
}
