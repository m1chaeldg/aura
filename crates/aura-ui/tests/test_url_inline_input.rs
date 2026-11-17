use aura_core::model::{AuraRequest, HttpMethod};
use aura_ui::app_state::{ActiveInputTarget, AuraAppState};
use aura_ui::stores::WorkspaceTab;

#[test]
fn test_url_active_input_selection_and_typing() {
    let mut state = AuraAppState::new();
    let req = AuraRequest::new(
        "Echo GET",
        HttpMethod::GET,
        "https://postman-echo.com/get",
    );
    state.tabs.push(WorkspaceTab::new(req));
    state.active_tab_index = 0;

    // Focus URL input
    state.set_active_input(Some(ActiveInputTarget::Url));
    assert_eq!(state.active_input_target, Some(ActiveInputTarget::Url));

    // Type character into URL
    state.type_char_into_active('?');
    state.type_char_into_active('a');
    state.type_char_into_active('=');
    state.type_char_into_active('1');

    assert_eq!(
        state.tabs[0].request.url,
        "https://postman-echo.com/get?a=1"
    );
    assert!(state.tabs[0].is_dirty);

    // Backspace
    state.backspace_active();
    assert_eq!(
        state.tabs[0].request.url,
        "https://postman-echo.com/get?a="
    );
}

#[test]
fn test_url_syncs_to_params() {
    let mut state = AuraAppState::new();
    let req = AuraRequest::new(
        "Echo GET",
        HttpMethod::GET,
        "https://postman-echo.com/get",
    );
    state.tabs.push(WorkspaceTab::new(req));
    state.active_tab_index = 0;

    state.set_active_input(Some(ActiveInputTarget::Url));
    // Type query param ?page=2&limit=50
    for c in "?page=2&limit=50".chars() {
        state.type_char_into_active(c);
    }

    assert_eq!(
        state.tabs[0].request.url,
        "https://postman-echo.com/get?page=2&limit=50"
    );
    assert_eq!(state.tabs[0].request.params.len(), 2);
    assert_eq!(state.tabs[0].request.params[0].key, "page");
    assert_eq!(state.tabs[0].request.params[0].value, "2");
    assert_eq!(state.tabs[0].request.params[1].key, "limit");
    assert_eq!(state.tabs[0].request.params[1].value, "50");
}

#[test]
fn test_params_sync_to_url() {
    let mut state = AuraAppState::new();
    let req = AuraRequest::new(
        "Echo GET",
        HttpMethod::GET,
        "https://postman-echo.com/get",
    );
    state.tabs.push(WorkspaceTab::new(req));
    state.active_tab_index = 0;

    state.add_param("filter".to_string(), "active".to_string());
    assert_eq!(
        state.tabs[0].request.url,
        "https://postman-echo.com/get?filter=active"
    );

    state.add_param("sort".to_string(), "desc".to_string());
    assert_eq!(
        state.tabs[0].request.url,
        "https://postman-echo.com/get?filter=active&sort=desc"
    );

    // Update param
    state.update_param(0, "filter".to_string(), "all".to_string());
    assert_eq!(
        state.tabs[0].request.url,
        "https://postman-echo.com/get?filter=all&sort=desc"
    );

    // Remove param
    state.remove_param(1);
    assert_eq!(
        state.tabs[0].request.url,
        "https://postman-echo.com/get?filter=all"
    );
}
