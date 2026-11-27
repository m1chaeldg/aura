//! Core HTTP Request Dispatcher and Execution Engine.

use anyhow::{anyhow, Result};
use aura_core::graphql::{parse_introspection_response, GraphQLSchema, INTROSPECTION_QUERY};
use aura_core::grpc::parse_proto_file;
use aura_core::cookies::{CookieEntry, CookieJar};
use aura_core::interceptor::InterceptorManager;
use aura_core::model::{
    AuthConfig, AuraRequest, AuraResponse, BodyType, ClientCertificateConfig, HttpMethod,
    ProxyConfig, RequestProtocol, SseEvent,
};
use aura_core::security::{
    apply_request_auth, resolve_aws_profile_credentials, sign_aws_sigv4, AwsSigV4Credentials,
};
use aura_core::security_schemes::{
    build_digest_authorization, build_ntlm_type1_negotiate, build_ntlm_type3_authenticate,
    parse_ntlm_type2_challenge, DigestChallenge,
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
            auth_url,
            redirect_uri,
            scope,
            username,
            password,
            refresh_token,
            expires_at,
            use_pkce,
            auto_refresh,
        } => AuthConfig::OAuth2 {
            grant_type: ctx.render(grant_type),
            access_token: ctx.render(access_token),
            token_url: token_url.as_ref().map(|u| ctx.render(u)),
            client_id: client_id.as_ref().map(|c| ctx.render(c)),
            client_secret: client_secret.as_ref().map(|s| ctx.render(s)),
            auth_url: auth_url.as_ref().map(|u| ctx.render(u)),
            redirect_uri: redirect_uri.as_ref().map(|u| ctx.render(u)),
            scope: scope.as_ref().map(|s| ctx.render(s)),
            username: username.as_ref().map(|s| ctx.render(s)),
            password: password.as_ref().map(|s| ctx.render(s)),
            refresh_token: refresh_token.as_ref().map(|s| ctx.render(s)),
            expires_at: *expires_at,
            use_pkce: *use_pkce,
            auto_refresh: *auto_refresh,
        },
        AuthConfig::Digest {
            username,
            password,
            realm,
            nonce,
            qop,
            algorithm,
        } => AuthConfig::Digest {
            username: ctx.render(username),
            password: ctx.render(password),
            realm: realm.as_ref().map(|s| ctx.render(s)),
            nonce: nonce.as_ref().map(|s| ctx.render(s)),
            qop: qop.as_ref().map(|s| ctx.render(s)),
            algorithm: algorithm.as_ref().map(|s| ctx.render(s)),
        },
        AuthConfig::Ntlm {
            username,
            password,
            domain,
            workstation,
        } => AuthConfig::Ntlm {
            username: ctx.render(username),
            password: ctx.render(password),
            domain: ctx.render(domain),
            workstation: ctx.render(workstation),
        },
        AuthConfig::OAuth1 {
            consumer_key,
            consumer_secret,
            token,
            token_secret,
            signature_method,
            callback_url,
            realm,
            add_to_query,
        } => AuthConfig::OAuth1 {
            consumer_key: ctx.render(consumer_key),
            consumer_secret: ctx.render(consumer_secret),
            token: ctx.render(token),
            token_secret: ctx.render(token_secret),
            signature_method: ctx.render(signature_method),
            callback_url: callback_url.as_ref().map(|s| ctx.render(s)),
            realm: realm.as_ref().map(|s| ctx.render(s)),
            add_to_query: *add_to_query,
        },
        AuthConfig::Hawk {
            id,
            key,
            algorithm,
            ext,
            app,
            dlg,
        } => AuthConfig::Hawk {
            id: ctx.render(id),
            key: ctx.render(key),
            algorithm: ctx.render(algorithm),
            ext: ext.as_ref().map(|s| ctx.render(s)),
            app: app.as_ref().map(|s| ctx.render(s)),
            dlg: dlg.as_ref().map(|s| ctx.render(s)),
        },
        AuthConfig::AkamaiEdgeGrid {
            client_token,
            client_secret,
            access_token,
            max_body,
        } => AuthConfig::AkamaiEdgeGrid {
            client_token: ctx.render(client_token),
            client_secret: ctx.render(client_secret),
            access_token: ctx.render(access_token),
            max_body: *max_body,
        },
        AuthConfig::Wsse {
            username,
            password,
            digest_password,
        } => AuthConfig::Wsse {
            username: ctx.render(username),
            password: ctx.render(password),
            digest_password: *digest_password,
        },
    }
}

/// Workspace-level execution options (mTLS certs, secret vaults, global proxy, cookie jar, interceptor).
#[derive(Debug, Clone, Default)]
pub struct ExecutionOptions {
    pub domain_certs: Vec<ClientCertificateConfig>,
    pub vault_config: crate::vaults::VaultConfig,
    pub global_proxy: Option<ProxyConfig>,
    pub cookie_jar: CookieJar,
    pub interceptor: Option<InterceptorManager>,
}

/// Execute an AuraRequest with default options.
pub async fn execute_request(req: &AuraRequest, context: &TemplateContext) -> Result<AuraResponse> {
    let mut cloned = req.clone();
    execute_request_with_options(&mut cloned, context, &ExecutionOptions::default()).await
}

/// Execute an AuraRequest with full lifecycle, vault secret resolution,
/// OAuth 2.0 auto-refresh (persisted back into `req.auth`), mTLS client certificates,
/// Digest/NTLM challenge-response retries, and post-response scripting.
pub async fn execute_request_with_options(
    req: &mut AuraRequest,
    context: &TemplateContext,
    options: &ExecutionOptions,
) -> Result<AuraResponse> {
    let mut all_console_logs = Vec::new();
    let mut script_errors = Vec::new();

    // 0. Resolve secret URIs (keyring://, vault://, aws-sm://) into a cloned context
    let mut resolved_ctx = context.clone();
    crate::vaults::resolve_template_secrets(
        &mut resolved_ctx,
        &options.vault_config,
        req.settings.verify_ssl,
    )
    .await;

    let mut working_vars = resolved_ctx.request_vars.clone();
    for (k, v) in &req.variables {
        working_vars.entry(k.clone()).or_insert_with(|| v.clone());
    }
    for (k, v) in &resolved_ctx.environment_vars {
        working_vars.entry(k.clone()).or_insert_with(|| v.clone());
    }
    for (k, v) in &resolved_ctx.collection_vars {
        working_vars.entry(k.clone()).or_insert_with(|| v.clone());
    }

    let script_http_handler = build_script_http_handler(req.settings.verify_ssl);
    let mut next_request_override: Option<String> = None;
    let mut stop_chain_override = false;

    // 1. Run Pre-Request script if present (supports async fetch / sendRequest & setNextRequest)
    if !req.scripts.pre_request.is_empty() {
        let vars_in = working_vars.clone();
        let script_src = req.scripts.pre_request.clone();
        let http_h = std::sync::Arc::clone(&script_http_handler);
        let run_res = tokio::task::spawn_blocking(move || -> Result<aura_scripting::ScriptExecutionResult> {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(async move {
                let mut script_rt = AuraScriptRuntime::with_http_handler(vars_in, http_h)?;
                script_rt.run_async(&script_src).await
            })
        })
        .await
        .map_err(|e| anyhow!("Pre-request script worker failed: {e}"))?;

        match run_res {
            Ok(res) => {
                working_vars = res.variables;
                all_console_logs.extend(res.console_logs);
                if res.stop_chain {
                    stop_chain_override = true;
                    next_request_override = None;
                } else if res.next_request.is_some() {
                    next_request_override = res.next_request;
                    stop_chain_override = false;
                }
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
                    timing: None,
                    captured_cookies: Vec::new(),
                    next_request: None,
                    stop_chain: false,
                    exported_variables: working_vars,
                });
            }
        }
    }

    // Update context with potentially mutated variables
    let mut active_context = resolved_ctx;
    active_context.request_vars = working_vars.clone();

    // Auto-refresh OAuth 2.0 token when enabled and expired/missing, persisting updated tokens to req.auth
    if matches!(req.auth, AuthConfig::OAuth2 { .. }) {
        let mut rendered_oauth = render_auth_config(&req.auth, &active_context);
        if crate::oauth2::maybe_refresh_oauth2_auth(&mut rendered_oauth, req.settings.verify_ssl)
            .await
            .unwrap_or(false)
        {
            if let (
                AuthConfig::OAuth2 {
                    access_token: dst_tok,
                    refresh_token: dst_rt,
                    expires_at: dst_exp,
                    ..
                },
                AuthConfig::OAuth2 {
                    access_token: src_tok,
                    refresh_token: src_rt,
                    expires_at: src_exp,
                    ..
                },
            ) = (&mut req.auth, &rendered_oauth)
            {
                *dst_tok = src_tok.clone();
                if src_rt.is_some() {
                    *dst_rt = src_rt.clone();
                }
                *dst_exp = *src_exp;
            }
        }
    }

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

    // 2b. Optional interceptor rules + automatic Cookie Jar injection
    let mut rendered_url = rendered_url;
    if let Some(ref mgr) = options.interceptor {
        let mut hmap: HashMap<String, String> = header_pairs.iter().cloned().collect();
        rendered_url = mgr.apply_rewrites(&rendered_url, &mut hmap);
        header_pairs = hmap.into_iter().collect();
    }
    if !header_pairs
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case("cookie"))
    {
        if let Some(cookie_hdr) = options.cookie_jar.cookie_header_for_url(&rendered_url) {
            header_pairs.push(("Cookie".to_string(), cookie_hdr));
        }
    }

    let rendered_auth = render_auth_config(&req.auth, &active_context);

    // 4. Build Reqwest Client (timeout_ms == 0 means unlimited / no timeout) + mTLS + Proxy + Timing
    let mut client_builder = reqwest::Client::builder()
        .danger_accept_invalid_certs(!req.settings.verify_ssl);
    if req.settings.timeout_ms > 0 {
        client_builder =
            client_builder.timeout(std::time::Duration::from_millis(req.settings.timeout_ms));
    }
    if let Some(matched_cert) = crate::mtls::resolve_client_cert_for_url(
        &rendered_url,
        req.settings.client_cert.as_ref(),
        &options.domain_certs,
    ) {
        let rendered_cert = ClientCertificateConfig {
            enabled: matched_cert.enabled,
            domain: matched_cert.domain.clone(),
            cert_path: active_context.render(&matched_cert.cert_path),
            key_path: matched_cert.key_path.as_ref().map(|k| active_context.render(k)),
            passphrase: matched_cert
                .passphrase
                .as_ref()
                .map(|p| active_context.render(p)),
        };
        let identity = crate::mtls::build_reqwest_identity(&rendered_cert)?;
        client_builder = client_builder.identity(identity);
    }

    let rendered_req_proxy = req.settings.proxy.as_ref().map(|p| ProxyConfig {
        enabled: p.enabled,
        protocol: p.protocol,
        host: active_context.render(&p.host),
        port: p.port,
        auth_enabled: p.auth_enabled,
        username: active_context.render(&p.username),
        password: active_context.render(&p.password),
        bypass_list: p
            .bypass_list
            .iter()
            .map(|b| active_context.render(b))
            .collect(),
    });
    let matched_proxy = crate::proxy::resolve_proxy_for_url(
        &rendered_url,
        rendered_req_proxy.as_ref(),
        options.global_proxy.as_ref(),
    );
    let (client_builder, _phase_tunnel, dns_lookup_ms) =
        crate::proxy::configure_proxy_and_timing(client_builder, &rendered_url, matched_proxy)
            .await?;

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

    // Add body and capture payload bytes + summary for signing and diagnostics
    let is_multipart = matches!(req.body, BodyType::Multipart(_));
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

    // Inject auth (OAuth1, Hawk, EdgeGrid, Digest static, WSSE, Bearer, Basic, ApiKey, OAuth2, NTLM Type 1)
    if !matches!(rendered_auth, AuthConfig::AwsSigV4 { .. }) {
        let mut full_url = url::Url::parse(&rendered_url)
            .map_err(|e| anyhow!("Invalid URL '{rendered_url}': {e}"))?;
        if !url_params.is_empty() {
            let mut qp = full_url.query_pairs_mut();
            for (k, v) in &url_params {
                qp.append_pair(k, v);
            }
        }
        apply_request_auth(
            req_method.as_str(),
            full_url.as_str(),
            &body_bytes_for_signing,
            &rendered_auth,
            &mut header_pairs,
            &mut url_params,
        )?;
        if matches!(rendered_auth, AuthConfig::Ntlm { .. }) {
            header_pairs.push(("Authorization".to_string(), build_ntlm_type1_negotiate()));
        }
    } else if let AuthConfig::AwsSigV4 {
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

    if !url_params.is_empty() {
        request_builder = request_builder.query(&url_params);
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
    for (k, v) in &header_pairs {
        match (HeaderName::from_str(k), HeaderValue::from_str(v)) {
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
    request_builder = request_builder.headers(header_map.clone());

    // Execute Request & measure exact duration
    let start_time = Instant::now();
    let mut resp = request_builder.send().await.map_err(|e| {
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

    // Digest / NTLM 401 challenge-response handshake
    if resp.status().as_u16() == 401 && !is_multipart {
        let www_auths: Vec<String> = resp
            .headers()
            .get_all(reqwest::header::WWW_AUTHENTICATE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .collect();

        let retry_auth_hdr: Option<String> = match &rendered_auth {
            AuthConfig::Digest {
                username, password, ..
            } => www_auths
                .iter()
                .find_map(|hdr| DigestChallenge::parse(hdr))
                .map(|challenge| {
                    let uri = url::Url::parse(&rendered_url)
                        .map(|u| match u.query() {
                            Some(q) => format!("{}?{q}", u.path()),
                            None => u.path().to_string(),
                        })
                        .unwrap_or_else(|_| "/".to_string());
                    build_digest_authorization(
                        req_method.as_str(),
                        &uri,
                        username,
                        password,
                        &challenge,
                    )
                }),
            AuthConfig::Ntlm {
                username,
                password,
                domain,
                workstation,
            } => www_auths
                .iter()
                .find_map(|hdr| parse_ntlm_type2_challenge(hdr).ok())
                .and_then(|type2| {
                    build_ntlm_type3_authenticate(username, password, domain, workstation, &type2)
                        .ok()
                }),
            _ => None,
        };

        if let Some(auth_val) = retry_auth_hdr {
            if let Ok(val) = HeaderValue::from_str(&auth_val) {
                // Consume initial 401 body so connection can be kept alive for NTLM/Digest
                let _ = resp.bytes().await;
                let mut retry_headers = header_map;
                retry_headers.insert(reqwest::header::AUTHORIZATION, val);
                let mut retry_builder = client
                    .request(req_method.clone(), &rendered_url)
                    .headers(retry_headers);
                if !url_params.is_empty() {
                    retry_builder = retry_builder.query(&url_params);
                }
                if !body_bytes_for_signing.is_empty() {
                    retry_builder = retry_builder.body(body_bytes_for_signing.clone());
                }
                resp = retry_builder.send().await.map_err(|e| {
                    anyhow!("Auth challenge-response retry failed: {e}")
                })?;
            }
        }
    }
    let send_elapsed_ms = start_time.elapsed().as_millis() as u64;
    let final_url = resp.url().to_string();

    let status = resp.status().as_u16();
    let status_text = resp.status().canonical_reason().unwrap_or("").to_string();

    let set_cookie_raws: Vec<String> = resp
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .collect();
    let mut jar_copy = options.cookie_jar.clone();
    let captured_cookies: Vec<CookieEntry> = jar_copy.apply_set_cookie_headers(&final_url, &set_cookie_raws);

    let mut resp_headers = HashMap::new();
    for (name, val) in resp.headers() {
        if let Ok(val_str) = val.to_str() {
            resp_headers.insert(name.as_str().to_string(), val_str.to_string());
        }
    }

    let dl_start = Instant::now();
    let body_bytes = resp.bytes().await.unwrap_or_default();
    let content_download_ms = dl_start.elapsed().as_millis() as u64;
    let duration_ms = dns_lookup_ms.saturating_add(start_time.elapsed().as_millis() as u64);
    let timing = crate::proxy::build_timing_breakdown(
        dns_lookup_ms,
        _phase_tunnel.as_ref(),
        send_elapsed_ms,
        content_download_ms,
        duration_ms,
    );
    drop(_phase_tunnel);

    let size_bytes = body_bytes.len();
    let body_str = String::from_utf8_lossy(&body_bytes).to_string();

    // 5. Post-response, Test assertions & Declarative Assert execution
    let mut assertions = Vec::new();
    let mut exported_vars = working_vars.clone();

    if !req.scripts.post_response.is_empty() || !req.scripts.tests.is_empty() {
        let vars_in = working_vars.clone();
        let post_src = req.scripts.post_response.clone();
        let test_src = req.scripts.tests.clone();
        let http_h = std::sync::Arc::clone(&script_http_handler);
        let resp_body_clone = body_str.clone();
        let resp_headers_clone = resp_headers.clone();

        #[allow(clippy::type_complexity)]
        let script_bundle = tokio::task::spawn_blocking(
            move || -> Result<(
                Option<Result<aura_scripting::ScriptExecutionResult>>,
                Option<Result<aura_scripting::ScriptExecutionResult>>,
            )> {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                rt.block_on(async move {
                    let mut script_rt = AuraScriptRuntime::with_http_handler(vars_in, http_h)?;
                    script_rt.inject_response_context_with_duration(
                        status,
                        &resp_body_clone,
                        &resp_headers_clone,
                        duration_ms,
                    )?;
                    let post_res = if !post_src.is_empty() {
                        Some(script_rt.run_async(&post_src).await)
                    } else {
                        None
                    };
                    let test_res = if !test_src.is_empty() {
                        Some(script_rt.run_async(&test_src).await)
                    } else {
                        None
                    };
                    Ok((post_res, test_res))
                })
            },
        )
        .await
        .map_err(|e| anyhow!("Post-response script worker failed: {e}"))??;

        if let Some(post_res) = script_bundle.0 {
            match post_res {
                Ok(res) => {
                    exported_vars = res.variables;
                    all_console_logs.extend(res.console_logs);
                    assertions.extend(res.test_assertions);
                    if res.stop_chain {
                        stop_chain_override = true;
                        next_request_override = None;
                    } else if res.next_request.is_some() {
                        next_request_override = res.next_request;
                        stop_chain_override = false;
                    }
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

        if let Some(test_res) = script_bundle.1 {
            match test_res {
                Ok(res) => {
                    exported_vars = res.variables;
                    all_console_logs.extend(res.console_logs);
                    assertions.extend(res.test_assertions);
                    if res.stop_chain {
                        stop_chain_override = true;
                        next_request_override = None;
                    } else if res.next_request.is_some() {
                        next_request_override = res.next_request;
                        stop_chain_override = false;
                    }
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

    let mut final_resp = AuraResponse {
        status,
        status_text,
        duration_ms,
        size_bytes,
        headers: resp_headers.clone(),
        body: body_str.clone(),
        timestamp: chrono::Utc::now(),
        assertions,
        console_logs: all_console_logs,
        script_errors,
        sse_events,
        stream_messages: Vec::new(),
        timing: Some(timing),
        captured_cookies,
        next_request: next_request_override,
        stop_chain: stop_chain_override,
        exported_variables: exported_vars,
    };

    if !req.scripts.assertions.is_empty() {
        let mut decl = aura_core::assertions::evaluate_assertions(&req.scripts.assertions, &final_resp);
        decl.append(&mut final_resp.assertions);
        final_resp.assertions = decl;
    }

    let resp_headers_pairs: Vec<(String, String)> = final_resp
        .headers
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let resp_headers_summary = format_kv_for_log(&resp_headers_pairs, false);
    let resp_body_summary = truncate_for_log(&final_resp.body, MAX_LOG_PAYLOAD_LEN);

    tracing::info!(
        target: "aura_engine::response",
        method = %req.method,
        url = %rendered_url,
        status = status,
        status_text = %final_resp.status_text,
        duration_ms = duration_ms,
        size_bytes = size_bytes,
        headers = %resp_headers_summary,
        body = %resp_body_summary,
        assertions_count = final_resp.assertions.len(),
        assertions_failed = final_resp.assertions.iter().filter(|a| !a.passed).count(),
        script_errors = final_resp.script_errors.len(),
        "Received HTTP response"
    );

    Ok(final_resp)
}

fn build_script_http_handler(verify_ssl: bool) -> aura_scripting::ScriptHttpHandler {
    std::sync::Arc::new(move |req: aura_scripting::ScriptHttpRequest| {
        Box::pin(async move {
            let client = reqwest::Client::builder()
                .danger_accept_invalid_certs(!verify_ssl)
                .timeout(std::time::Duration::from_secs(30))
                .build()?;
            let method = reqwest::Method::from_bytes(req.method.as_bytes())
                .unwrap_or(reqwest::Method::GET);
            let mut rb = client.request(method, &req.url);
            for (k, v) in &req.headers {
                rb = rb.header(k, v);
            }
            if let Some(body) = req.body {
                rb = rb.body(body);
            }
            let resp = rb.send().await?;
            let status = resp.status().as_u16();
            let status_text = resp.status().canonical_reason().unwrap_or("").to_string();
            let mut headers = HashMap::new();
            for (k, v) in resp.headers() {
                if let Ok(s) = v.to_str() {
                    headers.insert(k.as_str().to_string(), s.to_string());
                }
            }
            let body = resp.text().await.unwrap_or_default();
            Ok(aura_scripting::ScriptHttpResponse {
                ok: (200..300).contains(&status),
                status,
                status_text,
                headers,
                body,
                error: None,
            })
        })
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
