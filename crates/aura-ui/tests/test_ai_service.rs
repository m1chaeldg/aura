use aura_ui::preferences::{AiCliProfile, AiPreferences, AuraPreferences};
use aura_ui::services::ai_discovery::{
    AiDiscoveryResult, discover_local_clis_in_dirs, merge_discovery_into_preferences,
};
use aura_ui::services::ai_service::{
    AiContextSnapshot, AuraAgentAction, build_context_prompt, parse_agent_actions,
    parse_anthropic_response, parse_openai_response, resolve_api_key,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!("aura_test_ai_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&p).expect("create temp dir");
        Self(p)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn test_discover_local_clis_in_dirs() {
    let tmp = TempDir::new();
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let claude_name = if cfg!(windows) { "claude.exe" } else { "claude" };
    let agy_name = if cfg!(windows) { "agy.cmd" } else { "agy" };

    let claude_path = bin_dir.join(claude_name);
    let agy_path = bin_dir.join(agy_name);
    fs::write(&claude_path, b"stub").unwrap();
    fs::write(&agy_path, b"stub").unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&claude_path, fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(&agy_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let found = discover_local_clis_in_dirs(&[bin_dir]);
    let ids: Vec<&str> = found.iter().map(|p| p.id.as_str()).collect();
    assert!(ids.contains(&"cli:claude"));
    assert!(ids.contains(&"cli:agy"));
}

#[test]
fn test_merge_discovery_sets_default_active_provider() {
    let mut prefs = AiPreferences::default();
    assert_eq!(prefs.active_provider_id, None);

    let discovery = AiDiscoveryResult {
        cli_profiles: vec![AiCliProfile {
            id: "cli:claude".to_string(),
            name: "Claude Code CLI".to_string(),
            command: "/usr/bin/claude".to_string(),
            args: vec!["-p".to_string(), "{prompt}".to_string()],
            enabled: true,
            auto_discovered: true,
            timeout_secs: 120,
        }],
        local_http_profiles: vec![],
    };

    merge_discovery_into_preferences(&mut prefs, &discovery);
    assert_eq!(prefs.active_provider_id.as_deref(), Some("cli:claude"));
    assert_eq!(prefs.cli_profiles.len(), 1);
}

#[test]
fn test_context_prompt_redacts_secrets_by_default() {
    let mut env_vars = BTreeMap::new();
    env_vars.insert("BASE_URL".to_string(), "https://api.example.com".to_string());
    env_vars.insert("API_TOKEN".to_string(), "super-secret-123".to_string());

    let ctx = AiContextSnapshot {
        request_name: Some("Get Users".to_string()),
        method: Some("GET".to_string()),
        url: Some("{{BASE_URL}}/users".to_string()),
        headers: vec![
            ("Accept".to_string(), "application/json".to_string()),
            ("Authorization".to_string(), "Bearer secret-jwt".to_string()),
        ],
        env_vars,
        ..Default::default()
    };

    let redacted = build_context_prompt(&ctx, true, true, true);
    assert!(redacted.contains("Accept: application/json"));
    assert!(redacted.contains("Authorization: [REDACTED]"));
    assert!(!redacted.contains("secret-jwt"));
    assert!(redacted.contains("API_TOKEN=[REDACTED]"));
    assert!(!redacted.contains("super-secret-123"));

    let unredacted = build_context_prompt(&ctx, true, true, false);
    assert!(unredacted.contains("Bearer secret-jwt"));
    assert!(unredacted.contains("super-secret-123"));
}

#[test]
fn test_parse_agent_actions_extracts_fenced_blocks() {
    let raw = r#"Here is a test script and updated body:

```aura-action
{"action": "update_body", "content": "{\"ok\": true}", "content_type": "application/json"}
```

```aura-action
[
  {"action": "update_test_script", "script": "aura.test('200', () => aura.expect(aura.response.status).toBe(200));"},
  {"action": "run_command", "command_id": "cmd:save_request", "reason": "Persist changes"}
]
```

Done!"#;

    let (display, actions) = parse_agent_actions(raw);
    assert!(display.contains("Here is a test script"));
    assert!(display.contains("Done!"));
    assert!(!display.contains("aura-action"));
    assert_eq!(actions.len(), 3);
    assert!(matches!(actions[0], AuraAgentAction::UpdateBody { .. }));
    assert!(matches!(actions[1], AuraAgentAction::UpdateTestScript { .. }));
    assert!(matches!(actions[2], AuraAgentAction::RunCommand { .. }));
}

#[test]
fn test_parse_openai_and_anthropic_tool_calls() {
    let openai_json = r#"{
        "choices": [{
            "message": {
                "content": "Updating pre-script.",
                "tool_calls": [{
                    "function": {
                        "name": "aura_action",
                        "arguments": "{\"action\":\"update_pre_script\",\"script\":\"console.log('hi');\"}"
                    }
                }]
            }
        }]
    }"#;
    let resp = parse_openai_response(openai_json).unwrap();
    assert_eq!(resp.display_text, "Updating pre-script.");
    assert_eq!(resp.actions.len(), 1);
    assert!(matches!(resp.actions[0], AuraAgentAction::UpdatePreScript { .. }));

    let anthropic_json = r#"{
        "content": [
            {"type": "text", "text": "Updating spec."},
            {"type": "tool_use", "name": "aura_action", "input": {"action": "update_spec", "spec": "GET https://example.com"}}
        ]
    }"#;
    let resp = parse_anthropic_response(anthropic_json).unwrap();
    assert_eq!(resp.display_text, "Updating spec.");
    assert_eq!(resp.actions.len(), 1);
    assert!(matches!(resp.actions[0], AuraAgentAction::UpdateSpec { .. }));
}

#[test]
fn test_ai_preferences_roundtrip_and_env_resolution() {
    let tmp = TempDir::new();
    let pref_file = tmp.path().join("prefs.json");
    let mut prefs = AuraPreferences::default();
    prefs.ai.redact_secrets = false;
    prefs.ai.auto_apply_edits = true;
    prefs.save_to_file(&pref_file).unwrap();

    let loaded = AuraPreferences::load_from_file(&pref_file);
    assert!(!loaded.ai.redact_secrets);
    assert!(loaded.ai.auto_apply_edits);
    assert_eq!(resolve_api_key("sk-literal-123"), "sk-literal-123");
}

#[test]
fn test_app_state_apply_agent_actions_and_context_snapshot() {
    use aura_core::model::{AuraRequest, BodyType, HttpMethod};
    use aura_ui::app_state::AuraAppState;

    let mut state = AuraAppState::default();
    let req = AuraRequest::new("Test", HttpMethod::POST, "https://api.example.com/items");
    state.open_request(req);

    state
        .apply_agent_action(&AuraAgentAction::UpdateBody {
            content: "{\"item\":\"book\"}".to_string(),
            content_type: Some("application/json".to_string()),
        })
        .unwrap();
    state
        .apply_agent_action(&AuraAgentAction::UpdatePreScript {
            script: "aura.variables.set('ts', '1');".to_string(),
        })
        .unwrap();
    state
        .apply_agent_action(&AuraAgentAction::UpdatePostScript {
            script: "aura.variables.set('id', '42');".to_string(),
        })
        .unwrap();
    state
        .apply_agent_action(&AuraAgentAction::UpdateTestScript {
            script: "aura.test('ok', () => {});".to_string(),
        })
        .unwrap();

    let tab = &state.tabs[state.active_tab_index];
    assert_eq!(tab.request.body, BodyType::Json("{\"item\":\"book\"}".to_string()));
    assert_eq!(tab.request.scripts.pre_request, "aura.variables.set('ts', '1');");
    assert_eq!(tab.request.scripts.post_response, "aura.variables.set('id', '42');");
    assert_eq!(tab.request.scripts.tests, "aura.test('ok', () => {});");

    let snap = state.build_ai_context_snapshot();
    assert_eq!(snap.method.as_deref(), Some("POST"));
    assert_eq!(snap.body.as_deref(), Some("{\"item\":\"book\"}"));
    assert!(snap.spec.is_some());

    let cmd = state
        .apply_agent_action(&AuraAgentAction::RunCommand {
            command_id: "cmd:save_request".to_string(),
            reason: None,
        })
        .unwrap();
    assert_eq!(cmd.as_deref(), Some("cmd:save_request"));
}
