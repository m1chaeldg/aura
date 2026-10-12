//! Domain data models for Aura API Client.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// HTTP Method
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

    /// Color hex code for badge according to DESIGN.md
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

/// Query parameter item with enabled flag.
/// In dual-mode TOML, disabled parameters serialize with trailing tilde: `key~ = "value"`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamItem {
    pub key: String,
    pub value: String,
    pub enabled: bool,
    #[serde(default)]
    pub description: Option<String>,
}

/// HTTP Header item
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeaderItem {
    pub key: String,
    pub value: String,
    pub enabled: bool,
    #[serde(default)]
    pub description: Option<String>,
}

/// Request body representation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", content = "content")]
pub enum BodyType {
    #[default]
    None,
    Json(String),
    Xml(String),
    Text(String),
    FormUrlEncoded(Vec<ParamItem>),
    Multipart(Vec<MultipartField>),
    /// Anchored relative file path for binary uploads
    Binary {
        relative_path: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultipartField {
    pub name: String,
    pub value: Option<String>,
    pub file_path: Option<String>,
    pub enabled: bool,
}

/// Authentication type with cascading inheritance
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type")]
pub enum AuthConfig {
    #[default]
    Inherit,
    None,
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    ApiKey {
        key: String,
        value: String,
        in_header: bool, // true: header, false: query
    },
    AwsV4 {
        access_key: String,
        secret_key: String,
        region: String,
        service: String,
        profile: Option<String>,
    },
    OAuth2 {
        grant_type: String,
        access_token: String,
        token_url: Option<String>,
        client_id: Option<String>,
        client_secret: Option<String>,
    },
}

/// Network and execution settings
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestSettings {
    #[serde(default = "default_true")]
    pub inherit_auth: bool,
    #[serde(default = "default_true")]
    pub verify_ssl: bool,
    #[serde(default = "default_true")]
    pub follow_redirects: bool,
    #[serde(default = "default_max_redirects")]
    pub max_redirects: usize,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_true() -> bool {
    true
}
fn default_max_redirects() -> usize {
    10
}
fn default_timeout() -> u64 {
    30_000
}

impl Default for RequestSettings {
    fn default() -> Self {
        Self {
            inherit_auth: true,
            verify_ssl: true,
            follow_redirects: true,
            max_redirects: 10,
            timeout_ms: 30_000,
        }
    }
}

/// Scripts attached to request
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptConfig {
    #[serde(default)]
    pub pre_request: String,
    #[serde(default)]
    pub post_response: String,
    #[serde(default)]
    pub tests: String,
}

/// Core Aura Request representation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuraRequest {
    pub id: String,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    #[serde(default)]
    pub params: Vec<ParamItem>,
    #[serde(default)]
    pub headers: Vec<HeaderItem>,
    #[serde(default)]
    pub variables: HashMap<String, String>,
    #[serde(default)]
    pub body: BodyType,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub scripts: ScriptConfig,
    #[serde(default)]
    pub settings: RequestSettings,
    #[serde(default)]
    pub description: Option<String>,
}

impl AuraRequest {
    pub fn new(name: impl Into<String>, method: HttpMethod, url: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            method,
            url: url.into(),
            params: Vec::new(),
            headers: Vec::new(),
            variables: HashMap::new(),
            body: BodyType::None,
            auth: AuthConfig::default(),
            scripts: ScriptConfig::default(),
            settings: RequestSettings::default(),
            description: None,
        }
    }
}

/// Test assertion result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestAssertion {
    pub name: String,
    pub passed: bool,
    #[serde(default)]
    pub message: Option<String>,
    pub duration_ms: u64,
}

/// HTTP Response from client engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuraResponse {
    pub status: u16,
    pub status_text: String,
    pub duration_ms: u64,
    pub size_bytes: usize,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub assertions: Vec<TestAssertion>,
}

impl AuraResponse {
    pub fn status_pill_color(&self) -> &'static str {
        match self.status {
            200..=299 => crate::design::colors::STATUS_2XX,
            300..=399 => crate::design::colors::STATUS_3XX,
            400..=499 => crate::design::colors::STATUS_4XX,
            _ => crate::design::colors::STATUS_5XX,
        }
    }

    pub fn formatted_size(&self) -> String {
        if self.size_bytes < 1024 {
            format!("{} B", self.size_bytes)
        } else if self.size_bytes < 1024 * 1024 {
            format!("{:.2} KB", self.size_bytes as f64 / 1024.0)
        } else {
            format!("{:.2} MB", self.size_bytes as f64 / (1024.0 * 1024.0))
        }
    }
}

/// Variable item in environment profile
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariableItem {
    pub key: String,
    pub value: String,
    pub enabled: bool,
    pub is_secret: bool,
}

/// Environment profile
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentProfile {
    pub id: String,
    pub name: String,
    pub variables: Vec<VariableItem>,
}

/// Collection Folder item
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub auth: AuthConfig,
    pub requests: Vec<AuraRequest>,
    pub subfolders: Vec<Folder>,
}

/// Collection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub auth: AuthConfig,
    pub variables: HashMap<String, String>,
    pub requests: Vec<AuraRequest>,
    pub folders: Vec<Folder>,
}

/// Workspace root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    /// Absolute or relative path to workspace directory
    pub root_path: PathBuf,
    pub collections: Vec<Collection>,
    pub environments: Vec<EnvironmentProfile>,
    pub active_environment_id: Option<String>,
}

impl Workspace {
    pub fn new(name: impl Into<String>, root_path: impl AsRef<Path>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            root_path: root_path.as_ref().to_path_buf(),
            collections: Vec::new(),
            environments: Vec::new(),
            active_environment_id: None,
        }
    }
}
