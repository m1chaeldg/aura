//! Insomnia export importer

use aura_core::model::{AuraRequest, BodyType, Collection, HeaderItem, HttpMethod, ParamItem};
use anyhow::{Context, Result};
use serde_json::Value;

pub fn import_insomnia(json_str: &str) -> Result<Collection> {
    let root: Value = serde_json::from_str(json_str).context("Invalid JSON for Insomnia export")?;

    let resources = root["resources"]
        .as_array()
        .context("Missing 'resources' array in Insomnia export")?;

    let mut col_name = "Imported Insomnia Collection".to_string();
    let mut requests = Vec::new();

    // Check workspace name
    for r in resources {
        if r["_type"].as_str() == Some("workspace") {
            if let Some(n) = r["name"].as_str() {
                col_name = n.to_string();
            }
        }
    }

    for r in resources {
        if r["_type"].as_str() == Some("request") {
            let name = r["name"].as_str().unwrap_or("Untitled Request");
            let method_str = r["method"].as_str().unwrap_or("GET");
            let url_str = r["url"].as_str().unwrap_or("");

            let method = match method_str.to_uppercase().as_str() {
                "POST" => HttpMethod::POST,
                "PUT" => HttpMethod::PUT,
                "PATCH" => HttpMethod::PATCH,
                "DELETE" => HttpMethod::DELETE,
                "HEAD" => HttpMethod::HEAD,
                "OPTIONS" => HttpMethod::OPTIONS,
                _ => HttpMethod::GET,
            };

            let mut req = AuraRequest::new(name, method, url_str);

            // Headers
            if let Some(headers_arr) = r["headers"].as_array() {
                for h in headers_arr {
                    if let (Some(k), Some(v)) = (h["name"].as_str(), h["value"].as_str()) {
                        let disabled = h["disabled"].as_bool().unwrap_or(false);
                        req.headers.push(HeaderItem {
                            key: k.to_string(),
                            value: v.to_string(),
                            enabled: !disabled,
                            description: None,
                        });
                    }
                }
            }

            // Parameters
            if let Some(params_arr) = r["parameters"].as_array() {
                for p in params_arr {
                    if let (Some(k), Some(v)) = (p["name"].as_str(), p["value"].as_str()) {
                        let disabled = p["disabled"].as_bool().unwrap_or(false);
                        req.params.push(ParamItem {
                            key: k.to_string(),
                            value: v.to_string(),
                            enabled: !disabled,
                            description: None,
                        });
                    }
                }
            }

            // Body
            if let Some(body_obj) = r.get("body") {
                if let Some(text) = body_obj["text"].as_str() {
                    req.body = BodyType::Json(text.to_string());
                }
            }

            requests.push(req);
        }
    }

    Ok(Collection {
        id: uuid::Uuid::new_v4().to_string(),
        name: col_name,
        auth: aura_core::model::AuthConfig::Inherit,
        variables: std::collections::HashMap::new(),
        requests,
        folders: Vec::new(),
    })
}
