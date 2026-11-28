//! File/Manifest models compatible with `.aura` lossless disk format.

use super::{
    AuraRequest, AuthConfig, BodyType, FlowControl, GrpcConfig, HeaderItem, HttpMethod, ParamItem,
    RequestProtocol, RequestSettings, ScriptConfig, WsMessageTemplate,
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

        let headers = r
            .headers
            .into_iter()
            .map(|(k, v)| {
                let (key, enabled) = split_tilde_key(k);
                HeaderItem {
                    key,
                    value: v,
                    enabled,
                    description: None,
                }
            })
            .collect();

        let params = r
            .query_params
            .into_iter()
            .map(|(k, v)| {
                let (key, enabled) = split_tilde_key(k);
                ParamItem {
                    key,
                    value: v,
                    enabled,
                    description: None,
                }
            })
            .collect();

        let mut variables = HashMap::new();
        let mut disabled_variables = HashMap::new();
        for (k, v) in r.variables {
            let (key, enabled) = split_tilde_key(k);
            if enabled {
                variables.insert(key, v);
            } else {
                disabled_variables.insert(key, v);
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
            headers.insert(encode_tilde_key(h.key, h.enabled), h.value);
        }

        let mut query_params = IndexMap::new();
        for p in req.params {
            query_params.insert(encode_tilde_key(p.key, p.enabled), p.value);
        }

        let mut variables = IndexMap::new();
        for (k, v) in req.variables {
            variables.insert(k, v);
        }
        for (k, v) in req.disabled_variables {
            variables.insert(format!("{k}~"), v);
        }

        let auth = match req.auth {
            AuthConfig::Inherit => None,
            AuthConfig::None => Some(AuthConfig::None),
            other => Some(other),
        };

        let assert_block = if req.scripts.assertions.is_empty() {
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
                pre_request: (!req.scripts.pre_request.is_empty()).then_some(req.scripts.pre_request),
                post_response: (!req.scripts.post_response.is_empty())
                    .then_some(req.scripts.post_response),
            },
            assert: assert_block,
            tests: (!req.scripts.tests.is_empty()).then_some(req.scripts.tests),
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

fn split_tilde_key(key: String) -> (String, bool) {
    if key.ends_with('~') || key.starts_with('~') {
        (key.trim_matches('~').to_string(), false)
    } else {
        (key, true)
    }
}

fn encode_tilde_key(key: String, enabled: bool) -> String {
    if enabled { key } else { format!("{key}~") }
}
