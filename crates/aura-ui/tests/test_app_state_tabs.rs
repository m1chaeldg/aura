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
    assert_eq!(state.active_body_mode, ResponseBodyMode::Compress);
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

#[test]
fn test_top_menu_and_env_dropdown_toggles() {
    use aura_ui::app_state::TopMenuKind;

    let mut state = AuraAppState::new();
    assert_eq!(state.open_top_menu, None);
    state.toggle_top_menu(TopMenuKind::Tools);
    assert_eq!(state.open_top_menu, Some(TopMenuKind::Tools));
    state.toggle_top_menu(TopMenuKind::Tools);
    assert_eq!(state.open_top_menu, None);

    state.toggle_top_menu(TopMenuKind::File);
    assert_eq!(state.open_top_menu, Some(TopMenuKind::File));
    state.close_top_menu();
    assert_eq!(state.open_top_menu, None);

    assert!(!state.is_env_dropdown_open);
    state.toggle_env_dropdown();
    assert!(state.is_env_dropdown_open);
    state.close_env_dropdown();
    assert!(!state.is_env_dropdown_open);
}
