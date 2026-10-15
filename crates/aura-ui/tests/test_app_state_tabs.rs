use aura_ui::app_state::AuraAppState;
use aura_ui::stores::{EditorTab, ResponseBodyMode, ResponseTab};

#[test]
fn test_app_state_editor_and_response_tab_switching() {
    let mut state = AuraAppState::new();
    assert_eq!(state.active_editor_tab, EditorTab::Params);
    assert_eq!(state.active_response_tab, ResponseTab::Body);
    assert_eq!(state.active_body_mode, ResponseBodyMode::Pretty);

    state.set_editor_tab(EditorTab::Headers);
    assert_eq!(state.active_editor_tab, EditorTab::Headers);

    state.set_editor_tab(EditorTab::Body);
    assert_eq!(state.active_editor_tab, EditorTab::Body);

    state.set_response_tab(ResponseTab::Console);
    assert_eq!(state.active_response_tab, ResponseTab::Console);

    state.toggle_response_body_mode();
    assert_eq!(state.active_body_mode, ResponseBodyMode::Raw);
}

#[test]
fn test_tab_close_aborts_in_flight_request() {
    let mut state = AuraAppState::new();
    state.new_request();
    let tab_id = state.tabs[0].id.clone();

    let token = state.execution_service.cancellation_token();
    assert!(!token.is_cancelled());

    // Closing tab cancels the token
    state.close_tab(&tab_id);
    assert!(token.is_cancelled());
}
