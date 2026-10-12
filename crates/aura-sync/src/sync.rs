//! Dual-Mode Request Editor: Lossless bidirectional synchronization between
//! the Visual Editor model and the Code Editor TOML representation.
//! Disabled parameters and headers serialize with trailing tilde (`key~ = "value"`).

use anyhow::{Context, Result};
use aura_core::model::{AuraRequest, BodyType, HeaderItem, HttpMethod, ParamItem, ScriptConfig};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Serialized representation of AuraRequest in TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlRequestSchema {
    pub name: String,
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub params: BTreeMap<String, String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub body_type: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub pre_request_script: Option<String>,
    #[serde(default)]
    pub post_response_script: Option<String>,
    #[serde(default)]
    pub tests_script: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// Convert AuraRequest to lossless TOML string
pub fn request_to_toml(req: &AuraRequest) -> Result<String> {
    let mut params_map = BTreeMap::new();
    for p in &req.params {
        let key = if p.enabled {
            p.key.clone()
        } else {
            format!("{}~", p.key)
        };
        params_map.insert(key, p.value.clone());
    }

    let mut headers_map = BTreeMap::new();
    for h in &req.headers {
        let key = if h.enabled {
            h.key.clone()
        } else {
            format!("{}~", h.key)
        };
        headers_map.insert(key, h.value.clone());
    }

    let (body_type, body_content) = match &req.body {
        BodyType::None => (None, None),
        BodyType::Json(s) => (Some("json".to_string()), Some(s.clone())),
        BodyType::Xml(s) => (Some("xml".to_string()), Some(s.clone())),
        BodyType::Text(s) => (Some("text".to_string()), Some(s.clone())),
        BodyType::FormUrlEncoded(items) => {
            let json = serde_json::to_string_pretty(items).unwrap_or_default();
            (Some("form-urlencoded".to_string()), Some(json))
        }
        BodyType::Multipart(fields) => {
            let json = serde_json::to_string_pretty(fields).unwrap_or_default();
            (Some("multipart".to_string()), Some(json))
        }
        BodyType::Binary { relative_path } => {
            (Some("binary".to_string()), Some(relative_path.clone()))
        }
    };

    let schema = TomlRequestSchema {
        name: req.name.clone(),
        method: req.method.as_str().to_string(),
        url: req.url.clone(),
        description: req.description.clone(),
        params: params_map,
        headers: headers_map,
        body_type,
        body: body_content,
        pre_request_script: if req.scripts.pre_request.is_empty() {
            None
        } else {
            Some(req.scripts.pre_request.clone())
        },
        post_response_script: if req.scripts.post_response.is_empty() {
            None
        } else {
            Some(req.scripts.post_response.clone())
        },
        tests_script: if req.scripts.tests.is_empty() {
            None
        } else {
            Some(req.scripts.tests.clone())
        },
        timeout_ms: Some(req.settings.timeout_ms),
    };

    toml::to_string_pretty(&schema).context("Failed to serialize request to TOML")
}

/// Parse TOML string back into AuraRequest losslessly
pub fn toml_to_request(toml_str: &str, existing_id: Option<&str>) -> Result<AuraRequest> {
    let schema: TomlRequestSchema =
        toml::from_str(toml_str).context("Failed to parse TOML request")?;

    let method = match schema.method.to_uppercase().as_str() {
        "GET" => HttpMethod::GET,
        "POST" => HttpMethod::POST,
        "PUT" => HttpMethod::PUT,
        "PATCH" => HttpMethod::PATCH,
        "DELETE" => HttpMethod::DELETE,
        "HEAD" => HttpMethod::HEAD,
        "OPTIONS" => HttpMethod::OPTIONS,
        _ => HttpMethod::GET,
    };

    let mut params = Vec::new();
    for (k, v) in schema.params {
        if let Some(clean_key) = k.strip_suffix('~') {
            params.push(ParamItem {
                key: clean_key.to_string(),
                value: v,
                enabled: false,
                description: None,
            });
        } else {
            params.push(ParamItem {
                key: k,
                value: v,
                enabled: true,
                description: None,
            });
        }
    }

    let mut headers = Vec::new();
    for (k, v) in schema.headers {
        if let Some(clean_key) = k.strip_suffix('~') {
            headers.push(HeaderItem {
                key: clean_key.to_string(),
                value: v,
                enabled: false,
                description: None,
            });
        } else {
            headers.push(HeaderItem {
                key: k,
                value: v,
                enabled: true,
                description: None,
            });
        }
    }

    let body = match (schema.body_type.as_deref(), schema.body) {
        (Some("json"), Some(s)) => BodyType::Json(s),
        (Some("xml"), Some(s)) => BodyType::Xml(s),
        (Some("text"), Some(s)) => BodyType::Text(s),
        (Some("form-urlencoded"), Some(s)) => {
            let items = serde_json::from_str(&s).unwrap_or_default();
            BodyType::FormUrlEncoded(items)
        }
        (Some("multipart"), Some(s)) => {
            let items = serde_json::from_str(&s).unwrap_or_default();
            BodyType::Multipart(items)
        }
        (Some("binary"), Some(rel_path)) => BodyType::Binary {
            relative_path: rel_path,
        },
        _ => BodyType::None,
    };

    let mut req = AuraRequest::new(schema.name, method, schema.url);
    if let Some(id) = existing_id {
        req.id = id.to_string();
    }
    req.description = schema.description;
    req.params = params;
    req.headers = headers;
    req.body = body;
    req.scripts = ScriptConfig {
        pre_request: schema.pre_request_script.unwrap_or_default(),
        post_response: schema.post_response_script.unwrap_or_default(),
        tests: schema.tests_script.unwrap_or_default(),
    };
    if let Some(t) = schema.timeout_ms {
        req.settings.timeout_ms = t;
    }

    Ok(req)
}
