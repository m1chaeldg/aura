//! HTTP method, parameter, header, and request body types.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// HTTP Method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    HEAD,
    OPTIONS,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GET => "GET",
            Self::POST => "POST",
            Self::PUT => "PUT",
            Self::PATCH => "PATCH",
            Self::DELETE => "DELETE",
            Self::HEAD => "HEAD",
            Self::OPTIONS => "OPTIONS",
        }
    }

    /// Color hex code for badge according to DESIGN.md.
    pub fn badge_color(&self) -> &'static str {
        match self {
            Self::GET => crate::design::colors::METHOD_GET,
            Self::POST => crate::design::colors::METHOD_POST,
            Self::PUT => crate::design::colors::METHOD_PUT,
            Self::PATCH => crate::design::colors::METHOD_PATCH,
            Self::DELETE => crate::design::colors::METHOD_DELETE,
            Self::HEAD => crate::design::colors::BODY_MUTED,
            Self::OPTIONS => crate::design::colors::PRIMARY_ON_DARK,
        }
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Query parameter item with enabled flag.
/// In dual-mode TOML, disabled parameters serialize with trailing tilde: `key~ = "value"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamItem {
    pub key: String,
    pub value: String,
    pub enabled: bool,
    #[serde(default)]
    pub description: Option<String>,
}

impl ParamItem {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            enabled: true,
            description: None,
        }
    }

    pub fn disabled(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            enabled: false,
            description: None,
        }
    }
}

/// HTTP Header item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeaderItem {
    pub key: String,
    pub value: String,
    pub enabled: bool,
    #[serde(default)]
    pub description: Option<String>,
}

impl HeaderItem {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            enabled: true,
            description: None,
        }
    }

    pub fn disabled(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            enabled: false,
            description: None,
        }
    }
}

/// Request body representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", content = "content")]
pub enum BodyType {
    #[default]
    None,
    Json(String),
    Xml(String),
    Text(String),
    Raw {
        content: String,
        #[serde(default)]
        content_type: Option<String>,
    },
    Form(IndexMap<String, String>),
    FormUrlEncoded(Vec<ParamItem>),
    Multipart(Vec<MultipartPart>),
    /// Anchored relative file path for binary uploads.
    Binary {
        file_path: String,
        #[serde(default)]
        content_type: Option<String>,
    },
    GraphQL {
        query: String,
        #[serde(default)]
        variables: String,
    },
}

impl BodyType {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Json(_) => "JSON",
            Self::Xml(_) => "XML",
            Self::Text(_) => "Text",
            Self::Raw { .. } => "Raw",
            Self::Form(_) => "Form",
            Self::FormUrlEncoded(_) => "FormUrlEncoded",
            Self::Multipart(_) => "Multipart",
            Self::Binary { .. } => "Binary",
            Self::GraphQL { .. } => "GraphQL",
        }
    }

    pub fn content_str(&self) -> &str {
        match self {
            Self::Json(s) | Self::Xml(s) | Self::Text(s) => s.as_str(),
            Self::Raw { content, .. } => content.as_str(),
            Self::GraphQL { query, .. } => query.as_str(),
            _ => "",
        }
    }

    pub fn default_for_type(name: &str) -> Self {
        match name.to_uppercase().as_str() {
            "NONE" => Self::None,
            "JSON" => Self::Json("{\n  \"key\": \"value\"\n}".to_string()),
            "XML" => Self::Xml(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<root>\n  <item>value</item>\n</root>"
                    .to_string(),
            ),
            "TEXT" => Self::Text("".to_string()),
            "RAW" => Self::Raw {
                content: "".to_string(),
                content_type: Some("text/plain".to_string()),
            },
            "FORM" => Self::Form(IndexMap::new()),
            "FORMURLENCODED" | "FORM_URLENCODED" | "X-WWW-FORM-URLENCODED" => {
                Self::FormUrlEncoded(Vec::new())
            }
            "MULTIPART" | "MULTIPART/FORM-DATA" => Self::Multipart(Vec::new()),
            "BINARY" => Self::Binary {
                file_path: "".to_string(),
                content_type: None,
            },
            "GRAPHQL" | "GQL" => Self::GraphQL {
                query: "query {\n  \n}".to_string(),
                variables: "{}".to_string(),
            },
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultipartPart {
    pub name: String,
    pub value: Option<String>,
    /// Relative path to workspace file.
    pub file_path: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
}

pub type MultipartField = MultipartPart;
