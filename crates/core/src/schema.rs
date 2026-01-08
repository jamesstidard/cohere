use serde::Deserialize;
use serde_json::Value;

use crate::JsonPath;

/// A parsed schema with custom validation rules
#[derive(Debug)]
pub struct Schema {
    /// The raw JSON Schema (for potential future standard validation)
    pub raw: Value,
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

        Ok(Self {
            raw: value,
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
}

#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("invalid {0} rule: {1}")]
    InvalidRule(&'static str, String),
    #[error("invalid path '{0}': {1}")]
    InvalidPath(String, String),
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
