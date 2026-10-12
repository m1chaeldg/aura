//! OpenAPI (Swagger v2 / OpenAPI v3) importer

use aura_core::model::{AuraRequest, BodyType, Collection, HeaderItem, HttpMethod, ParamItem};
use anyhow::{Context, Result};
use serde_json::Value;

pub fn import_openapi(json_or_yaml_str: &str) -> Result<Collection> {
    // Try parsing as JSON first
    let root: Value =
        serde_json::from_str(json_or_yaml_str).context("Invalid JSON for OpenAPI specification")?;

    let title = root["info"]["title"]
        .as_str()
        .unwrap_or("OpenAPI Specification")
        .to_string();

    let mut base_url = String::new();
    // OpenAPI v3 servers array
    if let Some(servers) = root["servers"].as_array() {
        if let Some(first) = servers.first() {
            if let Some(url) = first["url"].as_str() {
                base_url = url.trim_end_matches('/').to_string();
            }
        }
    }
    // Swagger v2 host + basePath
    if base_url.is_empty() {
        if let Some(host) = root["host"].as_str() {
            let scheme = root["schemes"]
                .as_array()
                .and_then(|s| s.first())
                .and_then(|s| s.as_str())
                .unwrap_or("https");
            let base_path = root["basePath"].as_str().unwrap_or("");
            base_url = format!("{}://{}{}", scheme, host, base_path.trim_end_matches('/'));
        }
    }

    let mut requests = Vec::new();

    if let Some(paths) = root["paths"].as_object() {
        for (path, path_item) in paths {
            if let Some(methods_obj) = path_item.as_object() {
                for (method_str, op_val) in methods_obj {
                    let method = match method_str.to_uppercase().as_str() {
                        "GET" => HttpMethod::GET,
                        "POST" => HttpMethod::POST,
                        "PUT" => HttpMethod::PUT,
                        "PATCH" => HttpMethod::PATCH,
                        "DELETE" => HttpMethod::DELETE,
                        "HEAD" => HttpMethod::HEAD,
                        "OPTIONS" => HttpMethod::OPTIONS,
                        _ => continue,
                    };

                    let op_id = op_val["operationId"]
                        .as_str()
                        .or_else(|| op_val["summary"].as_str())
                        .unwrap_or(path);

                    let full_url = if base_url.is_empty() {
                        format!("https://api.example.com{}", path)
                    } else {
                        format!("{}{}", base_url, path)
                    };

                    let mut req = AuraRequest::new(op_id, method, full_url);
                    req.description = op_val["description"].as_str().map(|s| s.to_string());

                    // Parameters
                    if let Some(params_arr) = op_val["parameters"].as_array() {
                        for p in params_arr {
                            let name = p["name"].as_str().unwrap_or("");
                            let in_loc = p["in"].as_str().unwrap_or("");
                            let desc = p["description"].as_str().map(|s| s.to_string());

                            match in_loc {
                                "query" => {
                                    req.params.push(ParamItem {
                                        key: name.to_string(),
                                        value: String::new(),
                                        enabled: true,
                                        description: desc,
                                    });
                                }
                                "header" => {
                                    req.headers.push(HeaderItem {
                                        key: name.to_string(),
                                        value: String::new(),
                                        enabled: true,
                                        description: desc,
                                    });
                                }
                                _ => {}
                            }
                        }
                    }

                    // Body in OpenAPI v3
                    if let Some(_request_body) = op_val.get("requestBody") {
                        req.body = BodyType::Json("{}".to_string());
                    }

                    requests.push(req);
                }
            }
        }
    }

    Ok(Collection {
        id: uuid::Uuid::new_v4().to_string(),
        name: title,
        auth: aura_core::model::AuthConfig::Inherit,
        variables: std::collections::HashMap::new(),
        requests,
        folders: Vec::new(),
    })
}
