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

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
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

use indexmap::IndexMap;

/// Request body representation
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
    /// Anchored relative file path for binary uploads
    Binary {
        file_path: String,
        #[serde(default)]
        content_type: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultipartPart {
    pub name: String,
    pub value: Option<String>,
    /// Relative path to workspace file
    pub file_path: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
}

pub type MultipartField = MultipartPart;

/// Authentication type with cascading inheritance
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
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
    #[serde(alias = "aws_v4")]
    AwsSigV4 {
        access_key: String,
        secret_key: String,
        region: String,
        service: String,
        #[serde(default)]
        session_token: Option<String>,
    },
    OAuth2 {
        grant_type: String,
        access_token: String,
        #[serde(default)]
        token_url: Option<String>,
        #[serde(default)]
        client_id: Option<String>,
        #[serde(default)]
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

impl Default for AuraRequest {
    fn default() -> Self {
        Self::new("New Request", HttpMethod::GET, "")
    }
}

// =============================================================================
// File / Manifest Models (Compatible with .aura lossless disk format)
// =============================================================================

fn default_max_redirects_u32() -> u32 {
    5
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub inherit_auth: bool,
    pub verify_ssl: bool,
    pub follow_redirects: bool,
    #[serde(default = "default_max_redirects_u32")]
    pub max_redirects: u32,
    pub timeout_ms: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            inherit_auth: true,
            verify_ssl: true,
            follow_redirects: true,
            max_redirects: 5,
            timeout_ms: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowControl {
    pub once_per_run: bool,
    pub retry_until: Option<String>,
    pub retry_interval_ms: u64,
    pub max_retries: u32,
    pub delay_ms: Option<u64>,
    pub jitter_min_ms: Option<u64>,
    pub jitter_max_ms: Option<u64>,
}

impl Default for FlowControl {
    fn default() -> Self {
        Self {
            once_per_run: false,
            retry_until: None,
            retry_interval_ms: 1000,
            max_retries: 3,
            delay_ms: None,
            jitter_min_ms: None,
            jitter_max_ms: None,
        }
    }
}

impl FlowControl {
    pub fn calculate_pacing_delay(&self) -> Option<std::time::Duration> {
        let mut total_ms = self.delay_ms.unwrap_or(0);
        if let (Some(min), Some(max)) = (self.jitter_min_ms, self.jitter_max_ms) {
            if max >= min {
                let jitter = if max == min {
                    min
                } else {
                    use rand::Rng;
                    rand::thread_rng().gen_range(min..=max)
                };
                total_ms += jitter;
            }
        }
        if total_ms > 0 {
            Some(std::time::Duration::from_millis(total_ms))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ScriptHooks {
    pub pre_request: Option<String>,
    pub post_response: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuraFileRequest {
    pub version: String,
    pub name: String,
    pub method: String,
    pub url: String,
    pub settings: Settings,
    pub headers: IndexMap<String, String>,
    pub auth: Option<AuthConfig>,
    pub query_params: IndexMap<String, String>,
    #[serde(default)]
    pub variables: IndexMap<String, String>,
    pub flow: Option<FlowControl>,
    pub body: BodyType,
    pub scripts: ScriptHooks,
    #[serde(default)]
    pub assert: Option<String>,
    #[serde(default)]
    pub tests: Option<String>,
}

impl Default for AuraFileRequest {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            name: "New Request".to_string(),
            method: "GET".to_string(),
            url: "".to_string(),
            settings: Settings::default(),
            headers: IndexMap::new(),
            auth: None,
            query_params: IndexMap::new(),
            variables: IndexMap::new(),
            flow: None,
            body: BodyType::None,
            scripts: ScriptHooks::default(),
            assert: None,
            tests: None,
        }
    }
}

impl From<AuraFileRequest> for AuraRequest {
    fn from(r: AuraFileRequest) -> Self {
        let method = match r.method.to_uppercase().as_str() {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "PATCH" => HttpMethod::PATCH,
            "DELETE" => HttpMethod::DELETE,
            "HEAD" => HttpMethod::HEAD,
            "OPTIONS" => HttpMethod::OPTIONS,
            _ => HttpMethod::GET,
        };

        let mut headers = Vec::new();
        for (k, v) in r.headers {
            let (key, enabled) = if k.ends_with('~') || k.starts_with('~') {
                (k.trim_matches('~').to_string(), false)
            } else {
                (k, true)
            };
            headers.push(HeaderItem {
                key,
                value: v,
                enabled,
                description: None,
            });
        }

        let mut params = Vec::new();
        for (k, v) in r.query_params {
            let (key, enabled) = if k.ends_with('~') || k.starts_with('~') {
                (k.trim_matches('~').to_string(), false)
            } else {
                (k, true)
            };
            params.push(ParamItem {
                key,
                value: v,
                enabled,
                description: None,
            });
        }

        let mut variables = HashMap::new();
        for (k, v) in r.variables {
            variables.insert(k, v);
        }

        let auth = r.auth.unwrap_or(AuthConfig::Inherit);

        let scripts = ScriptConfig {
            pre_request: r.scripts.pre_request.unwrap_or_default(),
            post_response: r.scripts.post_response.unwrap_or_default(),
            tests: r.tests.or(r.assert).unwrap_or_default(),
        };

        let settings = RequestSettings {
            inherit_auth: r.settings.inherit_auth,
            verify_ssl: r.settings.verify_ssl,
            follow_redirects: r.settings.follow_redirects,
            max_redirects: r.settings.max_redirects as usize,
            timeout_ms: r.settings.timeout_ms,
        };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: r.name,
            method,
            url: r.url,
            params,
            headers,
            variables,
            body: r.body,
            auth,
            scripts,
            settings,
            description: None,
        }
    }
}

impl From<&AuraFileRequest> for AuraRequest {
    fn from(r: &AuraFileRequest) -> Self {
        r.clone().into()
    }
}

impl From<AuraRequest> for AuraFileRequest {
    fn from(req: AuraRequest) -> Self {
        let mut headers = IndexMap::new();
        for h in req.headers {
            let key = if h.enabled {
                h.key
            } else {
                format!("{}~", h.key)
            };
            headers.insert(key, h.value);
        }

        let mut query_params = IndexMap::new();
        for p in req.params {
            let key = if p.enabled {
                p.key
            } else {
                format!("{}~", p.key)
            };
            query_params.insert(key, p.value);
        }

        let mut variables = IndexMap::new();
        for (k, v) in req.variables {
            variables.insert(k, v);
        }

        let auth = match req.auth {
            AuthConfig::Inherit => None,
            AuthConfig::None => Some(AuthConfig::None),
            other => Some(other),
        };

        AuraFileRequest {
            version: "1.0".to_string(),
            name: req.name,
            method: req.method.as_str().to_string(),
            url: req.url,
            settings: Settings {
                inherit_auth: req.settings.inherit_auth,
                verify_ssl: req.settings.verify_ssl,
                follow_redirects: req.settings.follow_redirects,
                max_redirects: req.settings.max_redirects as u32,
                timeout_ms: req.settings.timeout_ms,
            },
            headers,
            auth,
            query_params,
            variables,
            flow: None,
            body: req.body,
            scripts: ScriptHooks {
                pre_request: if req.scripts.pre_request.is_empty() {
                    None
                } else {
                    Some(req.scripts.pre_request)
                },
                post_response: if req.scripts.post_response.is_empty() {
                    None
                } else {
                    Some(req.scripts.post_response)
                },
            },
            assert: None,
            tests: if req.scripts.tests.is_empty() {
                None
            } else {
                Some(req.scripts.tests)
            },
        }
    }
}

impl From<&AuraRequest> for AuraFileRequest {
    fn from(req: &AuraRequest) -> Self {
        req.clone().into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    pub name: String,
    pub variables: IndexMap<String, VariableValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariableValue {
    pub value: String,
    pub is_secret: bool,
    pub keyring_ref: Option<String>,
}

impl VariableValue {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            is_secret: false,
            keyring_ref: None,
        }
    }

    pub fn secret(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            is_secret: true,
            keyring_ref: None,
        }
    }

    pub fn from_keyring(keyring_ref: impl Into<String>) -> Self {
        Self {
            value: String::new(),
            is_secret: true,
            keyring_ref: Some(keyring_ref.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult {
    pub name: String,
    pub passed: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReport {
    pub request_name: String,
    pub method: String,
    pub url: String,
    pub status_code: u16,
    pub duration_ms: u64,
    pub response_headers: IndexMap<String, String>,
    pub response_body: String,
    pub assertions: Vec<AssertionResult>,
    pub logs: Vec<String>,
    pub error: Option<String>,
}

impl ExecutionReport {
    pub fn all_passed(&self) -> bool {
        self.error.is_none() && self.assertions.iter().all(|a| a.passed)
    }

    pub fn exit_code(&self) -> i32 {
        if self.error.is_some() {
            return 3;
        }
        if self.assertions.iter().any(|a| !a.passed) {
            1
        } else {
            0
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionMeta {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub request_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub collections: Vec<CollectionMeta>,
    #[serde(default)]
    pub active_environment_id: Option<String>,
    #[serde(default)]
    pub open_tabs: Vec<String>,
    #[serde(default)]
    pub active_tab: Option<String>,
}
