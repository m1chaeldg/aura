//! cURL command parser into AuraRequest

use crate::model::{AuraRequest, BodyType, HeaderItem, HttpMethod, ParamItem};
use anyhow::{anyhow, Result};
use url::Url;

/// Parse a raw cURL command into an AuraRequest
pub fn parse_curl_command(curl_str: &str) -> Result<AuraRequest> {
    let clean = curl_str.trim();
    if !clean.starts_with("curl") && !clean.starts_with("CURL") {
        return Err(anyhow!("Not a valid cURL command (must start with curl)"));
    }

    // Tokenize taking quotes into account
    let tokens = tokenize(clean);
    let mut method = HttpMethod::GET;
    let mut url_str = String::new();
    let mut headers = Vec::new();
    let mut data: Option<String> = None;
    let mut basic_auth: Option<String> = None;

    let mut i = 1;
    while i < tokens.len() {
        let token = &tokens[i];
        match token.as_str() {
            "-X" | "--request" => {
                if i + 1 < tokens.len() {
                    let m = tokens[i + 1].to_uppercase();
                    method = match m.as_str() {
                        "POST" => HttpMethod::POST,
                        "PUT" => HttpMethod::PUT,
                        "PATCH" => HttpMethod::PATCH,
                        "DELETE" => HttpMethod::DELETE,
                        "HEAD" => HttpMethod::HEAD,
                        "OPTIONS" => HttpMethod::OPTIONS,
                        _ => HttpMethod::GET,
                    };
                    i += 2;
                    continue;
                }
            }
            "-H" | "--header" => {
                if i + 1 < tokens.len() {
                    let h_val = &tokens[i + 1];
                    if let Some((k, v)) = h_val.split_once(':') {
                        headers.push(HeaderItem {
                            key: k.trim().to_string(),
                            value: v.trim().to_string(),
                            enabled: true,
                            description: None,
                        });
                    }
                    i += 2;
                    continue;
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" => {
                if i + 1 < tokens.len() {
                    data = Some(tokens[i + 1].clone());
                    if method == HttpMethod::GET {
                        method = HttpMethod::POST;
                    }
                    i += 2;
                    continue;
                }
            }
            "-u" | "--user" => {
                if i + 1 < tokens.len() {
                    basic_auth = Some(tokens[i + 1].clone());
                    i += 2;
                    continue;
                }
            }
            _ => {
                if !token.starts_with('-')
                    && (token.starts_with("http://") || token.starts_with("https://"))
                {
                    url_str = token.clone();
                }
            }
        }
        i += 1;
    }

    if url_str.is_empty() {
        // Fallback: look for any token that looks like a host or url
        for token in &tokens[1..] {
            if !token.starts_with('-') && (token.contains("://") || token.contains('.')) {
                url_str = token.clone();
                break;
            }
        }
    }

    if url_str.is_empty() {
        return Err(anyhow!("Could not find target URL in cURL command"));
    }

    let mut params = Vec::new();
    if let Ok(parsed_url) = Url::parse(&url_str) {
        for (k, v) in parsed_url.query_pairs() {
            params.push(ParamItem {
                key: k.to_string(),
                value: v.to_string(),
                enabled: true,
                description: None,
            });
        }
    }

    let body = if let Some(raw_data) = data {
        if raw_data.trim_start().starts_with('{') || raw_data.trim_start().starts_with('[') {
            BodyType::Json(raw_data)
        } else if raw_data.trim_start().starts_with('<') {
            BodyType::Xml(raw_data)
        } else {
            BodyType::Text(raw_data)
        }
    } else {
        BodyType::None
    };

    let mut req = AuraRequest::new("cURL Request", method, url_str);
    req.params = params;
    req.headers = headers;
    req.body = body;

    if let Some(auth_val) = basic_auth {
        if let Some((user, pass)) = auth_val.split_once(':') {
            req.auth = crate::model::AuthConfig::Basic {
                username: user.to_string(),
                password: pass.to_string(),
            };
        }
    }

    Ok(req)
}

fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        if ch == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            continue;
        }

        if ch == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            continue;
        }

        if ch.is_whitespace() && !in_single_quote && !in_double_quote {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
            continue;
        }

        current.push(ch);
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}
