use serde::Deserialize;
use serde_json::Value;

use crate::JsonPath;

/// A parsed schema with custom validation rules
#[derive(Debug)]
pub struct Schema {
    /// The raw JSON Schema (for potential future standard validation)
    pub raw: Value,
    /// Compiled JSON Schema validator (None if schema has no standard keywords)
    pub json_schema: Option<jsonschema::Validator>,
    /// Parsed x-uniqueAcross rules
    pub unique_across: Vec<UniqueAcrossRule>,
    /// Parsed x-references rules
    pub references: Vec<ReferencesRule>,
    /// Parsed x-referencedBy rules
    pub referenced_by: Vec<ReferencedByRule>,
}

#[derive(Debug)]
pub struct UniqueAcrossRule {
    pub paths: Vec<JsonPath>,
    pub message: Option<String>,
}

#[derive(Debug)]
pub struct ReferencesRule {
    pub from: JsonPath,
    pub to: Vec<JsonPath>,
    pub message: Option<String>,
}

#[derive(Debug)]
pub struct ReferencedByRule {
    pub target: JsonPath,
    pub from: Vec<JsonPath>,
    pub min: usize,
    pub max: Option<usize>,
    pub message: Option<String>,
}

/// Intermediate structs for deserializing the x- keywords
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UniqueAcrossRaw {
    paths: Vec<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReferencesRaw {
    from: String,
    to: StringOrVec,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReferencedByRaw {
    target: String,
    from: StringOrVec,
    #[serde(default = "default_min")]
    min: usize,
    max: Option<usize>,
    message: Option<String>,
}

fn default_min() -> usize {
    1
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StringOrVec {
    Single(String),
    Multiple(Vec<String>),
}

impl StringOrVec {
    fn into_vec(self) -> Vec<String> {
        match self {
            StringOrVec::Single(s) => vec![s],
            StringOrVec::Multiple(v) => v,
        }
    }
}

impl Schema {
    /// Parse a JSON Schema with custom x- keywords
    pub fn parse(value: Value) -> Result<Self, SchemaError> {
        let unique_across = Self::parse_unique_across(&value)?;
        let references = Self::parse_references(&value)?;
        let referenced_by = Self::parse_referenced_by(&value)?;

        // Compile JSON Schema validator if schema has standard keywords
        let json_schema = Self::compile_json_schema(&value)?;

        Ok(Self {
            raw: value,
            json_schema,
            unique_across,
            references,
            referenced_by,
        })
    }

    fn parse_unique_across(value: &Value) -> Result<Vec<UniqueAcrossRule>, SchemaError> {
        let Some(raw) = value.get("x-uniqueAcross") else {
            return Ok(Vec::new());
        };

        let rules: Vec<UniqueAcrossRaw> = serde_json::from_value(raw.clone())
            .map_err(|e| SchemaError::InvalidRule("x-uniqueAcross", e.to_string()))?;

        rules
            .into_iter()
            .map(|r| {
                let paths = r
                    .paths
                    .into_iter()
                    .map(|p| JsonPath::parse(&p).map_err(|e| SchemaError::InvalidPath(p, e.to_string())))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(UniqueAcrossRule {
                    paths,
                    message: r.message,
                })
            })
            .collect()
    }

    fn parse_references(value: &Value) -> Result<Vec<ReferencesRule>, SchemaError> {
        let Some(raw) = value.get("x-references") else {
            return Ok(Vec::new());
        };

        let rules: Vec<ReferencesRaw> = serde_json::from_value(raw.clone())
            .map_err(|e| SchemaError::InvalidRule("x-references", e.to_string()))?;

        rules
            .into_iter()
            .map(|r| {
                let from = JsonPath::parse(&r.from)
                    .map_err(|e| SchemaError::InvalidPath(r.from.clone(), e.to_string()))?;

                let to = r
                    .to
                    .into_vec()
                    .into_iter()
                    .map(|p| JsonPath::parse(&p).map_err(|e| SchemaError::InvalidPath(p, e.to_string())))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(ReferencesRule {
                    from,
                    to,
                    message: r.message,
                })
            })
            .collect()
    }

    fn parse_referenced_by(value: &Value) -> Result<Vec<ReferencedByRule>, SchemaError> {
        let Some(raw) = value.get("x-referencedBy") else {
            return Ok(Vec::new());
        };

        let rules: Vec<ReferencedByRaw> = serde_json::from_value(raw.clone())
            .map_err(|e| SchemaError::InvalidRule("x-referencedBy", e.to_string()))?;

        rules
            .into_iter()
            .map(|r| {
                let target = JsonPath::parse(&r.target)
                    .map_err(|e| SchemaError::InvalidPath(r.target.clone(), e.to_string()))?;

                let from = r
                    .from
                    .into_vec()
                    .into_iter()
                    .map(|p| JsonPath::parse(&p).map_err(|e| SchemaError::InvalidPath(p, e.to_string())))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(ReferencedByRule {
                    target,
                    from,
                    min: r.min,
                    max: r.max,
                    message: r.message,
                })
            })
            .collect()
    }

    /// Compile JSON Schema validator if schema has standard keywords
    fn compile_json_schema(value: &Value) -> Result<Option<jsonschema::Validator>, SchemaError> {
        // Check if schema has any standard JSON Schema keywords
        if !Self::has_standard_keywords(value) {
            return Ok(None);
        }

        // Use validator_for for auto-draft detection
        match jsonschema::validator_for(value) {
            Ok(compiled) => Ok(Some(compiled)),
            Err(e) => Err(SchemaError::JsonSchemaCompilation(e.to_string())),
        }
    }

    /// Check if schema has any standard JSON Schema keywords (non x- prefixed)
    fn has_standard_keywords(value: &Value) -> bool {
        if let Some(obj) = value.as_object() {
            obj.keys().any(|k| !k.starts_with("x-"))
        } else {
            false
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("invalid {0} rule: {1}")]
    InvalidRule(&'static str, String),
    #[error("invalid path '{0}': {1}")]
    InvalidPath(String, String),
    #[error("JSON Schema compilation failed: {0}")]
    JsonSchemaCompilation(String),
}

/// Enum for downstream code to work with any rule type
#[derive(Debug)]
pub enum ValidationRule<'a> {
    UniqueAcross(&'a UniqueAcrossRule),
    References(&'a ReferencesRule),
    ReferencedBy(&'a ReferencedByRule),
}

impl Schema {
    /// Iterate over all validation rules
    pub fn rules(&self) -> impl Iterator<Item = ValidationRule<'_>> {
        self.unique_across
            .iter()
            .map(ValidationRule::UniqueAcross)
            .chain(self.references.iter().map(ValidationRule::References))
            .chain(self.referenced_by.iter().map(ValidationRule::ReferencedBy))
    }
}
