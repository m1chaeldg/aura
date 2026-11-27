use crate::preferences::{AiCliProfile, AiHttpKind, AiHttpProfile, AiPreferences};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

/// Chat message role in an AI conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiChatRole {
    User,
    Assistant,
    System,
}

/// Single chat message in the AI Assistant history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiChatMessage {
    pub role: AiChatRole,
    pub content: String,
    #[serde(default)]
    pub actions: Vec<AuraAgentAction>,
}

/// Snapshot of active Aura workspace/request state attached to an AI prompt.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiContextSnapshot {
    pub request_name: Option<String>,
    pub method: Option<String>,
    pub url: Option<String>,
    pub headers: Vec<(String, String)>,
    pub params: Vec<(String, String)>,
    pub body: Option<String>,
    pub pre_script: Option<String>,
    pub post_script: Option<String>,
    pub test_script: Option<String>,
    pub spec: Option<String>,
    pub response_status: Option<u16>,
    pub response_headers: Vec<(String, String)>,
    pub response_body: Option<String>,
    pub env_name: Option<String>,
    pub env_vars: BTreeMap<String, String>,
    pub available_commands: Vec<(String, String)>,
}

/// Structured action proposed by an AI agent to modify Aura state or run a command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AuraAgentAction {
    UpdateBody {
        content: String,
        #[serde(default)]
        content_type: Option<String>,
    },
    UpdatePreScript {
        script: String,
    },
    UpdatePostScript {
        script: String,
    },
    UpdateTestScript {
        script: String,
    },
    UpdateSpec {
        spec: String,
    },
    UpdateHeaders {
        headers: Vec<KeyValuePair>,
    },
    UpdateParams {
        params: Vec<KeyValuePair>,
    },
    CreateRequest {
        name: String,
        method: String,
        url: String,
        #[serde(default)]
        body: Option<String>,
        #[serde(default)]
        pre_script: Option<String>,
        #[serde(default)]
        test_script: Option<String>,
    },
    RunCommand {
        command_id: String,
        #[serde(default)]
        reason: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValuePair {
    pub key: String,
    pub value: String,
}

impl AuraAgentAction {
    /// Human-readable title for the Preview & Apply card in the AI drawer.
    pub fn title(&self) -> String {
        match self {
            Self::UpdateBody { .. } => "Update Request Body".to_string(),
            Self::UpdatePreScript { .. } => "Update Pre-Request Script".to_string(),
            Self::UpdatePostScript { .. } => "Update Post-Response Script".to_string(),
            Self::UpdateTestScript { .. } => "Update Test Script".to_string(),
            Self::UpdateSpec { .. } => "Update .aura Spec".to_string(),
            Self::UpdateHeaders { headers } => format!("Update Headers ({})", headers.len()),
            Self::UpdateParams { params } => format!("Update Query Params ({})", params.len()),
            Self::CreateRequest { name, method, .. } => {
                format!("Create Request: {} {}", method.to_uppercase(), name)
            }
            Self::RunCommand { command_id, .. } => format!("Run Aura Command: {command_id}"),
        }
    }

    /// Short preview snippet for the UI card.
    pub fn preview_snippet(&self) -> String {
        let raw = match self {
            Self::UpdateBody { content, .. } => content.clone(),
            Self::UpdatePreScript { script }
            | Self::UpdatePostScript { script }
            | Self::UpdateTestScript { script } => script.clone(),
            Self::UpdateSpec { spec } => spec.clone(),
            Self::UpdateHeaders { headers } => headers
                .iter()
                .map(|h| format!("{}: {}", h.key, h.value))
                .collect::<Vec<_>>()
                .join("\n"),
            Self::UpdateParams { params } => params
                .iter()
                .map(|p| format!("{}={}", p.key, p.value))
                .collect::<Vec<_>>()
                .join("&"),
            Self::CreateRequest {
                method, url, body, ..
            } => {
                let mut s = format!("{} {}", method.to_uppercase(), url);
                if let Some(b) = body {
                    s.push('\n');
                    s.push_str(b);
                }
                s
            }
            Self::RunCommand { command_id, reason } => match reason {
                Some(r) => format!("{command_id} — {r}"),
                None => command_id.clone(),
            },
        };
        truncate_text(&raw, 400)
    }
}

/// Response returned by `AiService::send_prompt`.
#[derive(Debug, Clone)]
pub struct AiResponse {
    pub display_text: String,
    pub actions: Vec<AuraAgentAction>,
    pub raw_output: String,
}

/// Checks if a header or variable name looks sensitive (tokens, keys, passwords, secrets).
pub fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.contains("authorization")
        || lower.contains("token")
        || lower.contains("secret")
        || lower.contains("password")
        || lower.contains("passwd")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("api-key")
        || lower.contains("cookie")
        || lower.contains("credential")
        || lower.contains("private_key")
}

/// Builds the system prompt instructing the agent about Aura's scripting API, `.aura` spec,
/// available commands, and the `aura-action` JSON output format.
pub fn build_system_prompt(custom_prompt: Option<&str>) -> String {
    let mut base = String::from(
        "You are Aura AI, an expert API engineering assistant embedded inside the Aura REST API client.\n\
        You help users craft HTTP requests, JSON/GraphQL/XML bodies, JavaScript pre-request/post-response/test scripts, and `.aura` specifications.\n\
        Aura's JavaScript runtime provides `aura.request`, `aura.response`, `aura.env.get/set`, and `aura.test(name, fn)` / `aura.expect(val)`.\n\n\
        When you want to propose a concrete edit to the user's request, scripts, or spec, or run an Aura command, emit one or more fenced blocks with language `aura-action` containing valid JSON matching one of:\n\
        - {\"action\": \"update_body\", \"content\": \"...\", \"content_type\": \"application/json\"}\n\
        - {\"action\": \"update_pre_script\", \"script\": \"...\"}\n\
        - {\"action\": \"update_post_script\", \"script\": \"...\"}\n\
        - {\"action\": \"update_test_script\", \"script\": \"...\"}\n\
        - {\"action\": \"update_spec\", \"spec\": \"...\"}\n\
        - {\"action\": \"update_headers\", \"headers\": [{\"key\": \"...\", \"value\": \"...\"}]}\n\
        - {\"action\": \"update_params\", \"params\": [{\"key\": \"...\", \"value\": \"...\"}]}\n\
        - {\"action\": \"create_request\", \"name\": \"...\", \"method\": \"POST\", \"url\": \"...\", \"body\": \"...\"}\n\
        - {\"action\": \"run_command\", \"command_id\": \"cmd:...\", \"reason\": \"...\"}\n\n\
        Keep explanations concise and always ensure JSON in `aura-action` blocks is valid.",
    );
    if let Some(custom) = custom_prompt.map(str::trim).filter(|s| !s.is_empty()) {
        base.push_str("\n\nUser custom instructions:\n");
        base.push_str(custom);
    }
    base
}

/// Formats `AiContextSnapshot` into a compact prompt section, redacting sensitive values if requested.
pub fn build_context_prompt(
    ctx: &AiContextSnapshot,
    include_request: bool,
    include_response: bool,
    redact_secrets: bool,
) -> String {
    let mut out = String::new();

    if include_request {
        if let (Some(method), Some(url)) = (&ctx.method, &ctx.url) {
            out.push_str(&format!(
                "### Active Request{}\n{} {}\n",
                ctx.request_name
                    .as_deref()
                    .map(|n| format!(" ({n})"))
                    .unwrap_or_default(),
                method,
                url
            ));
        }
        if !ctx.headers.is_empty() {
            out.push_str("Headers:\n");
            for (k, v) in &ctx.headers {
                let val = if redact_secrets && is_sensitive_key(k) {
                    "[REDACTED]"
                } else {
                    v.as_str()
                };
                out.push_str(&format!("  {k}: {val}\n"));
            }
        }
        if !ctx.params.is_empty() {
            out.push_str("Query Params:\n");
            for (k, v) in &ctx.params {
                let val = if redact_secrets && is_sensitive_key(k) {
                    "[REDACTED]"
                } else {
                    v.as_str()
                };
                out.push_str(&format!("  {k}={val}\n"));
            }
        }
        if let Some(body) = ctx.body.as_deref().filter(|s| !s.trim().is_empty()) {
            out.push_str("Body:\n```\n");
            out.push_str(&truncate_text(body, 4000));
            out.push_str("\n```\n");
        }
        if let Some(s) = ctx.pre_script.as_deref().filter(|s| !s.trim().is_empty()) {
            out.push_str("Pre-Request Script:\n```js\n");
            out.push_str(&truncate_text(s, 2000));
            out.push_str("\n```\n");
        }
        if let Some(s) = ctx.post_script.as_deref().filter(|s| !s.trim().is_empty()) {
            out.push_str("Post-Response Script:\n```js\n");
            out.push_str(&truncate_text(s, 2000));
            out.push_str("\n```\n");
        }
        if let Some(s) = ctx.test_script.as_deref().filter(|s| !s.trim().is_empty()) {
            out.push_str("Test Script:\n```js\n");
            out.push_str(&truncate_text(s, 2000));
            out.push_str("\n```\n");
        }
        if let Some(spec) = ctx.spec.as_deref().filter(|s| !s.trim().is_empty()) {
            out.push_str("Aura Spec (.aura):\n```toml\n");
            out.push_str(&truncate_text(spec, 3000));
            out.push_str("\n```\n");
        }
    }

    if include_response {
        if let Some(status) = ctx.response_status {
            out.push_str(&format!("\n### Latest Response (HTTP {status})\n"));
            if !ctx.response_headers.is_empty() {
                for (k, v) in ctx.response_headers.iter().take(20) {
                    let val = if redact_secrets && is_sensitive_key(k) {
                        "[REDACTED]"
                    } else {
                        v.as_str()
                    };
                    out.push_str(&format!("  {k}: {val}\n"));
                }
            }
            if let Some(body) = ctx.response_body.as_deref().filter(|s| !s.trim().is_empty()) {
                out.push_str("Response Body:\n```\n");
                out.push_str(&truncate_text(body, 4000));
                out.push_str("\n```\n");
            }
        }
    }

    if !ctx.env_vars.is_empty() {
        let env_label = ctx.env_name.as_deref().unwrap_or("Active");
        out.push_str(&format!("\n### Environment ({env_label})\n"));
        for (k, v) in &ctx.env_vars {
            let val = if redact_secrets && is_sensitive_key(k) {
                "[REDACTED]"
            } else {
                v.as_str()
            };
            out.push_str(&format!("  {k}={val}\n"));
        }
    }

    if !ctx.available_commands.is_empty() {
        out.push_str("\n### Available Aura Commands\n");
        for (id, label) in &ctx.available_commands {
            out.push_str(&format!("  - {id}: {label}\n"));
        }
    }

    out
}

/// Extracts `aura-action` fenced JSON blocks from a model response and returns
/// `(cleaned_display_text, parsed_actions)`.
pub fn parse_agent_actions(raw: &str) -> (String, Vec<AuraAgentAction>) {
    let mut actions = Vec::new();
    let mut display = String::with_capacity(raw.len());
    let mut remaining = raw;

    while let Some(start_idx) = remaining.find("```aura-action") {
        display.push_str(&remaining[..start_idx]);
        let after_fence = &remaining[start_idx + "```aura-action".len()..];
        let after_newline = after_fence.strip_prefix("\r\n").or_else(|| after_fence.strip_prefix('\n')).unwrap_or(after_fence);
        if let Some(end_idx) = after_newline.find("```") {
            let json_block = after_newline[..end_idx].trim();
            if let Ok(single) = serde_json::from_str::<AuraAgentAction>(json_block) {
                actions.push(single);
            } else if let Ok(many) = serde_json::from_str::<Vec<AuraAgentAction>>(json_block) {
                actions.extend(many);
            } else {
                // Keep unparseable block visible so the user still sees what the model output.
                display.push_str("```aura-action\n");
                display.push_str(json_block);
                display.push_str("\n```");
            }
            remaining = &after_newline[end_idx + 3..];
        } else {
            display.push_str(&remaining[start_idx..]);
            remaining = "";
            break;
        }
    }
    display.push_str(remaining);

    (display.trim().to_string(), actions)
}

/// Resolves an API key value, expanding `$ENV_VAR` references if present.
pub fn resolve_api_key(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(var_name) = trimmed.strip_prefix('$') {
        let var_name = var_name.trim_matches(|c| c == '{' || c == '}');
        std::env::var(var_name).unwrap_or_default()
    } else {
        trimmed.to_string()
    }
}

fn truncate_text(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max_chars).collect();
        out.push_str("\n... [truncated]");
        out
    }
}

/// Asynchronous service that dispatches prompts to local Agent CLIs or HTTP endpoints.
pub struct AiService;

impl AiService {
    /// Sends a user prompt with Aura context to the currently selected provider in `prefs`.
    pub async fn send_prompt(
        prefs: &AiPreferences,
        history: &[AiChatMessage],
        user_prompt: &str,
        context: &AiContextSnapshot,
        cancel: CancellationToken,
    ) -> Result<AiResponse> {
        let provider_id = prefs
            .active_provider_id
            .as_deref()
            .ok_or_else(|| anyhow!("No AI provider selected. Open AI -> AI & Agent Settings or run Discover Local CLIs."))?;

        let system_prompt = build_system_prompt(prefs.custom_system_prompt.as_deref());
        let ctx_prompt = build_context_prompt(
            context,
            prefs.attach_request_context,
            prefs.attach_response_context,
            prefs.redact_secrets,
        );

        let full_user_prompt = if ctx_prompt.trim().is_empty() {
            user_prompt.to_string()
        } else {
            format!("{ctx_prompt}\n\n### User Request\n{user_prompt}")
        };

        if let Some(cli) = prefs.cli_profiles.iter().find(|p| p.id == provider_id) {
            return Self::run_cli(cli, &system_prompt, &full_user_prompt, cancel).await;
        }

        if let Some(http) = prefs.http_profiles.iter().find(|p| p.id == provider_id) {
            return Self::run_http(http, &system_prompt, history, &full_user_prompt, cancel).await;
        }

        Err(anyhow!("Configured AI provider '{provider_id}' was not found."))
    }

    async fn run_cli(
        cli: &AiCliProfile,
        system_prompt: &str,
        user_prompt: &str,
        cancel: CancellationToken,
    ) -> Result<AiResponse> {
        let combined_prompt = format!("{system_prompt}\n\n---\n\n{user_prompt}");
        let mut cmd = tokio::process::Command::new(&cli.command);
        let mut uses_placeholder = false;

        for arg in &cli.args {
            if arg.contains("{prompt}") {
                uses_placeholder = true;
                cmd.arg(arg.replace("{prompt}", &combined_prompt));
            } else {
                cmd.arg(arg);
            }
        }

        if !uses_placeholder {
            cmd.stdin(Stdio::piped());
        } else {
            cmd.stdin(Stdio::null());
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn AI CLI '{}'", cli.command))?;

        if !uses_placeholder {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(combined_prompt.as_bytes()).await?;
                stdin.shutdown().await?;
            }
        }

        let timeout_dur = Duration::from_secs(cli.timeout_secs.max(10));
        let output = tokio::select! {
            _ = cancel.cancelled() => {
                // `kill_on_drop(true)` terminates the child when `wait_with_output` is dropped.
                return Err(anyhow!("AI CLI request cancelled"));
            }
            res = tokio::time::timeout(timeout_dur, child.wait_with_output()) => {
                res.map_err(|_| anyhow!("AI CLI '{}' timed out after {}s", cli.name, cli.timeout_secs))??
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            let detail = if stderr.trim().is_empty() {
                stdout.trim()
            } else {
                stderr.trim()
            };
            return Err(anyhow!(
                "CLI '{}' exited with {}: {}",
                cli.name,
                output.status,
                truncate_text(detail, 500)
            ));
        }

        let (display_text, actions) = parse_agent_actions(&stdout);
        Ok(AiResponse {
            display_text,
            actions,
            raw_output: stdout,
        })
    }

    async fn run_http(
        http: &AiHttpProfile,
        system_prompt: &str,
        history: &[AiChatMessage],
        user_prompt: &str,
        cancel: CancellationToken,
    ) -> Result<AiResponse> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()?;
        let api_key = resolve_api_key(&http.api_key);
        let base = http.base_url.trim_end_matches('/');

        match http.kind {
            AiHttpKind::OpenAiCompatible => {
                let url = format!("{base}/chat/completions");
                let mut messages = vec![serde_json::json!({
                    "role": "system",
                    "content": system_prompt,
                })];
                for msg in history.iter().rev().take(10).rev() {
                    let role = match msg.role {
                        AiChatRole::User => "user",
                        AiChatRole::Assistant => "assistant",
                        AiChatRole::System => "system",
                    };
                    messages.push(serde_json::json!({
                        "role": role,
                        "content": msg.content,
                    }));
                }
                messages.push(serde_json::json!({
                    "role": "user",
                    "content": user_prompt,
                }));

                let body = serde_json::json!({
                    "model": http.model,
                    "messages": messages,
                    "tools": openai_tool_definitions(),
                    "tool_choice": "auto",
                });

                let mut req = client.post(&url).json(&body);
                if !api_key.is_empty() {
                    req = req.bearer_auth(&api_key);
                }

                let resp = tokio::select! {
                    _ = cancel.cancelled() => return Err(anyhow!("AI request cancelled")),
                    res = req.send() => res?,
                };

                let status = resp.status();
                let text = resp.text().await?;
                if !status.is_success() {
                    return Err(anyhow!("HTTP {status} from {}: {}", http.name, truncate_text(&text, 400)));
                }

                parse_openai_response(&text)
            }
            AiHttpKind::Anthropic => {
                let url = format!("{base}/messages");
                if api_key.is_empty() {
                    return Err(anyhow!(
                        "Anthropic API key is empty. Set it or $ANTHROPIC_API_KEY in AI Settings."
                    ));
                }

                let mut messages = Vec::new();
                for msg in history.iter().rev().take(10).rev() {
                    if msg.role == AiChatRole::System {
                        continue;
                    }
                    let role = match msg.role {
                        AiChatRole::User => "user",
                        AiChatRole::Assistant => "assistant",
                        AiChatRole::System => "user",
                    };
                    messages.push(serde_json::json!({
                        "role": role,
                        "content": msg.content,
                    }));
                }
                messages.push(serde_json::json!({
                    "role": "user",
                    "content": user_prompt,
                }));

                let body = serde_json::json!({
                    "model": http.model,
                    "max_tokens": 4096,
                    "system": system_prompt,
                    "messages": messages,
                    "tools": anthropic_tool_definitions(),
                });

                let req = client
                    .post(&url)
                    .header("x-api-key", &api_key)
                    .header("anthropic-version", "2023-06-01")
                    .json(&body);

                let resp = tokio::select! {
                    _ = cancel.cancelled() => return Err(anyhow!("AI request cancelled")),
                    res = req.send() => res?,
                };

                let status = resp.status();
                let text = resp.text().await?;
                if !status.is_success() {
                    return Err(anyhow!("HTTP {status} from {}: {}", http.name, truncate_text(&text, 400)));
                }

                parse_anthropic_response(&text)
            }
        }
    }
}

fn openai_tool_definitions() -> serde_json::Value {
    serde_json::json!([
        {
            "type": "function",
            "function": {
                "name": "aura_action",
                "description": "Propose a structured edit to the active Aura HTTP request, scripts, or .aura spec, or run an Aura command.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": [
                                "update_body", "update_pre_script", "update_post_script",
                                "update_test_script", "update_spec", "update_headers",
                                "update_params", "create_request", "run_command"
                            ]
                        },
                        "content": { "type": "string" },
                        "content_type": { "type": "string" },
                        "script": { "type": "string" },
                        "spec": { "type": "string" },
                        "name": { "type": "string" },
                        "method": { "type": "string" },
                        "url": { "type": "string" },
                        "body": { "type": "string" },
                        "command_id": { "type": "string" },
                        "reason": { "type": "string" }
                    },
                    "required": ["action"]
                }
            }
        }
    ])
}

fn anthropic_tool_definitions() -> serde_json::Value {
    serde_json::json!([
        {
            "name": "aura_action",
            "description": "Propose a structured edit to the active Aura HTTP request, scripts, or .aura spec, or run an Aura command.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": [
                            "update_body", "update_pre_script", "update_post_script",
                            "update_test_script", "update_spec", "update_headers",
                            "update_params", "create_request", "run_command"
                        ]
                    },
                    "content": { "type": "string" },
                    "content_type": { "type": "string" },
                    "script": { "type": "string" },
                    "spec": { "type": "string" },
                    "name": { "type": "string" },
                    "method": { "type": "string" },
                    "url": { "type": "string" },
                    "body": { "type": "string" },
                    "command_id": { "type": "string" },
                    "reason": { "type": "string" }
                },
                "required": ["action"]
            }
        }
    ])
}

/// Parses an OpenAI `/v1/chat/completions` JSON response into `AiResponse`.
pub fn parse_openai_response(raw: &str) -> Result<AiResponse> {
    let val: serde_json::Value = serde_json::from_str(raw).context("Invalid OpenAI JSON response")?;
    let msg = val
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .ok_or_else(|| anyhow!("Missing choices[0].message in OpenAI response"))?;

    let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
    let (display_text, mut actions) = parse_agent_actions(content);

    if let Some(tool_calls) = msg.get("tool_calls").and_then(|t| t.as_array()) {
        for tc in tool_calls {
            if let Some(args_str) = tc
                .get("function")
                .and_then(|f| f.get("arguments"))
                .and_then(|a| a.as_str())
            {
                if let Ok(action) = serde_json::from_str::<AuraAgentAction>(args_str) {
                    actions.push(action);
                }
            }
        }
    }

    Ok(AiResponse {
        display_text,
        actions,
        raw_output: raw.to_string(),
    })
}

/// Parses an Anthropic `/v1/messages` JSON response into `AiResponse`.
pub fn parse_anthropic_response(raw: &str) -> Result<AiResponse> {
    let val: serde_json::Value =
        serde_json::from_str(raw).context("Invalid Anthropic JSON response")?;
    let content_blocks = val
        .get("content")
        .and_then(|c| c.as_array())
        .ok_or_else(|| anyhow!("Missing content array in Anthropic response"))?;

    let mut text_parts = Vec::new();
    let mut actions = Vec::new();

    for block in content_blocks {
        match block.get("type").and_then(|t| t.as_str()) {
            Some("text") => {
                if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
                    text_parts.push(t.to_string());
                }
            }
            Some("tool_use") => {
                if let Some(input) = block.get("input") {
                    if let Ok(action) = serde_json::from_value::<AuraAgentAction>(input.clone()) {
                        actions.push(action);
                    }
                }
            }
            _ => {}
        }
    }

    let joined = text_parts.join("\n");
    let (display_text, mut block_actions) = parse_agent_actions(&joined);
    actions.append(&mut block_actions);

    Ok(AiResponse {
        display_text,
        actions,
        raw_output: raw.to_string(),
    })
}
