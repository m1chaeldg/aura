//! Offline Sandboxed JavaScript Scripting Runtime powered by deno_core.
//! Eradicates any telemetry or tracking. Scripts run completely offline.
//! Provides BDD `aura.test()` framework and console log capture.

use aura_core::model::TestAssertion;
use anyhow::{Context, Result};
use deno_core::{JsRuntime, RuntimeOptions};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleMessage {
    pub level: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

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

        // Injected JS environment with console, aura, and BDD test suite
        let init_script = format!(
            r#"
            (function() {{
                const _vars = {init_vars_json};
                const _logs = [];
                const _assertions = [];

                globalThis.__AURA_VARS = _vars;
                globalThis.__AURA_LOGS = _logs;
                globalThis.__AURA_ASSERTIONS = _assertions;

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
                        const start = Date.now();
                        try {{
                            fn();
                            _assertions.push({{
                                name: name,
                                passed: true,
                                message: null,
                                duration_ms: Date.now() - start
                            }});
                        }} catch (err) {{
                            _assertions.push({{
                                name: name,
                                passed: false,
                                message: err && err.message ? err.message : String(err),
                                duration_ms: Date.now() - start
                            }});
                        }}
                    }},
                    expect: function(actual) {{
                        return {{
                            toBe: function(expected) {{
                                if (actual !== expected) {{
                                    throw new Error(`Expected ${{expected}}, got ${{actual}}`);
                                }}
                            }},
                            toEqual: function(expected) {{
                                if (JSON.stringify(actual) !== JSON.stringify(expected)) {{
                                    throw new Error(`Expected ${{JSON.stringify(expected)}}, got ${{JSON.stringify(actual)}}`);
                                }}
                            }},
                            toBeTruthy: function() {{
                                if (!actual) throw new Error(`Expected truthy value, got ${{actual}}`);
                            }},
                            toBeGreaterThan: function(n) {{
                                if (!(actual > n)) throw new Error(`Expected ${{actual}} > ${{n}}`);
                            }}
                        }};
                    }}
                }};

                // Global expect helper alias
                globalThis.expect = globalThis.aura.expect;
            }})();
            "#
        );

        self.runtime
            .execute_script("aura_sandbox_bootstrap.js", init_script)
            .context("Failed to bootstrap deno_core sandbox")?;

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
                globalThis.response = {{
                    status: {status},
                    body: {escaped_body},
                    headers: {headers_json},
                    json: function() {{
                        return JSON.parse({escaped_body});
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

        // Extract mutated variables, console logs, and test assertions
        let extract_script = r#"
            JSON.stringify({
                vars: globalThis.__AURA_VARS,
                logs: globalThis.__AURA_LOGS,
                assertions: globalThis.__AURA_ASSERTIONS
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
            message: Option<String>,
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
