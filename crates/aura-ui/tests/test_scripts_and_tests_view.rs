use aura_core::model::AuraResponse;
use aura_ui::app_state::{AuraAppState, ScriptSubTab};
use std::collections::HashMap;

#[test]
fn test_script_sub_tabs_and_snippet_insertion() {
    let mut state = AuraAppState::new();
    state.new_request();

    // Verify sub-tab switching
    assert_eq!(state.active_script_sub_tab, ScriptSubTab::PreRequest);
    state.set_script_sub_tab(ScriptSubTab::PostResponse);
    assert_eq!(state.active_script_sub_tab, ScriptSubTab::PostResponse);

    // Insert snippet into post-response
    state.append_script_snippet(false, "console.log('done');\n");
    assert_eq!(
        state.tabs[0].request.scripts.post_response,
        "console.log('done');\n"
    );

    // Switch back to pre-request and insert snippet
    state.set_script_sub_tab(ScriptSubTab::PreRequest);
    state.append_script_snippet(true, "aura.environment.set('token', '123');\n");
    assert_eq!(
        state.tabs[0].request.scripts.pre_request,
        "aura.environment.set('token', '123');\n"
    );
}

#[test]
fn test_tests_panel_snippets_and_manual_evaluation() {
    let mut state = AuraAppState::new();
    state.new_request();

    // Insert test assertion snippet
    let test_code = "describe('Status Check', () => {\n    it('returns 200 OK', () => expect(response.status).toBe(200));\n});\n";
    state.append_test_snippet(test_code);
    assert_eq!(state.tabs[0].request.scripts.tests, test_code);

    // Attach mock response to active tab
    state.tabs[0].response = Some(AuraResponse {
        status: 200,
        status_text: "OK".to_string(),
        duration_ms: 45,
        size_bytes: 128,
        headers: HashMap::new(),
        body: "{\"success\": true}".to_string(),
        timestamp: chrono::Utc::now(),
        assertions: Vec::new(),
        console_logs: Vec::new(),
    });

    // Run tests directly on demand
    let result = state.run_active_tests();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);

    // Verify assertions populated in the response
    let assertions = &state.tabs[0].response.as_ref().unwrap().assertions;
    assert_eq!(assertions.len(), 1);
    assert_eq!(assertions[0].name, "Status Check > returns 200 OK");
    assert!(assertions[0].passed);
}
