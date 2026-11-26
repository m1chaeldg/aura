//! Core HTTP Request Dispatcher and Execution Engine.

use anyhow::{anyhow, Result};
use aura_core::graphql::{parse_introspection_response, GraphQLSchema, INTROSPECTION_QUERY};
use aura_core::grpc::parse_proto_file;
use aura_core::model::{
    AuthConfig, AuraRequest, AuraResponse, BodyType, HttpMethod, RequestProtocol, SseEvent,
};
use aura_core::security::{
    apply_auth, resolve_aws_profile_credentials, sign_aws_sigv4, AwsSigV4Credentials,
};
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
        "Timeout" => {
            if timeout_ms == 0 {
                "Request timed out at the OS/network layer.".to_string()
            } else {
                format!(
                    "Request timed out after {timeout_ms}ms. The server may be slow or unreachable, or timeout_ms may need to be increased."
                )
            }
        }
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

fn render_auth_config(auth: &AuthConfig, ctx: &TemplateContext) -> AuthConfig {
    match auth {
        AuthConfig::Inherit => AuthConfig::Inherit,
        AuthConfig::None => AuthConfig::None,
        AuthConfig::Bearer { token } => AuthConfig::Bearer {
            token: ctx.render(token),
        },
        AuthConfig::Basic { username, password } => AuthConfig::Basic {
            username: ctx.render(username),
            password: ctx.render(password),
        },
        AuthConfig::ApiKey {
            key,
            value,
            in_header,
        } => AuthConfig::ApiKey {
            key: ctx.render(key),
            value: ctx.render(value),
            in_header: *in_header,
        },
        AuthConfig::AwsSigV4 {
            use_profile,
            profile,
            credentials_path,
            access_key,
            secret_key,
            region,
            service,
            session_token,
        } => AuthConfig::AwsSigV4 {
            use_profile: *use_profile,
            profile: ctx.render(profile),
            credentials_path: credentials_path.as_ref().map(|p| ctx.render(p)),
            access_key: ctx.render(access_key),
            secret_key: ctx.render(secret_key),
            region: ctx.render(region),
            service: ctx.render(service),
            session_token: session_token.as_ref().map(|s| ctx.render(s)),
        },
        AuthConfig::OAuth2 {
            grant_type,
            access_token,
            token_url,
            client_id,
            client_secret,
        } => AuthConfig::OAuth2 {
            grant_type: ctx.render(grant_type),
            access_token: ctx.render(access_token),
            token_url: token_url.as_ref().map(|u| ctx.render(u)),
            client_id: client_id.as_ref().map(|c| ctx.render(c)),
            client_secret: client_secret.as_ref().map(|s| ctx.render(s)),
        },
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
    let mut script_errors = Vec::new();
    let mut working_vars = context.request_vars.clone();
    for (k, v) in &req.variables {
        working_vars.entry(k.clone()).or_insert_with(|| v.clone());
    }
    for (k, v) in &context.environment_vars {
        working_vars.entry(k.clone()).or_insert_with(|| v.clone());
    }
    for (k, v) in &context.collection_vars {
        working_vars.entry(k.clone()).or_insert_with(|| v.clone());
    }

    // 1. Run Pre-Request script if present
    if !req.scripts.pre_request.is_empty() {
        let mut script_rt = AuraScriptRuntime::new(working_vars.clone())?;
        match script_rt.run(&req.scripts.pre_request) {
            Ok(res) => {
                working_vars = res.variables;
                all_console_logs.extend(res.console_logs);
            }
            Err(e) => {
                let msg = format!("{e:#}");
                tracing::error!(
                    target: "aura_engine::scripting",
                    stage = "pre_request",
                    method = %req.method,
                    url = %req.url,
                    error = %msg,
                    "Pre-request script failed"
                );
                all_console_logs.push(aura_core::model::ConsoleMessage {
                    level: "ERROR".to_string(),
                    message: format!("Pre-request script error: {msg}"),
                    timestamp: chrono::Utc::now(),
                });
                script_errors.push(aura_core::model::ScriptError {
                    stage: "Pre-request Script".to_string(),
                    message: msg,
                });
                return Ok(AuraResponse {
                    status: 0,
                    status_text: "Pre-request Script Error".to_string(),
                    duration_ms: 0,
                    size_bytes: 0,
                    headers: HashMap::new(),
                    body: String::new(),
                    timestamp: chrono::Utc::now(),
                    assertions: Vec::new(),
                    console_logs: all_console_logs,
                    script_errors,
                    sse_events: Vec::new(),
                    stream_messages: Vec::new(),
                });
            }
        }
    }

    // Update context with potentially mutated variables
    let mut active_context = context.clone();
    active_context.request_vars = working_vars.clone();

    // Protocol dispatch for gRPC and SSE when explicitly selected
    if req.protocol == RequestProtocol::Grpc {
        let proto_schema = req.grpc.as_ref().and_then(|g| {
            let src = g.proto_content.clone().or_else(|| {
                g.proto_path
                    .as_deref()
                    .and_then(|p| std::fs::read_to_string(p).ok())
            })?;
            parse_proto_file(&src).ok()
        });
        return crate::grpc::execute_grpc_call(req, proto_schema.as_ref(), &active_context, None)
            .await;
    }
    if req.protocol == RequestProtocol::Sse {
        return crate::sse::execute_sse_request(req, &active_context, None).await;
    }

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

    // 4. Interpolate & Inject Auth (non-SigV4 auth injects immediately; SigV4 signs after body/URL finalization)
    let rendered_auth = render_auth_config(&req.auth, &active_context);
    if !matches!(rendered_auth, AuthConfig::AwsSigV4 { .. }) {
        apply_auth(&rendered_auth, &mut header_pairs, &mut url_params);
    }

    // Build Reqwest Client (timeout_ms == 0 means unlimited / no timeout)
    let mut client_builder = reqwest::Client::builder()
        .danger_accept_invalid_certs(!req.settings.verify_ssl);
    if req.settings.timeout_ms > 0 {
        client_builder =
            client_builder.timeout(std::time::Duration::from_millis(req.settings.timeout_ms));
    }

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

    let mut request_builder = client.request(req_method.clone(), &rendered_url);

    // Add query parameters
    if !url_params.is_empty() {
        request_builder = request_builder.query(&url_params);
    }

    // Add body and capture payload bytes + summary for SigV4 signing and diagnostics
    let mut body_bytes_for_signing: Vec<u8> = Vec::new();
    let payload_summary = match &req.body {
        BodyType::Json(s) => {
            let rendered_body = active_context.render(s);
            let summary = format!(
                "JSON ({} bytes): {}",
                rendered_body.len(),
                truncate_for_log(&rendered_body, MAX_LOG_PAYLOAD_LEN)
            );
            if !header_pairs
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            {
                header_pairs.push(("Content-Type".to_string(), "application/json".to_string()));
            }
            body_bytes_for_signing = rendered_body.as_bytes().to_vec();
            request_builder = request_builder.body(rendered_body);
            summary
        }
        BodyType::Xml(s) => {
            let rendered_body = active_context.render(s);
            let summary = format!(
                "XML ({} bytes): {}",
                rendered_body.len(),
                truncate_for_log(&rendered_body, MAX_LOG_PAYLOAD_LEN)
            );
            if !header_pairs
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            {
                header_pairs.push(("Content-Type".to_string(), "application/xml".to_string()));
            }
            body_bytes_for_signing = rendered_body.as_bytes().to_vec();
            request_builder = request_builder.body(rendered_body);
            summary
        }
        BodyType::Text(s) => {
            let rendered_body = active_context.render(s);
            let summary = format!(
                "Text ({} bytes): {}",
                rendered_body.len(),
                truncate_for_log(&rendered_body, MAX_LOG_PAYLOAD_LEN)
            );
            if !header_pairs
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            {
                header_pairs.push(("Content-Type".to_string(), "text/plain".to_string()));
            }
            body_bytes_for_signing = rendered_body.as_bytes().to_vec();
            request_builder = request_builder.body(rendered_body);
            summary
        }
        BodyType::Raw {
            content,
            content_type,
        } => {
            let rendered_body = active_context.render(content);
            let rendered_ct = content_type
                .as_deref()
                .map(|c| active_context.render(c))
                .unwrap_or_else(|| "text/plain".to_string());
            let summary = format!(
                "Raw[{}] ({} bytes): {}",
                rendered_ct,
                rendered_body.len(),
                truncate_for_log(&rendered_body, MAX_LOG_PAYLOAD_LEN)
            );
            if !header_pairs
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            {
                header_pairs.push(("Content-Type".to_string(), rendered_ct));
            }
            body_bytes_for_signing = rendered_body.as_bytes().to_vec();
            request_builder = request_builder.body(rendered_body);
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
            let encoded = url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(form_data.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .finish();
            body_bytes_for_signing = encoded.as_bytes().to_vec();
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
            let encoded = url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(form_data.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .finish();
            body_bytes_for_signing = encoded.as_bytes().to_vec();
            request_builder = request_builder.form(&form_data);
            summary
        }
        BodyType::Binary {
            file_path,
            content_type,
        } => {
            let rendered_path = active_context.render(file_path);
            let rendered_ct = content_type.as_deref().map(|c| active_context.render(c));
            match std::fs::read(&rendered_path) {
                Ok(bytes) => {
                    let len = bytes.len();
                    if let Some(ct) = rendered_ct {
                        if !header_pairs
                            .iter()
                            .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                        {
                            header_pairs.push(("Content-Type".to_string(), ct));
                        }
                    }
                    body_bytes_for_signing = bytes.clone();
                    request_builder = request_builder.body(bytes);
                    format!("Binary (file={rendered_path}, {len} bytes)")
                }
                Err(err) => {
                    tracing::warn!(
                        target: "aura_engine::request",
                        file_path = %rendered_path,
                        error = %err,
                        "Failed to read binary body file"
                    );
                    format!("Binary (file={rendered_path}, read_error={err})")
                }
            }
        }
        BodyType::Multipart(fields) => {
            let mut form = reqwest::multipart::Form::new();
            let mut field_summaries = Vec::new();
            for f in fields {
                let field_name = active_context.render(&f.name);
                if let Some(v) = &f.value {
                    let rendered = active_context.render(v);
                    field_summaries.push(format!(
                        "{}={}",
                        field_name,
                        truncate_for_log(&rendered, 128)
                    ));
                    form = form.text(field_name, rendered);
                } else if let Some(path) = &f.file_path {
                    let rendered_path = active_context.render(path);
                    match std::fs::read(&rendered_path) {
                        Ok(bytes) => {
                            field_summaries.push(format!(
                                "{}=<file:{} ({} bytes)>",
                                field_name,
                                rendered_path,
                                bytes.len()
                            ));
                            let part = reqwest::multipart::Part::bytes(bytes);
                            form = form.part(field_name, part);
                        }
                        Err(err) => {
                            tracing::warn!(
                                target: "aura_engine::request",
                                field = %field_name,
                                file_path = %rendered_path,
                                error = %err,
                                "Failed to read multipart file"
                            );
                            field_summaries.push(format!(
                                "{}=<file:{rendered_path} read_error={err}>",
                                field_name
                            ));
                        }
                    }
                }
            }
            let summary = format!("Multipart [{}]", field_summaries.join(", "));
            request_builder = request_builder.multipart(form);
            summary
        }
        BodyType::GraphQL { query, variables } => {
            let rendered_query = active_context.render(query);
            let rendered_vars = active_context.render(variables);
            let vars_json: serde_json::Value = if rendered_vars.trim().is_empty() {
                serde_json::json!({})
            } else {
                serde_json::from_str(&rendered_vars).unwrap_or_else(|_| serde_json::json!({}))
            };
            let payload = serde_json::json!({
                "query": rendered_query,
                "variables": vars_json,
            })
            .to_string();
            let summary = format!(
                "GraphQL ({} bytes): {}",
                payload.len(),
                truncate_for_log(&payload, MAX_LOG_PAYLOAD_LEN)
            );
            if !header_pairs
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            {
                header_pairs.push(("Content-Type".to_string(), "application/json".to_string()));
            }
            body_bytes_for_signing = payload.as_bytes().to_vec();
            request_builder = request_builder.body(payload);
            summary
        }
        BodyType::None => "None".to_string(),
    };

    // Perform AWS SigV4 signing if configured
    if let AuthConfig::AwsSigV4 {
        use_profile,
        profile,
        credentials_path,
        access_key,
        secret_key,
        region,
        service,
        session_token,
    } = &rendered_auth
    {
        let creds = if *use_profile {
            resolve_aws_profile_credentials(
                profile,
                credentials_path.as_deref(),
                region,
                service,
            )?
        } else {
            AwsSigV4Credentials {
                access_key: access_key.clone(),
                secret_key: secret_key.clone(),
                session_token: session_token.clone(),
                region: if region.trim().is_empty() {
                    "us-east-1".to_string()
                } else {
                    region.clone()
                },
                service: if service.trim().is_empty() {
                    "execute-api".to_string()
                } else {
                    service.clone()
                },
            }
        };
        let mut full_url = url::Url::parse(&rendered_url)
            .map_err(|e| anyhow!("Invalid URL '{rendered_url}' for AWS SigV4: {e}"))?;
        if !url_params.is_empty() {
            let mut qp = full_url.query_pairs_mut();
            for (k, v) in &url_params {
                qp.append_pair(k, v);
            }
        }
        sign_aws_sigv4(
            req_method.as_str(),
            full_url.as_str(),
            &mut header_pairs,
            &body_bytes_for_signing,
            &creds,
            chrono::Utc::now(),
        )?;
    }

    let query_summary = format_kv_for_log(&url_params, false);
    let headers_summary = format_kv_for_log(&header_pairs, false);

    tracing::info!(
        target: "aura_engine::request",
        method = %req.method,
        url = %rendered_url,
        query_params = %query_summary,
        headers = %headers_summary,
        payload = %payload_summary,
        "Dispatching HTTP request"
    );

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
                Err(e) => {
                    let msg = format!("{e:#}");
                    tracing::error!(
                        target: "aura_engine::scripting",
                        stage = "post_response",
                        method = %req.method,
                        url = %rendered_url,
                        error = %msg,
                        "Post-response script failed"
                    );
                    all_console_logs.push(aura_core::model::ConsoleMessage {
                        level: "ERROR".to_string(),
                        message: format!("Post-response script error: {msg}"),
                        timestamp: chrono::Utc::now(),
                    });
                    script_errors.push(aura_core::model::ScriptError {
                        stage: "Post-response Script".to_string(),
                        message: msg,
                    });
                }
            }
        }

        if !req.scripts.tests.is_empty() {
            match script_rt.run(&req.scripts.tests) {
                Ok(test_res) => {
                    all_console_logs.extend(test_res.console_logs);
                    assertions.extend(test_res.test_assertions);
                }
                Err(e) => {
                    let msg = format!("{e:#}");
                    tracing::error!(
                        target: "aura_engine::scripting",
                        stage = "tests",
                        method = %req.method,
                        url = %rendered_url,
                        error = %msg,
                        "Test script failed"
                    );
                    all_console_logs.push(aura_core::model::ConsoleMessage {
                        level: "ERROR".to_string(),
                        message: format!("Test script error: {msg}"),
                        timestamp: chrono::Utc::now(),
                    });
                    assertions.push(aura_core::model::TestAssertion {
                        name: "Test Script Execution".to_string(),
                        passed: false,
                        message: Some(msg.clone()),
                        duration_ms: 0,
                    });
                    script_errors.push(aura_core::model::ScriptError {
                        stage: "Test Script".to_string(),
                        message: msg,
                    });
                }
            }
        }
    }

    let resp_headers_pairs: Vec<(String, String)> = resp_headers
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let resp_headers_summary = format_kv_for_log(&resp_headers_pairs, false);
    let resp_body_summary = truncate_for_log(&body_str, MAX_LOG_PAYLOAD_LEN);

    tracing::info!(
        target: "aura_engine::response",
        method = %req.method,
        url = %rendered_url,
        status = status,
        status_text = %status_text,
        duration_ms = duration_ms,
        size_bytes = size_bytes,
        headers = %resp_headers_summary,
        body = %resp_body_summary,
        assertions_count = assertions.len(),
        assertions_failed = assertions.iter().filter(|a| !a.passed).count(),
        script_errors = script_errors.len(),
        "Received HTTP response"
    );

    let is_sse = resp_headers
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("content-type") && v.to_ascii_lowercase().contains("text/event-stream"));
    let sse_events: Vec<SseEvent> = if is_sse {
        let mut parser = crate::sse::SseStreamParser::new();
        let mut evs = parser.push_chunk(&body_str);
        evs.extend(parser.finish());
        evs
    } else {
        Vec::new()
    };

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
        script_errors,
        sse_events,
        stream_messages: Vec::new(),
    })
}

pub async fn fetch_graphql_introspection(
    req: &AuraRequest,
    context: &TemplateContext,
) -> Result<GraphQLSchema> {
    let mut introspect_req = req.clone();
    introspect_req.method = HttpMethod::POST;
    introspect_req.protocol = RequestProtocol::Graphql;
    introspect_req.scripts = Default::default();
    introspect_req.body = BodyType::GraphQL {
        query: INTROSPECTION_QUERY.to_string(),
        variables: "{}".to_string(),
    };
    let resp = execute_request(&introspect_req, context).await?;
    parse_introspection_response(&resp.body)
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
