//! Offline Sandboxed JavaScript Scripting Runtime powered by deno_core.
//! Eradicates any telemetry or tracking. Scripts run completely offline.
//! Provides BDD `aura.test()` framework and console log capture.

use anyhow::{Context, Result};
use aura_core::model::TestAssertion;
use deno_core::{JsRuntime, RuntimeOptions};
use parking_lot::Mutex;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

pub use aura_core::model::ConsoleMessage;

#[derive(Debug, Clone, Default)]
pub struct ScriptExecutionResult {
    pub variables: HashMap<String, String>,
    pub console_logs: Vec<ConsoleMessage>,
    pub test_assertions: Vec<TestAssertion>,
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
        let options = RuntimeOptions {
            is_main: false,
            ..Default::default()
        };

        let runtime = JsRuntime::new(options);
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
                    assertions: _assertions
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

                // Aura BDD test and variable framework
                globalThis.aura = {{
                    variables: {{
                        get: function(key) {{ return _vars[key]; }},
                        set: function(key, val) {{ _vars[key] = String(val); }}
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

        // Link aura.expect to global expect
        let alias_script = r#"
            if (typeof globalThis.expect === 'function' && globalThis.aura) {
                globalThis.aura.expect = globalThis.expect;
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
        let headers_json = serde_json::to_string(headers)?;
        let escaped_body = serde_json::to_string(body)?;

        let script = format!(
            r#"
            (function() {{
                const _rawBody = {escaped_body};
                globalThis.response = {{
                    status: {status},
                    body: _rawBody,
                    headers: {headers_json},
                    json: function() {{
                        return JSON.parse(_rawBody);
                    }}
                }};
                globalThis.aura.response = globalThis.response;
            }})();
            "#
        );

        self.runtime
            .execute_script("aura_response_context.js", script)
            .context("Failed to inject response into deno_core runtime")?;

        Ok(())
    }

    /// Execute user script in sandbox
    pub fn run(&mut self, script: &str) -> Result<ScriptExecutionResult> {
        if script.trim().is_empty() {
            return Ok(ScriptExecutionResult {
                variables: self.variables.lock().clone(),
                console_logs: Vec::new(),
                test_assertions: Vec::new(),
            });
        }

        self.runtime
            .execute_script("user_script.js", script.to_string())
            .context("JavaScript execution error")?;

        // Drain async tests
        let drain_script = r#"
            (async function() {
                if (typeof globalThis.__aura_drain_tests === 'function') {
                    await globalThis.__aura_drain_tests();
                }
            })();
        "#;
        let _ = self.runtime.execute_script("aura_drain.js", drain_script.to_string());
        let _ = futures::executor::block_on(self.runtime.run_event_loop(Default::default()));

        // Extract mutated variables, console logs, and test assertions
        let extract_script = r#"
            JSON.stringify({
                vars: globalThis.__aura_internal.vars,
                logs: globalThis.__aura_internal.logs,
                assertions: globalThis.__aura_internal.assertions
            })
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
        })
    }
}
