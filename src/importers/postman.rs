//! Postman Collection (v2.1) importer

use crate::model::{AuraRequest, BodyType, Collection, Folder, HeaderItem, HttpMethod, ParamItem};
use anyhow::{Context, Result};
use serde_json::Value;

pub fn import_postman_collection(json_str: &str) -> Result<Collection> {
    let root: Value =
        serde_json::from_str(json_str).context("Invalid JSON for Postman Collection")?;

    let name = root["info"]["name"]
        .as_str()
        .unwrap_or("Imported Postman Collection")
        .to_string();

    let mut collection = Collection {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        auth: crate::model::AuthConfig::Inherit,
        variables: std::collections::HashMap::new(),
        requests: Vec::new(),
        folders: Vec::new(),
    };

    if let Some(items) = root["item"].as_array() {
        for item in items {
            process_item(item, &mut collection.requests, &mut collection.folders);
        }
    }

    Ok(collection)
}

fn process_item(item: &Value, requests: &mut Vec<AuraRequest>, folders: &mut Vec<Folder>) {
    let name = item["name"].as_str().unwrap_or("Untitled").to_string();

    // Check if it's a folder (contains "item" array)
    if let Some(sub_items) = item["item"].as_array() {
        let mut folder = Folder {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            auth: crate::model::AuthConfig::Inherit,
            requests: Vec::new(),
            subfolders: Vec::new(),
        };

        for sub in sub_items {
            process_item(sub, &mut folder.requests, &mut folder.subfolders);
        }

        folders.push(folder);
    } else if let Some(req_val) = item.get("request") {
        // It's a request item
        if let Some(req) = parse_postman_request(&name, req_val) {
            requests.push(req);
        }
    }
}

fn parse_postman_request(name: &str, req_val: &Value) -> Option<AuraRequest> {
    let method_str = req_val["method"].as_str().unwrap_or("GET");
    let method = match method_str.to_uppercase().as_str() {
        "POST" => HttpMethod::POST,
        "PUT" => HttpMethod::PUT,
        "PATCH" => HttpMethod::PATCH,
        "DELETE" => HttpMethod::DELETE,
        "HEAD" => HttpMethod::HEAD,
        "OPTIONS" => HttpMethod::OPTIONS,
        _ => HttpMethod::GET,
    };

    let url_str = if let Some(url_obj) = req_val.get("url") {
        if let Some(raw) = url_obj.get("raw").and_then(|r| r.as_str()) {
            raw.to_string()
        } else if let Some(raw_s) = url_obj.as_str() {
            raw_s.to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let mut aura_req = AuraRequest::new(name, method, url_str);

    // Parse headers
    if let Some(headers_arr) = req_val["header"].as_array() {
        for h in headers_arr {
            if let (Some(k), Some(v)) = (h["key"].as_str(), h["value"].as_str()) {
                let enabled = !h["disabled"].as_bool().unwrap_or(false);
                aura_req.headers.push(HeaderItem {
                    key: k.to_string(),
                    value: v.to_string(),
                    enabled,
                    description: h["description"].as_str().map(|s| s.to_string()),
                });
            }
        }
    }

    // Parse query params if present in URL
    if let Some(query_arr) = req_val["url"]["query"].as_array() {
        for q in query_arr {
            if let (Some(k), Some(v)) = (q["key"].as_str(), q["value"].as_str()) {
                let enabled = !q["disabled"].as_bool().unwrap_or(false);
                aura_req.params.push(ParamItem {
                    key: k.to_string(),
                    value: v.to_string(),
                    enabled,
                    description: q["description"].as_str().map(|s| s.to_string()),
                });
            }
        }
    }

    // Parse body
    if let Some(body_obj) = req_val.get("body") {
        let mode = body_obj["mode"].as_str().unwrap_or("");
        match mode {
            "raw" => {
                if let Some(raw) = body_obj["raw"].as_str() {
                    aura_req.body = BodyType::Json(raw.to_string());
                }
            }
            "urlencoded" => {
                if let Some(arr) = body_obj["urlencoded"].as_array() {
                    let mut items = Vec::new();
                    for item in arr {
                        if let (Some(k), Some(v)) = (item["key"].as_str(), item["value"].as_str()) {
                            items.push(ParamItem {
                                key: k.to_string(),
                                value: v.to_string(),
                                enabled: !item["disabled"].as_bool().unwrap_or(false),
                                description: None,
                            });
                        }
                    }
                    aura_req.body = BodyType::FormUrlEncoded(items);
                }
            }
            _ => {}
        }
    }

    Some(aura_req)
}
