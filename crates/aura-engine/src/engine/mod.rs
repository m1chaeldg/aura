//! Core HTTP Request Dispatcher and Execution Engine.

mod auth_render;
mod body_builder;
mod logging;
mod script_runner;

use anyhow::{anyhow, Result};
use aura_core::cookies::{CookieEntry, CookieJar};
use aura_core::graphql::{parse_introspection_response, GraphQLSchema, INTROSPECTION_QUERY};
use aura_core::grpc::parse_proto_file;
use aura_core::interceptor::InterceptorManager;
use aura_core::model::{
    AuthConfig, AuraRequest, AuraResponse, BodyType, ClientCertificateConfig, HttpMethod,
    ProxyConfig, RequestProtocol, SseEvent,
};
use aura_core::templating::TemplateContext;
use auth_render::{apply_rendered_auth, maybe_handle_auth_challenge, render_auth_config};
use body_builder::attach_request_body;
use logging::{MAX_LOG_PAYLOAD_LEN, describe_reqwest_error, format_kv_for_log, truncate_for_log};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use script_runner::{build_script_http_handler, run_post_and_test_scripts, run_pre_request_script};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;

/// Workspace-level execution options (mTLS certs, secret vaults, global proxy, cookie jar, interceptor).
#[derive(Debug, Clone, Default)]
pub struct ExecutionOptions {
    pub domain_certs: Vec<ClientCertificateConfig>,
    pub vault_config: crate::vaults::VaultConfig,
    pub global_proxy: Option<ProxyConfig>,
    pub cookie_jar: CookieJar,
    pub interceptor: Option<InterceptorManager>,
}

/// Execute an `AuraRequest` with default options.
pub async fn execute_request(req: &AuraRequest, context: &TemplateContext) -> Result<AuraResponse> {
    let mut cloned = req.clone();
    execute_request_with_options(&mut cloned, context, &ExecutionOptions::default()).await
}

/// Execute an `AuraRequest` with full lifecycle, vault secret resolution,
/// OAuth 2.0 auto-refresh, mTLS client certificates, Digest/NTLM retries, and scripting.
pub async fn execute_request_with_options(
    req: &mut AuraRequest,
    context: &TemplateContext,
    options: &ExecutionOptions,
) -> Result<AuraResponse> {
    let mut all_console_logs = Vec::new();
    let mut script_errors = Vec::new();

    let mut resolved_ctx = context.clone();
    crate::vaults::resolve_template_secrets(
        &mut resolved_ctx,
        &options.vault_config,
        req.settings.verify_ssl,
    )
    .await;

    let mut working_vars = collect_working_vars(req, &resolved_ctx);
    let script_http_handler = build_script_http_handler(req.settings.verify_ssl);
    let mut next_request_override: Option<String> = None;
    let mut stop_chain_override = false;

    if !req.scripts.pre_request.is_empty() {
        match run_pre_request_script(
            req.scripts.pre_request.clone(),
            working_vars.clone(),
            script_http_handler.clone(),
        )
        .await?
        {
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
            Err(msg) => {
                return Ok(pre_script_error_response(
                    req,
                    msg,
                    all_console_logs,
                    script_errors,
                    working_vars,
                ));
            }
        }
    }

    let mut active_context = resolved_ctx;
    active_context.request_vars = working_vars.clone();

    maybe_refresh_oauth2(req, &active_context).await;

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

    let mut rendered_url = active_context.render(&req.url);
    let mut url_params: Vec<(String, String)> = req
        .params
        .iter()
        .filter(|p| p.enabled)
        .map(|p| (active_context.render(&p.key), active_context.render(&p.value)))
        .collect();
    let mut header_pairs: Vec<(String, String)> = req
        .headers
        .iter()
        .filter(|h| h.enabled)
        .map(|h| (active_context.render(&h.key), active_context.render(&h.value)))
        .collect();

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
    let (client, phase_tunnel, dns_lookup_ms) =
        build_http_client(req, &active_context, options, &rendered_url).await?;

    let req_method = to_reqwest_method(req.method);
    let is_multipart = matches!(req.body, BodyType::Multipart(_));
    let (mut request_builder, body_bytes_for_signing, payload_summary) = attach_request_body(
        &req.body,
        &active_context,
        client.request(req_method.clone(), &rendered_url),
        &mut header_pairs,
    );

    apply_rendered_auth(
        &req_method,
        &rendered_url,
        &rendered_auth,
        &body_bytes_for_signing,
        &mut header_pairs,
        &mut url_params,
    )?;

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

    let header_map = build_header_map(&header_pairs);
    request_builder = request_builder.headers(header_map.clone());

    let start_time = Instant::now();
    let resp = request_builder.send().await.map_err(|e| {
        let elapsed_ms = start_time.elapsed().as_millis() as u64;
        let details =
            describe_reqwest_error(&e, req.settings.timeout_ms, req.settings.max_redirects);
        tracing::error!(
            target: "aura_engine",
            method = %req.method,
            url = %rendered_url,
            error_kind = details.kind,
            duration_ms = elapsed_ms,
            cause_chain = %details.cause_chain,
            possible_cause = %details.possible_cause,
            "HTTP network error encountered"
        );
        anyhow!(
            "Network error ({}): {} | Possible cause: {}",
            details.kind,
            details.cause_chain,
            details.possible_cause
        )
    })?;

    let resp = maybe_handle_auth_challenge(
        &client,
        &req_method,
        &rendered_url,
        &rendered_auth,
        &header_map,
        &url_params,
        &body_bytes_for_signing,
        is_multipart,
        resp,
    )
    .await?;

    let send_elapsed_ms = start_time.elapsed().as_millis() as u64;
    let final_url = resp.url().to_string();
    let status = resp.status().as_u16();
    let status_text = resp.status().canonical_reason().unwrap_or("").to_string();

    let set_cookie_raws: Vec<String> = resp
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(ToString::to_string)
        .collect();
    let mut jar_copy = options.cookie_jar.clone();
    let captured_cookies: Vec<CookieEntry> =
        jar_copy.apply_set_cookie_headers(&final_url, &set_cookie_raws);

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
        phase_tunnel.as_ref(),
        send_elapsed_ms,
        content_download_ms,
        duration_ms,
    );
    drop(phase_tunnel);

    let size_bytes = body_bytes.len();
    let body_str = String::from_utf8_lossy(&body_bytes).to_string();

    let mut assertions = Vec::new();
    let mut exported_vars = working_vars.clone();

    if !req.scripts.post_response.is_empty() || !req.scripts.tests.is_empty() {
        let stage_out = run_post_and_test_scripts(
            req.scripts.post_response.clone(),
            req.scripts.tests.clone(),
            working_vars,
            script_http_handler,
            status,
            body_str.clone(),
            resp_headers.clone(),
            duration_ms,
            req.method.as_str(),
            &rendered_url,
            next_request_override,
            stop_chain_override,
        )
        .await?;
        if !stage_out.variables.is_empty() {
            exported_vars = stage_out.variables;
        }
        all_console_logs.extend(stage_out.console_logs);
        script_errors.extend(stage_out.script_errors);
        assertions.extend(stage_out.assertions);
        next_request_override = stage_out.next_request;
        stop_chain_override = stage_out.stop_chain;
    }

    let is_sse = resp_headers.iter().any(|(k, v)| {
        k.eq_ignore_ascii_case("content-type")
            && v.to_ascii_lowercase().contains("text/event-stream")
    });
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
        headers: resp_headers,
        body: body_str,
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

    if !req.extractors.is_empty() {
        let native_vars =
            aura_core::extractors::evaluate_native_extractors(&req.extractors, &final_resp);
        for (k, v) in native_vars {
            final_resp.exported_variables.entry(k).or_insert(v);
        }
    }

    if let Some(ref flow) = req.flow {
        let perf_cfg = aura_core::assertions::PerformanceAssertionConfig {
            max_duration_ms: flow.max_duration_ms,
            min_size_bytes: flow.min_size_bytes,
            max_size_bytes: flow.max_size_bytes,
        };
        if !perf_cfg.is_empty() {
            let mut perf_res =
                aura_core::assertions::evaluate_performance_assertions(&perf_cfg, &final_resp);
            final_resp.assertions.append(&mut perf_res);
        }
    }

    if !req.scripts.assertions.is_empty() {
        let mut decl =
            aura_core::assertions::evaluate_assertions(&req.scripts.assertions, &final_resp);
        decl.append(&mut final_resp.assertions);
        final_resp.assertions = decl;
    }

    let resp_headers_pairs: Vec<(String, String)> = final_resp
        .headers
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    tracing::info!(
        target: "aura_engine::response",
        method = %req.method,
        url = %rendered_url,
        status = status,
        duration_ms = duration_ms,
        size_bytes = size_bytes,
        headers = %format_kv_for_log(&resp_headers_pairs, false),
        body = %truncate_for_log(&final_resp.body, MAX_LOG_PAYLOAD_LEN),
        assertions_count = final_resp.assertions.len(),
        assertions_failed = final_resp.assertions.iter().filter(|a| !a.passed).count(),
        "Received HTTP response"
    );

    Ok(final_resp)
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

fn collect_working_vars(
    req: &AuraRequest,
    resolved_ctx: &TemplateContext,
) -> HashMap<String, String> {
    let mut vars = resolved_ctx.request_vars.clone();
    for (k, v) in &req.variables {
        vars.entry(k.clone()).or_insert_with(|| v.clone());
    }
    for (k, v) in &resolved_ctx.environment_vars {
        vars.entry(k.clone()).or_insert_with(|| v.clone());
    }
    for (k, v) in &resolved_ctx.collection_vars {
        vars.entry(k.clone()).or_insert_with(|| v.clone());
    }
    vars
}

fn pre_script_error_response(
    req: &AuraRequest,
    msg: String,
    mut console_logs: Vec<aura_core::model::ConsoleMessage>,
    mut script_errors: Vec<aura_core::model::ScriptError>,
    working_vars: HashMap<String, String>,
) -> AuraResponse {
    tracing::error!(
        target: "aura_engine::scripting",
        stage = "pre_request",
        method = %req.method,
        url = %req.url,
        error = %msg,
        "Pre-request script failed"
    );
    console_logs.push(aura_core::model::ConsoleMessage {
        level: "ERROR".to_string(),
        message: format!("Pre-request script error: {msg}"),
        timestamp: chrono::Utc::now(),
    });
    script_errors.push(aura_core::model::ScriptError {
        stage: "Pre-request Script".to_string(),
        message: msg,
    });
    AuraResponse {
        status: 0,
        status_text: "Pre-request Script Error".to_string(),
        duration_ms: 0,
        size_bytes: 0,
        headers: HashMap::new(),
        body: String::new(),
        timestamp: chrono::Utc::now(),
        assertions: Vec::new(),
        console_logs,
        script_errors,
        sse_events: Vec::new(),
        stream_messages: Vec::new(),
        timing: None,
        captured_cookies: Vec::new(),
        next_request: None,
        stop_chain: false,
        exported_variables: working_vars,
    }
}

async fn maybe_refresh_oauth2(req: &mut AuraRequest, active_context: &TemplateContext) {
    if !matches!(req.auth, AuthConfig::OAuth2 { .. }) {
        return;
    }
    let mut rendered_oauth = render_auth_config(&req.auth, active_context);
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

async fn build_http_client(
    req: &AuraRequest,
    active_context: &TemplateContext,
    options: &ExecutionOptions,
    rendered_url: &str,
) -> Result<(
    reqwest::Client,
    Option<crate::proxy::ActiveTunnel>,
    u64,
)> {
    let mut client_builder = reqwest::Client::builder()
        .danger_accept_invalid_certs(!req.settings.verify_ssl);
    if req.settings.timeout_ms > 0 {
        client_builder =
            client_builder.timeout(std::time::Duration::from_millis(req.settings.timeout_ms));
    }
    if let Some(matched_cert) = crate::mtls::resolve_client_cert_for_url(
        rendered_url,
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
        rendered_url,
        rendered_req_proxy.as_ref(),
        options.global_proxy.as_ref(),
    );
    let (client_builder, phase_tunnel, dns_lookup_ms) =
        crate::proxy::configure_proxy_and_timing(client_builder, rendered_url, matched_proxy)
            .await?;

    let client_builder = if req.settings.follow_redirects {
        client_builder.redirect(reqwest::redirect::Policy::limited(
            req.settings.max_redirects,
        ))
    } else {
        client_builder.redirect(reqwest::redirect::Policy::none())
    };

    Ok((client_builder.build()?, phase_tunnel, dns_lookup_ms))
}

fn to_reqwest_method(method: HttpMethod) -> reqwest::Method {
    match method {
        HttpMethod::GET => reqwest::Method::GET,
        HttpMethod::POST => reqwest::Method::POST,
        HttpMethod::PUT => reqwest::Method::PUT,
        HttpMethod::PATCH => reqwest::Method::PATCH,
        HttpMethod::DELETE => reqwest::Method::DELETE,
        HttpMethod::HEAD => reqwest::Method::HEAD,
        HttpMethod::OPTIONS => reqwest::Method::OPTIONS,
    }
}

fn build_header_map(header_pairs: &[(String, String)]) -> HeaderMap {
    let mut header_map = HeaderMap::new();
    for (k, v) in header_pairs {
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
    header_map
}
