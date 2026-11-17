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
