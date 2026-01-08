use serde_json::Value;

/// A parsed JSONPath expression
#[derive(Debug, Clone, PartialEq)]
pub struct JsonPath {
    segments: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq)]
enum Segment {
    /// Access a field by name: `.foo`
    Field(String),
    /// Access all array elements: `[*]`
    Wildcard,
    /// Access a specific index: `[0]`
    Index(usize),
}

impl JsonPath {
    /// Parse a JSONPath string like "nodes[*].name"
    pub fn parse(path: &str) -> Result<Self, JsonPathError> {
        let mut segments = Vec::new();
        let mut chars = path.chars().peekable();

        while let Some(&c) = chars.peek() {
            match c {
                '.' => {
                    chars.next();
                    let field = Self::parse_field(&mut chars)?;
                    segments.push(Segment::Field(field));
                }
                '[' => {
                    chars.next();
                    let segment = Self::parse_bracket(&mut chars)?;
                    segments.push(segment);
                }
                _ if segments.is_empty() => {
                    // First segment without leading dot
                    let field = Self::parse_field(&mut chars)?;
                    segments.push(Segment::Field(field));
                }
                _ => return Err(JsonPathError::UnexpectedChar(c)),
            }
        }

        Ok(Self { segments })
    }

    fn parse_field(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<String, JsonPathError> {
        let mut field = String::new();
        while let Some(&c) = chars.peek() {
            if c == '.' || c == '[' {
                break;
            }
            field.push(c);
            chars.next();
        }
        if field.is_empty() {
            return Err(JsonPathError::EmptyField);
        }
        Ok(field)
    }

    fn parse_bracket(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<Segment, JsonPathError> {
        let mut content = String::new();
        while let Some(&c) = chars.peek() {
            if c == ']' {
                chars.next();
                break;
            }
            content.push(c);
            chars.next();
        }

        if content == "*" {
            Ok(Segment::Wildcard)
        } else if let Ok(index) = content.parse::<usize>() {
            Ok(Segment::Index(index))
        } else {
            Err(JsonPathError::InvalidBracket(content))
        }
    }

    /// Evaluate this path against a JSON value, returning all matching values
    pub fn evaluate<'a>(&self, value: &'a Value) -> Vec<PathMatch<'a>> {
        let mut results = vec![PathMatch {
            value,
            path: String::new(),
        }];

        for segment in &self.segments {
            let mut next_results = Vec::new();

            for current in results {
                match segment {
                    Segment::Field(name) => {
                        if let Some(v) = current.value.get(name) {
                            next_results.push(PathMatch {
                                value: v,
                                path: if current.path.is_empty() {
                                    name.clone()
                                } else {
                                    format!("{}.{}", current.path, name)
                                },
                            });
                        }
                    }
                    Segment::Wildcard => {
                        if let Some(arr) = current.value.as_array() {
                            for (i, v) in arr.iter().enumerate() {
                                next_results.push(PathMatch {
                                    value: v,
                                    path: format!("{}[{}]", current.path, i),
                                });
                            }
                        }
                    }
                    Segment::Index(i) => {
                        if let Some(v) = current.value.get(i) {
                            next_results.push(PathMatch {
                                value: v,
                                path: format!("{}[{}]", current.path, i),
                            });
                        }
                    }
                }
            }

            results = next_results;
        }

        results
    }
}

/// A matched value with its path
#[derive(Debug, Clone)]
pub struct PathMatch<'a> {
    pub value: &'a Value,
    pub path: String,
}

#[derive(Debug, thiserror::Error)]
pub enum JsonPathError {
    #[error("unexpected character: {0}")]
    UnexpectedChar(char),
    #[error("empty field name")]
    EmptyField,
    #[error("invalid bracket content: {0}")]
    InvalidBracket(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_simple() {
        let path = JsonPath::parse("nodes").unwrap();
        assert_eq!(path.segments, vec![Segment::Field("nodes".into())]);
    }

    #[test]
    fn test_parse_nested() {
        let path = JsonPath::parse("nodes[*].name").unwrap();
        assert_eq!(
            path.segments,
            vec![
                Segment::Field("nodes".into()),
                Segment::Wildcard,
                Segment::Field("name".into()),
            ]
        );
    }

    #[test]
    fn test_evaluate_wildcard() {
        let path = JsonPath::parse("nodes[*].name").unwrap();
        let data = json!({
            "nodes": [
                {"name": "foo"},
                {"name": "bar"}
            ]
        });

        let matches = path.evaluate(&data);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].value, "foo");
        assert_eq!(matches[1].value, "bar");
    }
}
