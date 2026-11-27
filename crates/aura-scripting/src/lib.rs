//! Offline Sandboxed JavaScript Scripting Runtime powered by deno_core.
//! Eradicates any telemetry or tracking. Provides BDD `aura.test()`, `fetch()`,
//! `aura.sendRequest()`, `aura.setNextRequest()`, and console log capture.

use anyhow::{Context, Result};
use aura_core::model::TestAssertion;
use deno_core::{op2, Extension, JsRuntime, OpState, RuntimeOptions};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

pub use aura_core::model::ConsoleMessage;

#[derive(Debug, Clone, Deserialize)]
pub struct ScriptHttpRequest {
    pub url: String,
    #[serde(default = "default_http_method")]
    pub method: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
}

fn default_http_method() -> String {
    "GET".to_string()
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ScriptHttpResponse {
    pub ok: bool,
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    #[serde(default)]
    pub error: Option<String>,
}

pub type ScriptHttpHandler = Arc<
    dyn Fn(ScriptHttpRequest) -> Pin<Box<dyn Future<Output = Result<ScriptHttpResponse>> + Send>>
        + Send
        + Sync,
>;

#[op2(async)]
#[serde]
async fn op_aura_fetch(
    state: Rc<RefCell<OpState>>,
    #[serde] req: ScriptHttpRequest,
) -> ScriptHttpResponse {
    let handler_opt = {
        let borrowed = state.borrow();
        borrowed.try_borrow::<ScriptHttpHandler>().cloned()
    };

    let Some(handler) = handler_opt else {
        return ScriptHttpResponse {
            ok: false,
            status: 0,
            status_text: String::new(),
            headers: HashMap::new(),
            body: String::new(),
            error: Some("Auxiliary HTTP fetch handler is not enabled in this runtime".to_string()),
        };
    };

    // Execute future cleanly even when `run()` is invoked synchronously outside Tokio
    if tokio::runtime::Handle::try_current().is_ok() {
        match handler(req).await {
            Ok(resp) => resp,
            Err(e) => ScriptHttpResponse {
                ok: false,
                status: 0,
                status_text: String::new(),
                headers: HashMap::new(),
                body: String::new(),
                error: Some(format!("{e:#}")),
            },
        }
    } else {
        let join = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            match rt {
                Ok(rt) => rt.block_on(handler(req)),
                Err(e) => Err(anyhow::anyhow!("Failed to start HTTP runtime: {e}")),
            }
        });
        match join.join() {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => ScriptHttpResponse {
                ok: false,
                status: 0,
                status_text: String::new(),
                headers: HashMap::new(),
                body: String::new(),
                error: Some(format!("{e:#}")),
            },
            Err(_) => ScriptHttpResponse {
                ok: false,
                status: 0,
                status_text: String::new(),
                headers: HashMap::new(),
                body: String::new(),
                error: Some("HTTP fetch worker thread panicked".to_string()),
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ScriptExecutionResult {
    pub variables: HashMap<String, String>,
    pub console_logs: Vec<ConsoleMessage>,
    pub test_assertions: Vec<TestAssertion>,
    pub next_request: Option<String>,
    pub stop_chain: bool,
}

/// Script Runner wrapping a sandboxed deno_core JsRuntime
pub struct AuraScriptRuntime {
    runtime: JsRuntime,
    captured_logs: Arc<Mutex<Vec<ConsoleMessage>>>,
    variables: Arc<Mutex<HashMap<String, String>>>,
    assertions: Arc<Mutex<Vec<TestAssertion>>>,
}

const PRELUDE_JS: &str = include_str!("prelude.js");

impl AuraScriptRuntime {
    pub fn new(initial_vars: HashMap<String, String>) -> Result<Self> {
        Self::create_internal(initial_vars, None)
    }

    pub fn with_http_handler(
        initial_vars: HashMap<String, String>,
        http_handler: ScriptHttpHandler,
    ) -> Result<Self> {
        Self::create_internal(initial_vars, Some(http_handler))
    }

    fn create_internal(
        initial_vars: HashMap<String, String>,
        http_handler: Option<ScriptHttpHandler>,
    ) -> Result<Self> {
        let ext = Extension {
            name: "aura_runtime_ops",
            ops: std::borrow::Cow::Owned(vec![op_aura_fetch()]),
            ..Default::default()
        };

        let options = RuntimeOptions {
            is_main: false,
            extensions: vec![ext],
            ..Default::default()
        };

        let mut runtime = JsRuntime::new(options);
        if let Some(handler) = http_handler {
            runtime.op_state().borrow_mut().put(handler);
        }

        let captured_logs = Arc::new(Mutex::new(Vec::new()));
        let variables = Arc::new(Mutex::new(initial_vars));
        let assertions = Arc::new(Mutex::new(Vec::new()));

        let mut instance = Self {
            runtime,
            captured_logs,
            variables,
            assertions,
        };

        instance.bootstrap_sandbox()?;
        Ok(instance)
    }

    pub fn captured_logs(&self) -> Arc<Mutex<Vec<ConsoleMessage>>> {
        Arc::clone(&self.captured_logs)
    }

    pub fn assertions(&self) -> Arc<Mutex<Vec<TestAssertion>>> {
        Arc::clone(&self.assertions)
    }

    fn bootstrap_sandbox(&mut self) -> Result<()> {
        let init_vars_json = serde_json::to_string(&*self.variables.lock())?;

        // Injected JS environment with internal storage and console interceptor
        let init_script = format!(
            r#"
            (function() {{
                const _vars = {init_vars_json};
                const _logs = [];
                const _assertions = [];

                globalThis.__aura_internal = {{
                    vars: _vars,
                    logs: _logs,
                    assertions: _assertions,
                    nextRequest: null,
                    stopChain: false,
                    scriptError: null,
                    pendingFetches: []
                }};

                // Console interceptor
                globalThis.console = {{
                    log: function(...args) {{
                        _logs.push({{ level: "INFO", message: args.map(String).join(" "), timestamp: new Date().toISOString() }});
                    }},
                    warn: function(...args) {{
                        _logs.push({{ level: "WARN", message: args.map(String).join(" "), timestamp: new Date().toISOString() }});
                    }},
                    error: function(...args) {{
                        _logs.push({{ level: "ERROR", message: args.map(String).join(" "), timestamp: new Date().toISOString() }});
                    }}
                }};

                // Aura BDD test, variable, HTTP fetch/sendRequest, and chaining framework
                globalThis.aura = {{
                    variables: {{
                        get: function(key) {{ return _vars[key]; }},
                        set: function(key, val) {{ _vars[key] = String(val); }}
                    }},
                    setNextRequest: function(target) {{
                        if (target === null || target === undefined || String(target).trim() === "") {{
                            globalThis.__aura_internal.nextRequest = null;
                            globalThis.__aura_internal.stopChain = true;
                        }} else {{
                            globalThis.__aura_internal.nextRequest = String(target);
                            globalThis.__aura_internal.stopChain = false;
                        }}
                    }},
                    execution: {{
                        setNextRequest: function(target) {{
                            return globalThis.aura.setNextRequest(target);
                        }}
                    }},
                    test: function(name, fn) {{
                        if (typeof globalThis.it === 'function') {{
                            return globalThis.it(name, fn);
                        }}
                    }}
                }};
            }})();
            "#
        );

        self.runtime
            .execute_script("aura_sandbox_bootstrap.js", init_script)
            .context("Failed to bootstrap deno_core sandbox")?;

        self.runtime
            .execute_script("aura_prelude.js", PRELUDE_JS.to_string())
            .context("Failed to load prelude.js")?;

        // Link aura.expect to global expect & expose pm compatibility alias
        let alias_script = r#"
            if (typeof globalThis.expect === 'function' && globalThis.aura) {
                globalThis.aura.expect = globalThis.expect;
            }
            if (!globalThis.pm && globalThis.aura) {
                globalThis.pm = {
                    variables: globalThis.aura.variables,
                    environment: globalThis.aura.variables,
                    sendRequest: (...args) => globalThis.aura.sendRequest(...args),
                    setNextRequest: (target) => globalThis.aura.setNextRequest(target),
                    execution: globalThis.aura.execution,
                    test: (name, fn) => globalThis.aura.test(name, fn),
                    expect: globalThis.expect
                };
            }
        "#;
        self.runtime
            .execute_script("aura_alias.js", alias_script.to_string())?;

        Ok(())
    }

    /// Set response context before post-response or test execution
    pub fn inject_response_context(
        &mut self,
        status: u16,
        body: &str,
        headers: &HashMap<String, String>,
    ) -> Result<()> {
        self.inject_response_context_with_duration(status, body, headers, 0)
    }

    /// Set response context (including `duration_ms`) before post-response or test execution
    pub fn inject_response_context_with_duration(
        &mut self,
        status: u16,
        body: &str,
        headers: &HashMap<String, String>,
        duration_ms: u64,
    ) -> Result<()> {
        let headers_json = serde_json::to_string(headers)?;
        let escaped_body = serde_json::to_string(body)?;

        let script = format!(
            r#"
            (function() {{
                const _rawBody = {escaped_body};
                globalThis.response = {{
                    status: {status},
                    code: {status},
                    duration_ms: {duration_ms},
                    responseTime: {duration_ms},
                    body: _rawBody,
                    headers: {headers_json},
                    text: function() {{ return _rawBody; }},
                    json: function() {{
                        return JSON.parse(_rawBody);
                    }}
                }};
                globalThis.aura.response = globalThis.response;
                if (globalThis.pm) {{ globalThis.pm.response = globalThis.response; }}
            }})();
            "#
        );

        self.runtime
            .execute_script("aura_response_context.js", script)
            .context("Failed to inject response into deno_core runtime")?;

        Ok(())
    }

    fn prepare_and_drain_script(&mut self, script: &str) -> Result<()> {
        let wrapped = format!(
            r#"
            globalThis.__aura_internal.scriptError = null;
            (async function() {{
                try {{
                    await (async function() {{
{script}
                    }})();
                    if (typeof globalThis.__aura_drain_fetches === 'function') {{
                        await globalThis.__aura_drain_fetches();
                    }}
                    if (typeof globalThis.__aura_drain_tests === 'function') {{
                        await globalThis.__aura_drain_tests();
                    }}
                    if (typeof globalThis.__aura_drain_fetches === 'function') {{
                        await globalThis.__aura_drain_fetches();
                    }}
                }} catch (err) {{
                    globalThis.__aura_internal.scriptError = (err && err.stack)
                        ? String(err.stack)
                        : String(err);
                }}
            }})();
            "#
        );

        if let Err(err) = self.runtime.execute_script("user_script.js", wrapped) {
            return Err(anyhow::anyhow!("{err:#}"));
        }
        Ok(())
    }

    fn extract_result(&mut self) -> Result<ScriptExecutionResult> {
        let extract_script = r#"
            (function() {
                const payload = JSON.stringify({
                    vars: globalThis.__aura_internal.vars,
                    logs: globalThis.__aura_internal.logs,
                    assertions: globalThis.__aura_internal.assertions,
                    next_request: globalThis.__aura_internal.nextRequest,
                    stop_chain: Boolean(globalThis.__aura_internal.stopChain),
                    script_error: globalThis.__aura_internal.scriptError
                });
                globalThis.__aura_internal.logs.length = 0;
                globalThis.__aura_internal.assertions.length = 0;
                globalThis.__aura_internal.scriptError = null;
                return payload;
            })()
        "#;

        let result_val = self
            .runtime
            .execute_script("aura_extract.js", extract_script.to_string())?;

        let scope = &mut self.runtime.handle_scope();
        let local_val = deno_core::v8::Local::new(scope, result_val);
        let json_str = local_val.to_rust_string_lossy(scope);

        #[derive(Deserialize)]
        struct ExtractedData {
            vars: HashMap<String, String>,
            logs: Vec<ConsoleLogDto>,
            assertions: Vec<AssertionDto>,
            #[serde(default)]
            next_request: Option<String>,
            #[serde(default)]
            stop_chain: bool,
            #[serde(default)]
            script_error: Option<String>,
        }

        #[derive(Deserialize)]
        struct ConsoleLogDto {
            level: String,
            message: String,
        }

        #[derive(Deserialize)]
        struct AssertionDto {
            name: String,
            passed: bool,
            #[serde(alias = "error_message")]
            message: Option<String>,
            #[serde(default)]
            duration_ms: u64,
        }

        let extracted: ExtractedData =
            serde_json::from_str(&json_str).context("Failed to parse extracted script data")?;

        if let Some(err_msg) = extracted.script_error {
            return Err(anyhow::anyhow!("{err_msg}"));
        }

        *self.variables.lock() = extracted.vars.clone();

        let logs = extracted
            .logs
            .into_iter()
            .map(|l| ConsoleMessage {
                level: l.level,
                message: l.message,
                timestamp: chrono::Utc::now(),
            })
            .collect();

        let assertions = extracted
            .assertions
            .into_iter()
            .map(|a| TestAssertion {
                name: a.name,
                passed: a.passed,
                message: a.message,
                duration_ms: a.duration_ms,
            })
            .collect();

        Ok(ScriptExecutionResult {
            variables: extracted.vars,
            console_logs: logs,
            test_assertions: assertions,
            next_request: extracted.next_request,
            stop_chain: extracted.stop_chain,
        })
    }

    /// Execute user script synchronously (blocks current thread until promises/tests drain).
    pub fn run(&mut self, script: &str) -> Result<ScriptExecutionResult> {
        if script.trim().is_empty() {
            return Ok(ScriptExecutionResult {
                variables: self.variables.lock().clone(),
                ..Default::default()
            });
        }
        self.prepare_and_drain_script(script)?;
        let _ = futures::executor::block_on(self.runtime.run_event_loop(Default::default()));
        self.extract_result()
    }

    /// Execute user script asynchronously inside an active Tokio runtime so `fetch()`
    /// and `aura.sendRequest()` can run concurrent HTTP requests cleanly.
    pub async fn run_async(&mut self, script: &str) -> Result<ScriptExecutionResult> {
        if script.trim().is_empty() {
            return Ok(ScriptExecutionResult {
                variables: self.variables.lock().clone(),
                ..Default::default()
            });
        }
        self.prepare_and_drain_script(script)?;
        let _ = self.runtime.run_event_loop(Default::default()).await;
        self.extract_result()
    }
}
