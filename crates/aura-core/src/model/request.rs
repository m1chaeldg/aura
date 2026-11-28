//! Core `AuraRequest`, `RequestSettings`, and `ScriptConfig` models.

use super::{
    AuthConfig, BodyType, ClientCertificateConfig, FlowControl, GrpcConfig, HeaderItem, HttpMethod,
    ParamItem, ProxyConfig, RequestProtocol, WsMessageTemplate,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Network and execution settings for a request.
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

/// Scripts & declarative assertions attached to a request.
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

/// Core Aura Request representation.
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

impl Default for AuraRequest {
    fn default() -> Self {
        Self::new("New Request", HttpMethod::GET, "")
    }
}
