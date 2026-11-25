use aura_core::model::{AuthConfig, BodyType, HttpMethod, ParamItem};
use aura_ui::app_state::{ActiveInputTarget, AuraAppState, ScriptSubTab};
use aura_ui::stores::WorkspaceTab;

#[test]
fn test_active_input_target_selection_and_blur() {
    let mut state = AuraAppState::new();
    assert_eq!(state.active_input_target, None);

    state.set_active_input(Some(ActiveInputTarget::ParamKey(0)));
    assert_eq!(
        state.active_input_target,
        Some(ActiveInputTarget::ParamKey(0))
    );

    state.blur_active_input();
    assert_eq!(state.active_input_target, None);
}

#[test]
fn test_inline_typing_into_param() {
    let mut state = AuraAppState::new();
    // Start with a request tab
    let mut req = aura_core::model::AuraRequest::new(
        "Req 1",
        HttpMethod::GET,
        "https://postman-echo.com/get",
    );
    req.params
        .push(ParamItem::new("page".to_string(), "1".to_string()));
    state.tabs.push(WorkspaceTab::new(req));
    state.active_tab_index = 0;

    // Type into param key
    state.set_active_input(Some(ActiveInputTarget::ParamKey(0)));
    state.type_char_into_active('s');
    assert_eq!(state.tabs[0].request.params[0].key, "pages");
    assert!(state.tabs[0].is_dirty);

    // Backspace
    state.backspace_active();
    assert_eq!(state.tabs[0].request.params[0].key, "page");

    // Switch to value
    state.set_active_input(Some(ActiveInputTarget::ParamValue(0)));
    state.type_char_into_active('0');
    assert_eq!(state.tabs[0].request.params[0].value, "10");
}

#[test]
fn test_inline_typing_into_auth_bearer() {
    let mut state = AuraAppState::new();
    let mut req =
        aura_core::model::AuraRequest::new("Req 1", HttpMethod::GET, "https://api.example.com");
    req.auth = AuthConfig::Bearer {
        token: "abc".to_string(),
    };
    state.tabs.push(WorkspaceTab::new(req));
    state.active_tab_index = 0;

    state.set_active_input(Some(ActiveInputTarget::AuthBearerToken));
    state.type_char_into_active('d');
    if let AuthConfig::Bearer { token } = &state.tabs[0].request.auth {
        assert_eq!(token, "abcd");
    } else {
        panic!("Expected Bearer auth");
    }
}

#[test]
fn test_inline_typing_into_body() {
    let mut state = AuraAppState::new();
    let mut req =
        aura_core::model::AuraRequest::new("Req 1", HttpMethod::POST, "https://api.example.com");
    req.body = BodyType::Json("{}".to_string());
    state.tabs.push(WorkspaceTab::new(req));
    state.active_tab_index = 0;

    state.set_active_input(Some(ActiveInputTarget::BodyText));
    state.type_char_into_active('\n');
    assert_eq!(state.tabs[0].request.body.content_str(), "{}\n");
}

#[test]
fn test_scripts_sub_tab_and_typing() {
    let mut state = AuraAppState::new();
    let req =
        aura_core::model::AuraRequest::new("Req 1", HttpMethod::GET, "https://api.example.com");
    state.tabs.push(WorkspaceTab::new(req));
    state.active_tab_index = 0;

    assert_eq!(state.active_script_sub_tab, ScriptSubTab::PreRequest);
    state.set_script_sub_tab(ScriptSubTab::PostResponse);
    assert_eq!(state.active_script_sub_tab, ScriptSubTab::PostResponse);

    state.set_active_input(Some(ActiveInputTarget::PostResponseScript));
    state.type_char_into_active('c');
    state.type_char_into_active('o');
    state.type_char_into_active('n');
    state.type_char_into_active('s');
    state.type_char_into_active('t');
    assert_eq!(state.tabs[0].request.scripts.post_response, "const");
}

#[test]
fn test_tab_navigation_in_params_grid() {
    let mut state = AuraAppState::new();
    let mut req =
        aura_core::model::AuraRequest::new("Req 1", HttpMethod::GET, "https://api.example.com");
    req.params
        .push(ParamItem::new("k1".to_string(), "v1".to_string()));
    state.tabs.push(WorkspaceTab::new(req));
    state.active_tab_index = 0;

    // Start at ParamKey(0)
    state.set_active_input(Some(ActiveInputTarget::ParamKey(0)));
    // Tab -> ParamValue(0)
    state.tab_next_input(false);
    assert_eq!(
        state.active_input_target,
        Some(ActiveInputTarget::ParamValue(0))
    );

    // Tab -> Auto adds new row and focuses ParamKey(1)
    state.tab_next_input(false);
    assert_eq!(
        state.active_input_target,
        Some(ActiveInputTarget::ParamKey(1))
    );
    assert_eq!(state.tabs[0].request.params.len(), 2);
}

#[test]
fn test_get_and_set_active_input_text_all_targets() {
    let mut state = AuraAppState::new();
    let mut req =
        aura_core::model::AuraRequest::new("Test Req", HttpMethod::POST, "https://example.com/api");
    req.params
        .push(ParamItem::new("page".to_string(), "1".to_string()));
    req.headers
        .push(aura_core::model::HeaderItem::new("X-Test".to_string(), "Val".to_string()));
    req.body = BodyType::Json("{\"hello\": 1}".to_string());
    req.scripts.pre_request = "console.log('pre')".to_string();
    req.scripts.post_response = "console.log('post')".to_string();
    req.scripts.tests = "expect(200).toBe(200)".to_string();

    state.tabs.push(WorkspaceTab::new(req));
    state.active_tab_index = 0;

    // 1. Param Key & Value
    state.set_active_input(Some(ActiveInputTarget::ParamKey(0)));
    assert_eq!(state.get_active_input_text(), Some("page".to_string()));
    state.set_active_input_text("limit".to_string());
    assert_eq!(state.tabs[0].request.params[0].key, "limit");

    state.set_active_input(Some(ActiveInputTarget::ParamValue(0)));
    assert_eq!(state.get_active_input_text(), Some("1".to_string()));
    state.set_active_input_text("100".to_string());
    assert_eq!(state.tabs[0].request.params[0].value, "100");

    // 2. URL
    state.set_active_input(Some(ActiveInputTarget::Url));
    assert_eq!(state.get_active_input_text(), Some("https://example.com/api?limit=100".to_string()));
    state.set_active_input_text("https://example.com/v2?limit=100".to_string());
    assert_eq!(state.get_active_input_text(), Some("https://example.com/v2?limit=100".to_string()));
    assert_eq!(state.tabs[0].request.url, "https://example.com/v2?limit=100");

    // 3. Header Key & Value
    state.set_active_input(Some(ActiveInputTarget::HeaderKey(0)));
    assert_eq!(state.get_active_input_text(), Some("X-Test".to_string()));
    state.set_active_input_text("Authorization".to_string());
    assert_eq!(state.tabs[0].request.headers[0].key, "Authorization");

    state.set_active_input(Some(ActiveInputTarget::HeaderValue(0)));
    assert_eq!(state.get_active_input_text(), Some("Val".to_string()));
    state.set_active_input_text("Bearer abc".to_string());
    assert_eq!(state.tabs[0].request.headers[0].value, "Bearer abc");

    // 4. Body
    state.set_active_input(Some(ActiveInputTarget::BodyText));
    assert_eq!(state.get_active_input_text(), Some("{\"hello\": 1}".to_string()));
    state.set_active_input_text("{\"key\": \"updated\"}".to_string());
    assert_eq!(state.tabs[0].request.body.content_str(), "{\"key\": \"updated\"}");

    // 5. Scripts and Tests
    state.set_active_input(Some(ActiveInputTarget::PreRequestScript));
    assert_eq!(state.get_active_input_text(), Some("console.log('pre')".to_string()));
    state.set_active_input_text("// new pre".to_string());
    assert_eq!(state.tabs[0].request.scripts.pre_request, "// new pre");

    state.set_active_input(Some(ActiveInputTarget::PostResponseScript));
    assert_eq!(state.get_active_input_text(), Some("console.log('post')".to_string()));
    state.set_active_input_text("// new post".to_string());
    assert_eq!(state.tabs[0].request.scripts.post_response, "// new post");

    state.set_active_input(Some(ActiveInputTarget::TestScript));
    assert_eq!(state.get_active_input_text(), Some("expect(200).toBe(200)".to_string()));
    state.set_active_input_text("// new test".to_string());
    assert_eq!(state.tabs[0].request.scripts.tests, "// new test");
}

#[test]
fn test_paste_text_into_active_inputs() {
    let mut state = AuraAppState::new();
    let req =
        aura_core::model::AuraRequest::new("Req", HttpMethod::GET, "https://api.com");
    state.tabs.push(WorkspaceTab::new(req));
    state.active_tab_index = 0;

    // Paste into URL
    state.set_active_input(Some(ActiveInputTarget::Url));
    state.paste_text_into_active("/users?filter=admin");
    assert_eq!(state.tabs[0].request.url, "https://api.com/users?filter=admin");
    assert_eq!(state.tabs[0].request.params.len(), 1);
    assert_eq!(state.tabs[0].request.params[0].key, "filter");
    assert_eq!(state.tabs[0].request.params[0].value, "admin");

    // Paste into Body
    state.set_active_input(Some(ActiveInputTarget::BodyText));
    state.paste_text_into_active("{\"batch\": true}");
    assert_eq!(state.tabs[0].request.body.content_str(), "{\"batch\": true}");
}

#[test]
fn test_spec_copy_and_paste_without_errors() {
    let mut state = AuraAppState::new();
    let req =
        aura_core::model::AuraRequest::new("Spec Test", HttpMethod::POST, "https://postman-echo.com/post");
    state.tabs.push(WorkspaceTab::new(req));
    state.active_tab_index = 0;

    // Focus spec editor
    state.set_active_input(Some(ActiveInputTarget::SpecText));

    // Get active spec text (simulating Ctrl+C)
    let copied_spec = state.get_active_input_text().expect("spec text");
    assert!(copied_spec.contains("version = \"1.0\""));
    assert!(copied_spec.contains("Spec Test"));
    assert!(copied_spec.contains("https://postman-echo.com/post"));

    // Now paste a new valid spec (simulating Ctrl+V)
    let incoming_spec = "version = \"1.0\"\nname = \"Pasted Endpoint\"\nmethod = \"PUT\"\nurl = \"https://postman-echo.com/put\"\n\n[headers]\nAuthorization = \"Bearer secret-token-xyz\"\n";

    // Completely replace/paste spec
    state.set_active_input_text(incoming_spec.to_string());
    let tab = &state.tabs[0];
    assert!(tab.spec_error.is_none(), "Valid pasted spec should have no errors");
    assert_eq!(tab.request.name, "Pasted Endpoint");
    assert_eq!(tab.request.method, HttpMethod::PUT);
    assert_eq!(tab.request.url, "https://postman-echo.com/put");
    assert_eq!(tab.request.headers.len(), 1);
    assert_eq!(tab.request.headers[0].key, "Authorization");
    assert_eq!(tab.request.headers[0].value, "Bearer secret-token-xyz");

    // Test resilient paste: invalid spec reports error without crashing or corrupting model
    let invalid_spec = "version = \"1.0\"\ninvalid syntax :::";
    state.set_active_input_text(invalid_spec.to_string());
    assert!(state.tabs[0].spec_error.is_some(), "Invalid spec reports error in status pill");
    // Visual model remains safe
    assert_eq!(state.tabs[0].request.name, "Pasted Endpoint");

    // Re-paste valid spec restores clean state
    state.set_active_input_text(incoming_spec.to_string());
    assert!(state.tabs[0].spec_error.is_none());
}

#[test]
fn test_dialog_copy_and_paste_helpers() {
    let mut state = AuraAppState::new();
    state.new_request();
    state.open_add_header_dialog();
    assert!(state.editor_dialog.is_some());

    // Paste into key
    state.dialog_paste_text("X-Correlation-ID");
    assert_eq!(state.dialog_get_text(), Some("X-Correlation-ID".to_string()));

    // Tab to value
    state.dialog_tab_next_field();
    state.dialog_paste_text("corr-12345");
    assert_eq!(state.dialog_get_text(), Some("corr-12345".to_string()));

    // Set text (simulating cut or replace)
    state.dialog_set_text("corr-67890".to_string());
    assert_eq!(state.dialog_get_text(), Some("corr-67890".to_string()));

    // Confirm dialog
    state.confirm_editor_dialog();
    assert_eq!(state.tabs[0].request.headers.len(), 1);
    assert_eq!(state.tabs[0].request.headers[0].key, "X-Correlation-ID");
    assert_eq!(state.tabs[0].request.headers[0].value, "corr-67890");
}

#[test]
fn test_rename_and_move_to_copy_paste_helpers() {
    let mut state = AuraAppState::new();

    // 1. Rename dialog
    state.request_rename(std::path::PathBuf::from("/test/Request.aura"), false);
    assert_eq!(state.rename_get_text(), Some("Request".to_string()));
    state.update_rename_text(String::new());
    state.rename_paste_text("NewRequestName");
    assert_eq!(state.rename_get_text(), Some("NewRequestName".to_string()));
    state.cancel_rename();

    // 2. Move To dialog
    state.move_to_dialog = Some(aura_ui::app_state::MoveToDialogState {
        source_path: std::path::PathBuf::from("/test/req.aura"),
        search_query: String::new(),
        selected_index: 0,
        destinations: Vec::new(),
    });
    assert_eq!(state.move_to_get_text(), Some("".to_string()));
    state.move_to_paste_text("Auth");
    assert_eq!(state.move_to_get_text(), Some("Auth".to_string()));
    state.cancel_move_to();
}
