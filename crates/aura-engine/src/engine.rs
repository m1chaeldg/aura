//! Core HTTP Request Dispatcher and Execution Engine.

use anyhow::{anyhow, Result};
use aura_core::model::{AuraRequest, AuraResponse, BodyType, HttpMethod};
use aura_core::security::apply_auth;
use aura_core::templating::TemplateContext;
use aura_scripting::AuraScriptRuntime;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;

const MAX_LOG_PAYLOAD_LEN: usize = 2048;

struct RequestErrorDetails {
    kind: &'static str,
    cause_chain: String,
    possible_cause: String,
}

fn truncate_for_log(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut end = max_len;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}... [truncated, {} total bytes]", &s[..end], s.len())
    }
}

fn is_sensitive_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "authorization" | "proxy-authorization" | "cookie" | "set-cookie" | "x-api-key" | "api-key" | "x-auth-token"
    )
}

fn format_kv_for_log(pairs: &[(String, String)], redact_sensitive_keys: bool) -> String {
    if pairs.is_empty() {
        return "[]".to_string();
    }
    let items: Vec<String> = pairs
        .iter()
        .map(|(k, v)| {
            if redact_sensitive_keys && is_sensitive_header(k) {
                format!("{k}=***")
            } else {
                format!("{k}={}", truncate_for_log(v, 256))
            }
        })
        .collect();
    format!("[{}]", items.join(", "))
}

fn describe_reqwest_error(
    err: &reqwest::Error,
    timeout_ms: u64,
    max_redirects: usize,
) -> RequestErrorDetails {
    let mut causes = Vec::new();
    causes.push(err.to_string());
    let mut current: Option<&dyn std::error::Error> = std::error::Error::source(err);
    while let Some(src) = current {
        causes.push(src.to_string());
        current = src.source();
    }
    let cause_chain = causes.join(" -> ");
    let lower = cause_chain.to_ascii_lowercase();

    let kind = if err.is_timeout() {
        "Timeout"
    } else if err.is_connect() {
        "Connect"
    } else if err.is_redirect() {
        "Redirect"
    } else if err.is_builder() {
        "Builder"
    } else if err.is_request() {
        "Request"
    } else if err.is_body() {
        "Body"
    } else if err.is_decode() {
        "Decode"
    } else if err.is_status() {
        "Status"
    } else {
        "Network"
    };

    let possible_cause = match kind {
        "Timeout" => format!(
            "Request timed out after {timeout_ms}ms. The server may be slow or unreachable, or timeout_ms may need to be increased."
        ),
        "Connect" => {
            if lower.contains("dns")
                || lower.contains("failed to lookup")
                || lower.contains("no such host")
                || lower.contains("resolve")
            {
                "DNS resolution failed. Verify the hostname, internet connectivity, or DNS/VPN settings.".to_string()
            } else if lower.contains("connection refused") || lower.contains("10061") {
                "Connection refused by target host. Verify the host/port and ensure the server is running.".to_string()
            } else if lower.contains("tls")
                || lower.contains("ssl")
                || lower.contains("certificate")
                || lower.contains("handshake")
                || lower.contains("webpki")
                || lower.contains("rustls")
            {
                "TLS/SSL handshake failed. The server certificate may be invalid, self-signed, or expired (check verify_ssl).".to_string()
            } else if lower.contains("timed out") || lower.contains("timeout") {
                format!(
                    "TCP connection timed out (timeout_ms={timeout_ms}). Check firewall, proxy, or host reachability."
                )
            } else {
                "Failed to establish connection to the server. Check network connectivity, proxy/VPN, firewall, or target URL.".to_string()
            }
        }
        "Redirect" => format!(
            "Too many redirects or redirect loop (max_redirects={max_redirects})."
        ),
        "Builder" => "Invalid request URL, method, or header format.".to_string(),
        "Request" => "Failed while sending request headers or payload to the server (connection may have been reset or closed).".to_string(),
        "Body" => "Failed while reading or streaming the request/response body.".to_string(),
        "Decode" => "Failed to decode the response body (e.g., invalid encoding or compression).".to_string(),
        _ => "Unexpected network/transport error; inspect cause_chain for underlying OS or TLS details.".to_string(),
    };

    RequestErrorDetails {
        kind,
        cause_chain,
        possible_cause,
    }
}

/// Execute an AuraRequest with full lifecycle:
/// 1. Pre-request scripting (deno_core)
/// 2. Templating & variable interpolation
/// 3. Auth injection
/// 4. HTTP dispatch & timing
/// 5. Post-response scripting & test assertion evaluation (deno_core)
pub async fn execute_request(req: &AuraRequest, context: &TemplateContext) -> Result<AuraResponse> {
    let mut all_console_logs = Vec::new();
    let mut working_vars = context.request_vars.clone();
    for (k, v) in &context.environment_vars {
        working_vars.entry(k.clone()).or_insert_with(|| v.clone());
    }
    for (k, v) in &context.collection_vars {
        working_vars.entry(k.clone()).or_insert_with(|| v.clone());
    }

    // 1. Run Pre-Request script if present
    if !req.scripts.pre_request.is_empty() {
        let mut script_rt = AuraScriptRuntime::new(working_vars.clone())?;
        let res = script_rt.run(&req.scripts.pre_request)?;
        working_vars = res.variables;
        all_console_logs.extend(res.console_logs);
    }

    // Update context with potentially mutated variables
    let mut active_context = context.clone();
    active_context.request_vars = working_vars.clone();

    // 2. Interpolate URL and query parameters
    let rendered_url = active_context.render(&req.url);

    let mut url_params = Vec::new();
    for p in &req.params {
        if p.enabled {
            let rendered_k = active_context.render(&p.key);
            let rendered_v = active_context.render(&p.value);
            url_params.push((rendered_k, rendered_v));
        }
    }

    // 3. Headers
    let mut header_pairs = Vec::new();
    for h in &req.headers {
        if h.enabled {
            let rendered_k = active_context.render(&h.key);
            let rendered_v = active_context.render(&h.value);
            header_pairs.push((rendered_k, rendered_v));
        }
    }

    // 4. Inject Auth
    apply_auth(&req.auth, &mut header_pairs, &mut url_params);

    tracing::info!(
        target: "aura_engine::request",
        method = %req.method,
        url = %rendered_url,
        headers_count = header_pairs.len(),
        body_type = %req.body.type_name(),
        "Dispatching HTTP request"
    );

    // Build Reqwest Client
    let client_builder = reqwest::Client::builder()
        .danger_accept_invalid_certs(!req.settings.verify_ssl)
        .timeout(std::time::Duration::from_millis(req.settings.timeout_ms));

    let client_builder = if req.settings.follow_redirects {
        client_builder.redirect(reqwest::redirect::Policy::limited(
            req.settings.max_redirects,
        ))
    } else {
        client_builder.redirect(reqwest::redirect::Policy::none())
    };

    let client = client_builder.build()?;

    let req_method = match req.method {
        HttpMethod::GET => reqwest::Method::GET,
        HttpMethod::POST => reqwest::Method::POST,
        HttpMethod::PUT => reqwest::Method::PUT,
        HttpMethod::PATCH => reqwest::Method::PATCH,
        HttpMethod::DELETE => reqwest::Method::DELETE,
        HttpMethod::HEAD => reqwest::Method::HEAD,
        HttpMethod::OPTIONS => reqwest::Method::OPTIONS,
    };

    let mut request_builder = client.request(req_method, &rendered_url);

    let query_summary = format_kv_for_log(&url_params, false);
    let headers_summary = format_kv_for_log(&header_pairs, true);

    // Add query parameters
    if !url_params.is_empty() {
        request_builder = request_builder.query(&url_params);
    }

    // Add headers
    let mut header_map = HeaderMap::new();
    for (k, v) in header_pairs {
        match (HeaderName::from_str(&k), HeaderValue::from_str(&v)) {
            (Ok(name), Ok(val)) => {
                header_map.insert(name, val);
            }
            (name_res, val_res) => {
                tracing::warn!(
                    target: "aura_engine::request",
                    header_name = %k,
                    name_valid = name_res.is_ok(),
                    value_valid = val_res.is_ok(),
                    "Skipping invalid HTTP header"
                );
            }
        }
    }
    request_builder = request_builder.headers(header_map);

    // Add body and capture payload summary for diagnostics
    let payload_summary = match &req.body {
        BodyType::Json(s) => {
            let rendered_body = active_context.render(s);
            let summary = format!(
                "JSON ({} bytes): {}",
                rendered_body.len(),
                truncate_for_log(&rendered_body, MAX_LOG_PAYLOAD_LEN)
            );
            request_builder = request_builder
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(rendered_body);
            summary
        }
        BodyType::Xml(s) => {
            let rendered_body = active_context.render(s);
            let summary = format!(
                "XML ({} bytes): {}",
                rendered_body.len(),
                truncate_for_log(&rendered_body, MAX_LOG_PAYLOAD_LEN)
            );
            request_builder = request_builder
                .header(reqwest::header::CONTENT_TYPE, "application/xml")
                .body(rendered_body);
            summary
        }
        BodyType::Text(s) => {
            let rendered_body = active_context.render(s);
            let summary = format!(
                "Text ({} bytes): {}",
                rendered_body.len(),
                truncate_for_log(&rendered_body, MAX_LOG_PAYLOAD_LEN)
            );
            request_builder = request_builder
                .header(reqwest::header::CONTENT_TYPE, "text/plain")
                .body(rendered_body);
            summary
        }
        BodyType::Raw {
            content,
            content_type,
        } => {
            let rendered_body = active_context.render(content);
            let ct = content_type.as_deref().unwrap_or("text/plain");
            let summary = format!(
                "Raw[{ct}] ({} bytes): {}",
                rendered_body.len(),
                truncate_for_log(&rendered_body, MAX_LOG_PAYLOAD_LEN)
            );
            request_builder = request_builder
                .header(reqwest::header::CONTENT_TYPE, ct)
                .body(rendered_body);
            summary
        }
        BodyType::Form(map) => {
            let mut form_data = Vec::new();
            for (k, v) in map {
                form_data.push((active_context.render(k), active_context.render(v)));
            }
            let summary = format!(
                "Form ({} fields): {}",
                form_data.len(),
                format_kv_for_log(&form_data, false)
            );
            request_builder = request_builder.form(&form_data);
            summary
        }
        BodyType::FormUrlEncoded(items) => {
            let mut form_data = Vec::new();
            for item in items {
                if item.enabled {
                    form_data.push((
                        active_context.render(&item.key),
                        active_context.render(&item.value),
                    ));
                }
            }
            let summary = format!(
                "FormUrlEncoded ({} fields): {}",
                form_data.len(),
                format_kv_for_log(&form_data, false)
            );
            request_builder = request_builder.form(&form_data);
            summary
        }
        BodyType::Binary { file_path, .. } => match std::fs::read(file_path) {
            Ok(bytes) => {
                let len = bytes.len();
                request_builder = request_builder.body(bytes);
                format!("Binary (file={file_path}, {len} bytes)")
            }
            Err(err) => {
                tracing::warn!(
                    target: "aura_engine::request",
                    file_path = %file_path,
                    error = %err,
                    "Failed to read binary body file"
                );
                format!("Binary (file={file_path}, read_error={err})")
            }
        },
        BodyType::Multipart(fields) => {
            let mut form = reqwest::multipart::Form::new();
            let mut field_summaries = Vec::new();
            for f in fields {
                if let Some(v) = &f.value {
                    let rendered = active_context.render(v);
                    field_summaries.push(format!(
                        "{}={}",
                        f.name,
                        truncate_for_log(&rendered, 128)
                    ));
                    form = form.text(f.name.clone(), rendered);
                } else if let Some(path) = &f.file_path {
                    match std::fs::read(path) {
                        Ok(bytes) => {
                            field_summaries.push(format!(
                                "{}=<file:{} ({} bytes)>",
                                f.name,
                                path,
                                bytes.len()
                            ));
                            let part = reqwest::multipart::Part::bytes(bytes);
                            form = form.part(f.name.clone(), part);
                        }
                        Err(err) => {
                            tracing::warn!(
                                target: "aura_engine::request",
                                field = %f.name,
                                file_path = %path,
                                error = %err,
                                "Failed to read multipart file"
                            );
                            field_summaries.push(format!("{}=<file:{path} read_error={err}>", f.name));
                        }
                    }
                }
            }
            let summary = format!("Multipart [{}]", field_summaries.join(", "));
            request_builder = request_builder.multipart(form);
            summary
        }
        BodyType::None => "None".to_string(),
    };

    // Execute Request & measure exact duration
    let start_time = Instant::now();
    let resp = request_builder.send().await.map_err(|e| {
        let elapsed_ms = start_time.elapsed().as_millis() as u64;
        let details = describe_reqwest_error(&e, req.settings.timeout_ms, req.settings.max_redirects);
        tracing::error!(
            target: "aura_engine",
            method = %req.method,
            url = %rendered_url,
            query_params = %query_summary,
            headers = %headers_summary,
            payload = %payload_summary,
            error_kind = details.kind,
            duration_ms = elapsed_ms,
            timeout_ms = req.settings.timeout_ms,
            verify_ssl = req.settings.verify_ssl,
            cause_chain = %details.cause_chain,
            possible_cause = %details.possible_cause,
            error = %e,
            "HTTP network error encountered"
        );
        anyhow!(
            "Network error ({}): {} | Possible cause: {}",
            details.kind,
            details.cause_chain,
            details.possible_cause
        )
    })?;
    let duration_ms = start_time.elapsed().as_millis() as u64;

    let status = resp.status().as_u16();
    let status_text = resp.status().canonical_reason().unwrap_or("").to_string();

    let mut resp_headers = HashMap::new();
    for (name, val) in resp.headers() {
        if let Ok(val_str) = val.to_str() {
            resp_headers.insert(name.as_str().to_string(), val_str.to_string());
        }
    }

    let body_bytes = resp.bytes().await.unwrap_or_default();
    let size_bytes = body_bytes.len();
    let body_str = String::from_utf8_lossy(&body_bytes).to_string();

    // 5. Post-response & Test assertions execution
    let mut assertions = Vec::new();

    if !req.scripts.post_response.is_empty() || !req.scripts.tests.is_empty() {
        let mut script_rt = AuraScriptRuntime::new(working_vars)?;
        script_rt.inject_response_context_with_duration(
            status,
            &body_str,
            &resp_headers,
            duration_ms,
        )?;

        if !req.scripts.post_response.is_empty() {
            match script_rt.run(&req.scripts.post_response) {
                Ok(res) => {
                    all_console_logs.extend(res.console_logs);
                    assertions.extend(res.test_assertions);
                }
                Err(e) => all_console_logs.push(aura_core::model::ConsoleMessage {
                    level: "ERROR".to_string(),
                    message: format!("Post-response script error: {e}"),
                    timestamp: chrono::Utc::now(),
                }),
            }
        }

        if !req.scripts.tests.is_empty() {
            match script_rt.run(&req.scripts.tests) {
                Ok(test_res) => {
                    all_console_logs.extend(test_res.console_logs);
                    assertions.extend(test_res.test_assertions);
                }
                Err(e) => {
                    let msg = format!("Test script error: {e}");
                    all_console_logs.push(aura_core::model::ConsoleMessage {
                        level: "ERROR".to_string(),
                        message: msg.clone(),
                        timestamp: chrono::Utc::now(),
                    });
                    assertions.push(aura_core::model::TestAssertion {
                        name: "Test Script Execution".to_string(),
                        passed: false,
                        message: Some(msg),
                        duration_ms: 0,
                    });
                }
            }
        }
    }

    tracing::info!(
        target: "aura_engine::response",
        status = status,
        duration_ms = duration_ms,
        size_bytes = size_bytes,
        assertions_count = assertions.len(),
        assertions_failed = assertions.iter().filter(|a| !a.passed).count(),
        "Received HTTP response"
    );

    Ok(AuraResponse {
        status,
        status_text,
        duration_ms,
        size_bytes,
        headers: resp_headers,
        body: body_str,
        timestamp: chrono::Utc::now(),
        assertions,
        console_logs: all_console_logs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_for_log_respects_utf8_boundaries() {
        let s = "hello 🚀 world";
        let truncated = truncate_for_log(s, 8);
        assert!(truncated.contains(&format!("... [truncated, {} total bytes]", s.len())));
        assert!(truncated.starts_with("hello "));
    }

    #[test]
    fn test_format_kv_for_log_redacts_sensitive_headers() {
        let pairs = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Authorization".to_string(), "Bearer secret-token".to_string()),
            ("X-API-Key".to_string(), "my-secret-key".to_string()),
        ];
        let formatted = format_kv_for_log(&pairs, true);
        assert!(formatted.contains("Content-Type=application/json"));
        assert!(formatted.contains("Authorization=***"));
        assert!(formatted.contains("X-API-Key=***"));
        assert!(!formatted.contains("secret-token"));
        assert!(!formatted.contains("my-secret-key"));
    }
}
