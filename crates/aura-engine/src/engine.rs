//! Core HTTP Request Dispatcher and Execution Engine.

use aura_core::model::{AuraRequest, AuraResponse, BodyType, HttpMethod};
use aura_scripting::AuraScriptRuntime;
use aura_core::security::apply_auth;
use aura_core::templating::TemplateContext;
use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;

/// Execute an AuraRequest with full lifecycle:
/// 1. Pre-request scripting (deno_core)
/// 2. Templating & variable interpolation
/// 3. Auth injection
/// 4. HTTP dispatch & timing
/// 5. Post-response scripting & test assertion evaluation (deno_core)
pub async fn execute_request(req: &AuraRequest, context: &TemplateContext) -> Result<AuraResponse> {
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

    // Add query parameters
    if !url_params.is_empty() {
        request_builder = request_builder.query(&url_params);
    }

    // Add headers
    let mut header_map = HeaderMap::new();
    for (k, v) in header_pairs {
        if let (Ok(name), Ok(val)) = (HeaderName::from_str(&k), HeaderValue::from_str(&v)) {
            header_map.insert(name, val);
        }
    }
    request_builder = request_builder.headers(header_map);

    // Add body
    match &req.body {
        BodyType::Json(s) => {
            let rendered_body = active_context.render(s);
            request_builder = request_builder
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(rendered_body);
        }
        BodyType::Xml(s) => {
            let rendered_body = active_context.render(s);
            request_builder = request_builder
                .header(reqwest::header::CONTENT_TYPE, "application/xml")
                .body(rendered_body);
        }
        BodyType::Text(s) => {
            let rendered_body = active_context.render(s);
            request_builder = request_builder
                .header(reqwest::header::CONTENT_TYPE, "text/plain")
                .body(rendered_body);
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
            request_builder = request_builder.form(&form_data);
        }
        BodyType::Binary { relative_path } => {
            if let Ok(bytes) = std::fs::read(relative_path) {
                request_builder = request_builder.body(bytes);
            }
        }
        BodyType::Multipart(fields) => {
            let mut form = reqwest::multipart::Form::new();
            for f in fields {
                if f.enabled {
                    if let Some(v) = &f.value {
                        form = form.text(f.name.clone(), active_context.render(v));
                    }
                }
            }
            request_builder = request_builder.multipart(form);
        }
        BodyType::None => {}
    }

    // Execute Request & measure exact duration
    let start_time = Instant::now();
    let resp = request_builder
        .send()
        .await
        .map_err(|e| anyhow!("Network error: {e}"))?;
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
        script_rt.inject_response_context(status, &body_str, &resp_headers)?;

        if !req.scripts.post_response.is_empty() {
            let _ = script_rt.run(&req.scripts.post_response);
        }

        if !req.scripts.tests.is_empty() {
            if let Ok(test_res) = script_rt.run(&req.scripts.tests) {
                assertions = test_res.test_assertions;
            }
        }
    }

    Ok(AuraResponse {
        status,
        status_text,
        duration_ms,
        size_bytes,
        headers: resp_headers,
        body: body_str,
        timestamp: chrono::Utc::now(),
        assertions,
    })
}
