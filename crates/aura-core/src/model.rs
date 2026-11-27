//! Domain data models for Aura API Client.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Request transport protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RequestProtocol {
    #[default]
    Http,
    Graphql,
    Websocket,
    Sse,
    Grpc,
    Soap,
}

impl RequestProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Http => "HTTP",
            Self::Graphql => "GraphQL",
            Self::Websocket => "WS",
            Self::Sse => "SSE",
            Self::Grpc => "gRPC",
            Self::Soap => "SOAP",
        }
    }

    pub fn from_str_lenient(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "graphql" | "gql" => Self::Graphql,
            "ws" | "wss" | "websocket" => Self::Websocket,
            "sse" | "event-stream" => Self::Sse,
            "grpc" | "grpcs" => Self::Grpc,
            "soap" | "wsdl" => Self::Soap,
            _ => Self::Http,
        }
    }
}

/// Saved WebSocket message template attached to a request
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WsMessageTemplate {
    pub id: String,
    pub name: String,
    pub payload: String,
}

/// gRPC call configuration attached to a request
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GrpcConfig {
    #[serde(default)]
    pub proto_path: Option<String>,
    #[serde(default)]
    pub proto_content: Option<String>,
    #[serde(default)]
    pub service: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub use_reflection: bool,
    #[serde(default)]
    pub payload: String,
}

/// Parsed Server-Sent Event item
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SseEvent {
    #[serde(default)]
    pub id: Option<String>,
    pub event: String,
    pub data: String,
    #[serde(default)]
    pub retry_ms: Option<u64>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Unified live streaming message (WebSocket or gRPC stream)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamMessage {
    pub direction: String, // "in", "out", "system"
    pub payload: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

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

/// HTTP Header item
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
            "FORM" => Self::Form(indexmap::IndexMap::new()),
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
        #[serde(default)]
        use_profile: bool,
        #[serde(default)]
        profile: String,
        #[serde(default)]
        credentials_path: Option<String>,
        #[serde(default)]
        access_key: String,
        #[serde(default)]
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
        #[serde(default)]
        auth_url: Option<String>,
        #[serde(default)]
        redirect_uri: Option<String>,
        #[serde(default)]
        scope: Option<String>,
        #[serde(default)]
        username: Option<String>,
        #[serde(default)]
        password: Option<String>,
        #[serde(default)]
        refresh_token: Option<String>,
        #[serde(default)]
        expires_at: Option<i64>,
        #[serde(default)]
        use_pkce: bool,
        #[serde(default)]
        auto_refresh: bool,
    },
    Digest {
        username: String,
        password: String,
        #[serde(default)]
        realm: Option<String>,
        #[serde(default)]
        nonce: Option<String>,
        #[serde(default)]
        qop: Option<String>,
        #[serde(default)]
        algorithm: Option<String>,
    },
    Ntlm {
        username: String,
        password: String,
        #[serde(default)]
        domain: String,
        #[serde(default)]
        workstation: String,
    },
    #[serde(alias = "oauth1")]
    OAuth1 {
        consumer_key: String,
        consumer_secret: String,
        #[serde(default)]
        token: String,
        #[serde(default)]
        token_secret: String,
        #[serde(default)]
        signature_method: String,
        #[serde(default)]
        callback_url: Option<String>,
        #[serde(default)]
        realm: Option<String>,
        #[serde(default)]
        add_to_query: bool,
    },
    Hawk {
        id: String,
        key: String,
        #[serde(default)]
        algorithm: String,
        #[serde(default)]
        ext: Option<String>,
        #[serde(default)]
        app: Option<String>,
        #[serde(default)]
        dlg: Option<String>,
    },
    #[serde(alias = "edgegrid")]
    AkamaiEdgeGrid {
        client_token: String,
        client_secret: String,
        access_token: String,
        #[serde(default)]
        max_body: usize,
    },
    Wsse {
        username: String,
        password: String,
        #[serde(default)]
        digest_password: bool,
    },
}

impl AuthConfig {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Inherit => "Inherit",
            Self::None => "None",
            Self::Bearer { .. } => "Bearer Token",
            Self::Basic { .. } => "Basic Auth",
            Self::ApiKey { .. } => "API Key",
            Self::AwsSigV4 { .. } => "AWS SigV4",
            Self::OAuth2 { .. } => "OAuth 2.0",
            Self::Digest { .. } => "Digest Auth",
            Self::Ntlm { .. } => "NTLM",
            Self::OAuth1 { .. } => "OAuth 1.0",
            Self::Hawk { .. } => "Hawk",
            Self::AkamaiEdgeGrid { .. } => "Akamai EdgeGrid",
            Self::Wsse { .. } => "WSSE",
        }
    }

    pub fn default_for_type(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "inherit" => Self::Inherit,
            "none" => Self::None,
            "bearer" | "bearer token" => Self::Bearer {
                token: "".to_string(),
            },
            "basic" | "basic auth" => Self::Basic {
                username: "".to_string(),
                password: "".to_string(),
            },
            "api_key" | "apikey" | "api key" => Self::ApiKey {
                key: "X-API-Key".to_string(),
                value: "".to_string(),
                in_header: true,
            },
            "aws" | "aws_sigv4" | "awssigv4" | "aws sigv4" => Self::AwsSigV4 {
                use_profile: true,
                profile: "default".to_string(),
                credentials_path: None,
                access_key: "".to_string(),
                secret_key: "".to_string(),
                region: "us-east-1".to_string(),
                service: "execute-api".to_string(),
                session_token: None,
            },
            "oauth2" | "oauth 2.0" => Self::OAuth2 {
                grant_type: "authorization_code".to_string(),
                access_token: "".to_string(),
                token_url: None,
                client_id: None,
                client_secret: None,
                auth_url: None,
                redirect_uri: Some("http://127.0.0.1:8765/callback".to_string()),
                scope: None,
                username: None,
                password: None,
                refresh_token: None,
                expires_at: None,
                use_pkce: true,
                auto_refresh: true,
            },
            "digest" | "digest auth" => Self::Digest {
                username: "".to_string(),
                password: "".to_string(),
                realm: None,
                nonce: None,
                qop: Some("auth".to_string()),
                algorithm: Some("MD5".to_string()),
            },
            "ntlm" => Self::Ntlm {
                username: "".to_string(),
                password: "".to_string(),
                domain: "".to_string(),
                workstation: "AURA-WORKSTATION".to_string(),
            },
            "oauth1" | "oauth 1.0" => Self::OAuth1 {
                consumer_key: "".to_string(),
                consumer_secret: "".to_string(),
                token: "".to_string(),
                token_secret: "".to_string(),
                signature_method: "HMAC-SHA1".to_string(),
                callback_url: None,
                realm: None,
                add_to_query: false,
            },
            "hawk" => Self::Hawk {
                id: "".to_string(),
                key: "".to_string(),
                algorithm: "sha256".to_string(),
                ext: None,
                app: None,
                dlg: None,
            },
            "akamai_edgegrid" | "akamai edgegrid" | "edgegrid" => Self::AkamaiEdgeGrid {
                client_token: "".to_string(),
                client_secret: "".to_string(),
                access_token: "".to_string(),
                max_body: 131072,
            },
            "wsse" => Self::Wsse {
                username: "".to_string(),
                password: "".to_string(),
                digest_password: true,
            },
            _ => Self::None,
        }
    }
}

/// Client certificate configuration for mTLS (.pem or .pfx/.p12)
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ClientCertificateConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub cert_path: String,
    #[serde(default)]
    pub key_path: Option<String>,
    #[serde(default)]
    pub passphrase: Option<String>,
}

/// Supported outbound proxy protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ProxyProtocol {
    #[default]
    Http,
    Https,
    Socks5,
    Socks5h,
}

impl ProxyProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Http => "HTTP",
            Self::Https => "HTTPS",
            Self::Socks5 => "SOCKS5",
            Self::Socks5h => "SOCKS5H",
        }
    }

    pub fn scheme(&self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
            Self::Socks5 => "socks5",
            Self::Socks5h => "socks5h",
        }
    }
}

/// Global or per-request proxy configuration (`http://`, `https://`, `socks5://`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub protocol: ProxyProtocol,
    #[serde(default)]
    pub host: String,
    #[serde(default = "default_proxy_port")]
    pub port: u16,
    #[serde(default)]
    pub auth_enabled: bool,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub bypass_list: Vec<String>,
}

fn default_proxy_port() -> u16 {
    8080
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            protocol: ProxyProtocol::Http,
            host: String::new(),
            port: 8080,
            auth_enabled: false,
            username: String::new(),
            password: String::new(),
            bypass_list: vec!["localhost".to_string(), "127.0.0.1".to_string()],
        }
    }
}

impl ProxyConfig {
    pub fn effective_url(&self) -> Option<String> {
        let host = self.host.trim();
        if !self.enabled || host.is_empty() {
            return None;
        }
        let scheme = self.protocol.scheme();
        if self.auth_enabled && !self.username.is_empty() {
            Some(format!(
                "{scheme}://{}:{}@{host}:{}",
                self.username, self.password, self.port
            ))
        } else {
            Some(format!("{scheme}://{host}:{}", self.port))
        }
    }

    pub fn should_bypass(&self, target_url: &str) -> bool {
        let Ok(parsed) = url::Url::parse(target_url) else {
            return false;
        };
        let Some(host) = parsed.host_str().map(|h| h.to_ascii_lowercase()) else {
            return false;
        };
        let host_port = match parsed.port() {
            Some(p) => format!("{host}:{p}"),
            None => host.clone(),
        };
        for raw in &self.bypass_list {
            let pat = raw.trim().to_ascii_lowercase();
            if pat.is_empty() {
                continue;
            }
            if pat == "*" || pat == host || pat == host_port {
                return true;
            }
            if let Some(suffix) = pat.strip_prefix("*.") {
                if host == suffix || host.ends_with(&format!(".{suffix}")) {
                    return true;
                }
            } else if let Some(suffix) = pat.strip_prefix('.') {
                if host == suffix || host.ends_with(&format!(".{suffix}")) {
                    return true;
                }
            }
        }
        false
    }
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
    #[serde(default)]
    pub client_cert: Option<ClientCertificateConfig>,
    #[serde(default)]
    pub proxy: Option<ProxyConfig>,
}

fn default_true() -> bool {
    true
}
fn default_max_redirects() -> usize {
    10
}
fn default_timeout() -> u64 {
    0
}

impl Default for RequestSettings {
    fn default() -> Self {
        Self {
            inherit_auth: true,
            verify_ssl: true,
            follow_redirects: true,
            max_redirects: 10,
            timeout_ms: 0,
            client_cert: None,
            proxy: None,
        }
    }
}

/// Scripts & declarative assertions attached to request
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptConfig {
    #[serde(default)]
    pub pre_request: String,
    #[serde(default)]
    pub post_response: String,
    #[serde(default)]
    pub tests: String,
    #[serde(default)]
    pub assertions: Vec<crate::assertions::DeclarativeAssertion>,
}

/// Core Aura Request representation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuraRequest {
    pub id: String,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    #[serde(default)]
    pub protocol: RequestProtocol,
    #[serde(default)]
    pub params: Vec<ParamItem>,
    #[serde(default)]
    pub headers: Vec<HeaderItem>,
    #[serde(default)]
    pub variables: HashMap<String, String>,
    #[serde(default)]
    pub disabled_variables: HashMap<String, String>,
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
    #[serde(default)]
    pub grpc: Option<GrpcConfig>,
    #[serde(default)]
    pub ws_templates: Vec<WsMessageTemplate>,
    #[serde(default)]
    pub flow: Option<FlowControl>,
    #[serde(default)]
    pub extractors: Vec<crate::extractors::NativeExtractor>,
}

impl AuraRequest {
    pub fn new(name: impl Into<String>, method: HttpMethod, url: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            method,
            url: url.into(),
            protocol: RequestProtocol::Http,
            params: Vec::new(),
            headers: Vec::new(),
            variables: HashMap::new(),
            disabled_variables: HashMap::new(),
            body: BodyType::None,
            auth: AuthConfig::default(),
            scripts: ScriptConfig::default(),
            settings: RequestSettings::default(),
            description: None,
            grpc: None,
            ws_templates: Vec::new(),
            flow: None,
            extractors: Vec::new(),
        }
    }

    pub fn add_header(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.headers.push(HeaderItem::new(key, value));
    }

    pub fn remove_header(&mut self, index: usize) -> Option<HeaderItem> {
        if index < self.headers.len() {
            Some(self.headers.remove(index))
        } else {
            None
        }
    }

    pub fn toggle_header(&mut self, index: usize) -> Option<bool> {
        if let Some(h) = self.headers.get_mut(index) {
            h.enabled = !h.enabled;
            Some(h.enabled)
        } else {
            None
        }
    }

    pub fn add_param(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.params.push(ParamItem::new(key, value));
    }

    pub fn remove_param(&mut self, index: usize) -> Option<ParamItem> {
        if index < self.params.len() {
            Some(self.params.remove(index))
        } else {
            None
        }
    }

    pub fn toggle_param(&mut self, index: usize) -> Option<bool> {
        if let Some(p) = self.params.get_mut(index) {
            p.enabled = !p.enabled;
            Some(p.enabled)
        } else {
            None
        }
    }

    pub fn set_body(&mut self, body: BodyType) {
        self.body = body;
    }

    pub fn set_auth(&mut self, auth: AuthConfig) {
        self.auth = auth;
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

/// Console log captured during script execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsoleMessage {
    pub level: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// HTTP Response from client engine
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptError {
    pub stage: String,
    pub message: String,
}

/// Phase-level HTTP timing breakdown (all durations in milliseconds).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestTimingBreakdown {
    #[serde(default)]
    pub dns_lookup_ms: u64,
    #[serde(default)]
    pub tcp_connect_ms: u64,
    #[serde(default)]
    pub tls_handshake_ms: u64,
    #[serde(default)]
    pub ttfb_ms: u64,
    #[serde(default)]
    pub content_download_ms: u64,
    #[serde(default)]
    pub total_ms: u64,
}

impl RequestTimingBreakdown {
    pub fn phases(&self) -> [(&'static str, u64); 5] {
        [
            ("DNS Lookup", self.dns_lookup_ms),
            ("TCP Connect", self.tcp_connect_ms),
            ("TLS Handshake", self.tls_handshake_ms),
            ("TTFB", self.ttfb_ms),
            ("Download", self.content_download_ms),
        ]
    }
}

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
    #[serde(default)]
    pub console_logs: Vec<ConsoleMessage>,
    #[serde(default)]
    pub script_errors: Vec<ScriptError>,
    #[serde(default)]
    pub sse_events: Vec<SseEvent>,
    #[serde(default)]
    pub stream_messages: Vec<StreamMessage>,
    #[serde(default)]
    pub timing: Option<RequestTimingBreakdown>,
    #[serde(default)]
    pub captured_cookies: Vec<crate::cookies::CookieEntry>,
    #[serde(default)]
    pub next_request: Option<String>,
    #[serde(default)]
    pub stop_chain: bool,
    #[serde(default)]
    pub exported_variables: HashMap<String, String>,
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

/// JMeter-style think-time distribution for jittered pacing between requests.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum ThinkTimeConfig {
    #[default]
    None,
    /// Uniform random jitter between `min_ms..=max_ms`.
    Uniform { min_ms: u64, max_ms: u64 },
    /// Gaussian (Normal) distributed think-time `mean_ms ± stddev_ms` (clamped >= 0).
    Gaussian { mean_ms: f64, stddev_ms: f64 },
}

impl ThinkTimeConfig {
    pub fn sample_duration(&self) -> Option<std::time::Duration> {
        use rand::Rng;
        let ms: f64 = match *self {
            Self::None => return None,
            Self::Uniform { min_ms, max_ms } => {
                if max_ms == 0 && min_ms == 0 {
                    return None;
                }
                let lo = min_ms.min(max_ms);
                let hi = min_ms.max(max_ms);
                if lo == hi {
                    lo as f64
                } else {
                    rand::thread_rng().gen_range(lo..=hi) as f64
                }
            }
            Self::Gaussian { mean_ms, stddev_ms } => {
                if mean_ms <= 0.0 && stddev_ms <= 0.0 {
                    return None;
                }
                let mut rng = rand::thread_rng();
                // Box-Muller transform for standard normal variate Z ~ N(0, 1)
                let u1: f64 = rng.gen_range(1e-12_f64..=1.0_f64);
                let u2: f64 = rng.gen_range(0.0_f64..1.0_f64);
                let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                (mean_ms + stddev_ms.abs() * z).max(0.0)
            }
        };
        if ms > 0.5 {
            Some(std::time::Duration::from_millis(ms.round() as u64))
        } else {
            None
        }
    }
}

fn default_max_loops() -> u32 {
    10
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
    /// Optional Gaussian timer mean (ms).
    #[serde(default)]
    pub gaussian_mean_ms: Option<u64>,
    /// Optional Gaussian timer standard deviation (ms).
    #[serde(default)]
    pub gaussian_stddev_ms: Option<u64>,
    /// Relative percentage weight for Throughput / Weighted Controller distribution.
    #[serde(default)]
    pub weight_percent: Option<u8>,
    /// Loop Controller repeat count for this request (`>= 1`).
    #[serde(default)]
    pub loop_count: Option<u32>,
    /// Conditional `If` controller expression (evaluated before sending request).
    #[serde(default)]
    pub condition_if: Option<String>,
    /// Conditional `While` controller expression (loops after each response while true).
    #[serde(default)]
    pub loop_while: Option<String>,
    /// Safety cap for `loop_while` iterations (defaults to 10).
    #[serde(default = "default_max_loops")]
    pub max_loops: u32,
    /// Per-request performance bounds (`max_duration_ms`, `min_size_bytes`, `max_size_bytes`).
    #[serde(default)]
    pub max_duration_ms: Option<u64>,
    #[serde(default)]
    pub min_size_bytes: Option<usize>,
    #[serde(default)]
    pub max_size_bytes: Option<usize>,
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
            gaussian_mean_ms: None,
            gaussian_stddev_ms: None,
            weight_percent: None,
            loop_count: None,
            condition_if: None,
            loop_while: None,
            max_loops: default_max_loops(),
            max_duration_ms: None,
            min_size_bytes: None,
            max_size_bytes: None,
        }
    }
}

impl FlowControl {
    pub fn calculate_pacing_delay(&self) -> Option<std::time::Duration> {
        let mut total_ms = self.delay_ms.unwrap_or(0);
        if let (Some(min), Some(max)) = (self.jitter_min_ms, self.jitter_max_ms) {
            if let Some(d) = (ThinkTimeConfig::Uniform { min_ms: min, max_ms: max }).sample_duration() {
                total_ms += d.as_millis() as u64;
            }
        }
        if let Some(mean) = self.gaussian_mean_ms {
            let stddev = self.gaussian_stddev_ms.unwrap_or(0);
            if let Some(d) = (ThinkTimeConfig::Gaussian {
                mean_ms: mean as f64,
                stddev_ms: stddev as f64,
            })
            .sample_duration()
            {
                total_ms += d.as_millis() as u64;
            }
        }
        if total_ms > 0 {
            Some(std::time::Duration::from_millis(total_ms))
        } else {
            None
        }
    }
}

/// Samples a slice index proportional to positive weights (for Weighted / Throughput Controller).
pub fn pick_weighted_index(weights: &[u32]) -> Option<usize> {
    let total: u64 = weights.iter().map(|&w| w as u64).sum();
    if total == 0 {
        return None;
    }
    use rand::Rng;
    let mut roll = rand::thread_rng().gen_range(0..total);
    for (idx, &w) in weights.iter().enumerate() {
        let w64 = w as u64;
        if roll < w64 {
            return Some(idx);
        }
        roll -= w64;
    }
    Some(weights.len().saturating_sub(1))
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
    #[serde(default)]
    pub protocol: Option<RequestProtocol>,
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
    #[serde(default)]
    pub grpc: Option<GrpcConfig>,
    #[serde(default)]
    pub ws_templates: Vec<WsMessageTemplate>,
    #[serde(default)]
    pub extractors: Vec<crate::extractors::NativeExtractor>,
}

impl Default for AuraFileRequest {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            name: "New Request".to_string(),
            method: "GET".to_string(),
            url: "".to_string(),
            protocol: None,
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
            grpc: None,
            ws_templates: Vec::new(),
            extractors: Vec::new(),
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
        let mut disabled_variables = HashMap::new();
        for (k, v) in r.variables {
            if k.ends_with('~') || k.starts_with('~') {
                disabled_variables.insert(k.trim_matches('~').to_string(), v);
            } else {
                variables.insert(k, v);
            }
        }

        let auth = r.auth.unwrap_or(AuthConfig::Inherit);

        let scripts = ScriptConfig {
            pre_request: r.scripts.pre_request.unwrap_or_default(),
            post_response: r.scripts.post_response.unwrap_or_default(),
            tests: r.tests.unwrap_or_default(),
            assertions: r
                .assert
                .as_deref()
                .map(crate::assertions::parse_assert_block)
                .unwrap_or_default(),
        };

        let settings = RequestSettings {
            inherit_auth: r.settings.inherit_auth,
            verify_ssl: r.settings.verify_ssl,
            follow_redirects: r.settings.follow_redirects,
            max_redirects: r.settings.max_redirects as usize,
            timeout_ms: r.settings.timeout_ms,
            client_cert: None,
            proxy: None,
        };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: r.name,
            method,
            url: r.url,
            protocol: r.protocol.unwrap_or_default(),
            params,
            headers,
            variables,
            disabled_variables,
            body: r.body,
            auth,
            scripts,
            settings,
            description: None,
            grpc: r.grpc,
            ws_templates: r.ws_templates,
            flow: r.flow,
            extractors: r.extractors,
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
        for (k, v) in req.disabled_variables {
            variables.insert(format!("{}~", k), v);
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
            protocol: if req.protocol == RequestProtocol::Http {
                None
            } else {
                Some(req.protocol)
            },
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
            flow: req.flow,
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
            assert: if req.scripts.assertions.is_empty() {
                None
            } else {
                Some(
                    req.scripts
                        .assertions
                        .iter()
                        .map(|a| {
                            if a.enabled {
                                a.summary()
                            } else {
                                format!("~{}", a.summary())
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
            },
            tests: if req.scripts.tests.is_empty() {
                None
            } else {
                Some(req.scripts.tests)
            },
            grpc: req.grpc,
            ws_templates: req.ws_templates,
            extractors: req.extractors,
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
