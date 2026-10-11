use aura::model::{AuraRequest, HttpMethod};
use aura::ui::{AuraAppState, LayoutOrientation};

#[test]
fn test_ui_workspace_tab_lifecycle() {
    let mut state = AuraAppState::new();
    assert_eq!(state.tabs.len(), 0);

    // Persistent + action
    state.new_request();
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs[0].title, "Untitled Request");
    assert!(state.tabs[0].is_active);

    // Open another request
    let req2 = AuraRequest::new(
        "Fetch Users",
        HttpMethod::GET,
        "https://api.example.com/users",
    );
    state.open_request(req2);
    assert_eq!(state.tabs.len(), 2);
    assert_eq!(state.active_tab_index, 1);

    // Command palette search
    state.command_palette_search = "Fetch".to_string();
    let results = state.filtered_palette_items();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Fetch Users");

    // Divider reset to 50/50
    state.split_ratio = 0.75;
    state.reset_divider();
    assert_eq!(state.split_ratio, 0.5);

    // Orientation toggle
    assert_eq!(state.orientation, LayoutOrientation::Horizontal);
    state.toggle_orientation();
    assert_eq!(state.orientation, LayoutOrientation::Vertical);

    // Close tab
    let id_to_close = state.tabs[0].id.clone();
    state.close_tab(&id_to_close);
    assert_eq!(state.tabs.len(), 1);
}
