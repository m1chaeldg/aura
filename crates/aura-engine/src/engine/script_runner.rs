//! Sandboxed pre-request, post-response, and test script execution helpers.

use anyhow::{anyhow, Result};
use aura_core::model::{ConsoleMessage, ScriptError, TestAssertion};
use aura_scripting::{AuraScriptRuntime, ScriptExecutionResult, ScriptHttpHandler, ScriptHttpRequest, ScriptHttpResponse};
use std::collections::HashMap;

pub(super) struct ScriptStageOutput {
    pub(super) variables: HashMap<String, String>,
    pub(super) console_logs: Vec<ConsoleMessage>,
    pub(super) script_errors: Vec<ScriptError>,
    pub(super) assertions: Vec<TestAssertion>,
    pub(super) next_request: Option<String>,
    pub(super) stop_chain: bool,
}

pub(super) async fn run_pre_request_script(
    script_src: String,
    working_vars: HashMap<String, String>,
    http_handler: ScriptHttpHandler,
) -> Result<Result<ScriptExecutionResult, String>> {
    let run_res = tokio::task::spawn_blocking(move || -> Result<ScriptExecutionResult> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(async move {
            let mut script_rt = AuraScriptRuntime::with_http_handler(working_vars, http_handler)?;
            script_rt.run_async(&script_src).await
        })
    })
    .await
    .map_err(|e| anyhow!("Pre-request script worker failed: {e}"))?;

    Ok(run_res.map_err(|e| format!("{e:#}")))
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_post_and_test_scripts(
    post_src: String,
    test_src: String,
    working_vars: HashMap<String, String>,
    http_handler: ScriptHttpHandler,
    status: u16,
    body_str: String,
    resp_headers: HashMap<String, String>,
    duration_ms: u64,
    method: &str,
    rendered_url: &str,
    mut next_request_override: Option<String>,
    mut stop_chain_override: bool,
) -> Result<ScriptStageOutput> {
    #[allow(clippy::type_complexity)]
    let script_bundle = tokio::task::spawn_blocking(
        move || -> Result<(
            Option<Result<ScriptExecutionResult>>,
            Option<Result<ScriptExecutionResult>>,
        )> {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(async move {
                let mut script_rt = AuraScriptRuntime::with_http_handler(working_vars, http_handler)?;
                script_rt.inject_response_context_with_duration(
                    status,
                    &body_str,
                    &resp_headers,
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

    let mut exported_vars = HashMap::new();
    let mut console_logs = Vec::new();
    let mut script_errors = Vec::new();
    let mut assertions = Vec::new();

    for (stage_label, stage_key, is_test, maybe_res) in [
        ("Post-response Script", "post_response", false, script_bundle.0),
        ("Test Script", "tests", true, script_bundle.1),
    ] {
        let Some(res) = maybe_res else { continue };
        match res {
            Ok(out) => {
                exported_vars = out.variables;
                console_logs.extend(out.console_logs);
                assertions.extend(out.test_assertions);
                if out.stop_chain {
                    stop_chain_override = true;
                    next_request_override = None;
                } else if out.next_request.is_some() {
                    next_request_override = out.next_request;
                    stop_chain_override = false;
                }
            }
            Err(e) => {
                let msg = format!("{e:#}");
                tracing::error!(
                    target: "aura_engine::scripting",
                    stage = stage_key,
                    method = %method,
                    url = %rendered_url,
                    error = %msg,
                    "{stage_label} failed"
                );
                console_logs.push(ConsoleMessage {
                    level: "ERROR".to_string(),
                    message: format!("{stage_label} error: {msg}"),
                    timestamp: chrono::Utc::now(),
                });
                if is_test {
                    assertions.push(TestAssertion {
                        name: "Test Script Execution".to_string(),
                        passed: false,
                        message: Some(msg.clone()),
                        duration_ms: 0,
                    });
                }
                script_errors.push(ScriptError {
                    stage: stage_label.to_string(),
                    message: msg,
                });
            }
        }
    }

    Ok(ScriptStageOutput {
        variables: exported_vars,
        console_logs,
        script_errors,
        assertions,
        next_request: next_request_override,
        stop_chain: stop_chain_override,
    })
}

pub(super) fn build_script_http_handler(verify_ssl: bool) -> ScriptHttpHandler {
    std::sync::Arc::new(move |req: ScriptHttpRequest| {
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
            Ok(ScriptHttpResponse {
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
