use aura::model::{AuraRequest, HttpMethod};
use aura::ui::keybindings::{Action, KeybindingPreset};
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

#[test]
fn test_multi_tab_dirty_state_tracking() {
    let mut state = AuraAppState::new();
    state.new_request();
    let tab_id = state.tabs[0].id.clone();

    // Verify initial clean state
    assert!(!state.tabs[0].is_dirty);

    // Mark tab as dirty
    state.mark_active_tab_dirty(true);
    assert!(state.tabs[0].is_dirty);

    // Saving clears the dirty state
    state.save_active_tab();
    assert!(!state.tabs[0].is_dirty);

    // Dedicated tab mark dirty by ID
    state.mark_tab_dirty(&tab_id, true);
    assert!(state.tabs[0].is_dirty);
    state.mark_tab_dirty(&tab_id, false);
    assert!(!state.tabs[0].is_dirty);
}

#[test]
fn test_multi_tab_navigation_and_close_button() {
    let mut state = AuraAppState::new();
    let req1 = AuraRequest::new("Req 1", HttpMethod::GET, "https://postman-echo.com/get");
    let req2 = AuraRequest::new("Req 2", HttpMethod::POST, "https://postman-echo.com/post");
    let req3 = AuraRequest::new("Req 3", HttpMethod::PUT, "https://postman-echo.com/put");

    state.open_request(req1);
    state.open_request(req2);
    state.open_request(req3);
    assert_eq!(state.tabs.len(), 3);
    assert_eq!(state.active_tab_index, 2);

    // Tab selection
    state.select_tab(0);
    assert_eq!(state.active_tab_index, 0);
    assert!(state.tabs[0].is_active);
    assert!(!state.tabs[1].is_active);

    // Next / Prev tab cycling
    state.next_tab();
    assert_eq!(state.active_tab_index, 1);
    state.next_tab();
    assert_eq!(state.active_tab_index, 2);
    state.next_tab(); // Wraps around
    assert_eq!(state.active_tab_index, 0);

    state.prev_tab(); // Wraps around backwards
    assert_eq!(state.active_tab_index, 2);
    state.prev_tab();
    assert_eq!(state.active_tab_index, 1);

    // Dedicated close active tab
    state.close_active_tab();
    assert_eq!(state.tabs.len(), 2);
    assert_eq!(state.tabs[state.active_tab_index].title, "Req 3");
}

#[test]
fn test_responsive_pane_layout_and_divider_resizer() {
    let mut state = AuraAppState::new();
    assert_eq!(state.split_ratio, 0.5);

    // Adjust divider
    state.set_split_ratio(0.65);
    assert!((state.split_ratio - 0.65).abs() < 1e-5);

    // Clamping limits (0.1 to 0.9)
    state.set_split_ratio(0.05);
    assert_eq!(state.split_ratio, 0.1);

    state.set_split_ratio(0.98);
    assert_eq!(state.split_ratio, 0.9);

    // Double-click reset to 50/50
    state.reset_divider();
    assert_eq!(state.split_ratio, 0.5);

    // Orientation toggle
    assert_eq!(state.orientation, LayoutOrientation::Horizontal);
    state.toggle_orientation();
    assert_eq!(state.orientation, LayoutOrientation::Vertical);
    state.toggle_orientation();
    assert_eq!(state.orientation, LayoutOrientation::Horizontal);
}

#[test]
fn test_command_palette_modal_and_execution() {
    let mut state = AuraAppState::new();
    let req1 = AuraRequest::new(
        "Search Item",
        HttpMethod::GET,
        "https://postman-echo.com/get?q=test",
    );
    let req2 = AuraRequest::new(
        "Create User",
        HttpMethod::POST,
        "https://postman-echo.com/post",
    );
    state.open_request(req1);
    state.open_request(req2);
    state.select_tab(0);

    // Modal state toggling
    assert!(!state.is_command_palette_open);
    state.open_command_palette();
    assert!(state.is_command_palette_open);

    // Search and filter
    state.command_palette_search = "User".to_string();
    let filtered = state.filtered_palette_items();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].title, "Create User");

    // Keyboard navigation in palette
    state.palette_select_next();
    state.palette_select_prev();

    // Execute selected item switches active tab and closes modal
    let executed_id = state.execute_selected_palette_item();
    assert!(executed_id.is_some());
    assert!(!state.is_command_palette_open);
    assert_eq!(state.active_tab_index, 1);
    assert_eq!(state.tabs[state.active_tab_index].title, "Create User");

    // Close command palette directly
    state.open_command_palette();
    state.close_command_palette();
    assert!(!state.is_command_palette_open);
}

#[test]
fn test_http_method_badge_colors() {
    assert_eq!(HttpMethod::GET.badge_color(), "#0066cc"); // Blue
    assert_eq!(HttpMethod::POST.badge_color(), "#28a745"); // Green
    assert_eq!(HttpMethod::PUT.badge_color(), "#f5a623"); // Amber
    assert_eq!(HttpMethod::PATCH.badge_color(), "#8e44ad"); // Purple
    assert_eq!(HttpMethod::DELETE.badge_color(), "#e74c3c"); // Rose
}

#[test]
fn test_keyboard_shortcuts_and_keybinding_presets() {
    let mut state = AuraAppState::new();
    state.new_request();

    // Default preset is VS Code
    assert_eq!(state.keybinding_preset, KeybindingPreset::VsCode);

    // Verify VS Code shortcuts
    assert_eq!(
        state
            .keybinding_preset
            .primary_shortcut(Action::SendRequest),
        "Ctrl+Enter"
    );
    assert_eq!(
        state
            .keybinding_preset
            .primary_shortcut(Action::OpenCommandPalette),
        "Ctrl+P"
    );
    assert_eq!(
        state
            .keybinding_preset
            .primary_shortcut(Action::ToggleLogViewer),
        "Ctrl+Shift+L"
    );

    // Key shortcut resolution for VS Code
    assert_eq!(
        state.keybinding_preset.resolve("Enter", true, false, false),
        Some(Action::SendRequest)
    );
    assert_eq!(
        state.keybinding_preset.resolve("p", true, false, false),
        Some(Action::OpenCommandPalette)
    );
    assert_eq!(
        state.keybinding_preset.resolve("l", true, true, false),
        Some(Action::ToggleLogViewer)
    );

    // Switch to IntelliJ preset
    state.set_keybinding_preset(KeybindingPreset::IntelliJ);
    assert_eq!(state.keybinding_preset, KeybindingPreset::IntelliJ);
    assert_eq!(
        state
            .keybinding_preset
            .primary_shortcut(Action::SendRequest),
        "Ctrl+Enter"
    );
    assert_eq!(
        state
            .keybinding_preset
            .primary_shortcut(Action::OpenCommandPalette),
        "Shift+Shift"
    );
    // IntelliJ also responds to Ctrl+Shift+A
    assert_eq!(
        state.keybinding_preset.resolve("a", true, true, false),
        Some(Action::OpenCommandPalette)
    );

    // Switch to Sublime Text preset
    state.set_keybinding_preset(KeybindingPreset::SublimeText);
    assert_eq!(state.keybinding_preset, KeybindingPreset::SublimeText);
    assert_eq!(
        state
            .keybinding_preset
            .primary_shortcut(Action::OpenCommandPalette),
        "Ctrl+Shift+P"
    );

    // Action handling dispatch
    assert!(!state.is_command_palette_open);
    state.handle_key_action(Action::OpenCommandPalette);
    assert!(state.is_command_palette_open);
    state.handle_key_action(Action::CloseCommandPalette);
    assert!(!state.is_command_palette_open);

    assert_eq!(state.orientation, LayoutOrientation::Horizontal);
    state.handle_key_action(Action::ToggleLayout);
    assert_eq!(state.orientation, LayoutOrientation::Vertical);

    state.split_ratio = 0.8;
    state.handle_key_action(Action::ResetDivider);
    assert_eq!(state.split_ratio, 0.5);
}

#[test]
fn test_workspace_collections_loading_and_activation() {
    let mut state = AuraAppState::new();
    assert!(state.collections.is_empty());

    // Create a temporary directory with a dummy collection file
    let tmp = std::env::temp_dir().join(format!("aura_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&tmp).unwrap();
    let col_path = tmp.join("sample.collection.json");
    let mut req = AuraRequest::new(
        "Live GET Echo",
        HttpMethod::GET,
        "https://postman-echo.com/get",
    );
    req.id = "req-echo-1".to_string();
    let col = aura::model::Collection {
        id: "col-1".to_string(),
        name: "Echo Test Suite".to_string(),
        auth: aura::model::AuthConfig::Inherit,
        variables: std::collections::HashMap::new(),
        requests: vec![req],
        folders: Vec::new(),
    };
    std::fs::write(&col_path, serde_json::to_string_pretty(&col).unwrap()).unwrap();

    let loaded = state.load_workspace_collections(&tmp).unwrap();
    assert_eq!(loaded, 1);
    assert_eq!(state.collections.len(), 1);
    assert_eq!(state.collections[0].name, "Echo Test Suite");

    // Open request from collection
    let opened = state.open_collection_request("col-1", "req-echo-1");
    assert!(opened);
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs[0].title, "Live GET Echo");

    // Opening it again should focus the existing tab, not duplicate
    let reopened = state.open_collection_request("col-1", "req-echo-1");
    assert!(reopened);
    assert_eq!(state.tabs.len(), 1);

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_divider_drag_clamping_and_reset() {
    let mut state = AuraAppState::new();
    assert_eq!(state.split_ratio, 0.50);
    assert!(!state.is_dragging_divider);

    state.start_dragging_divider();
    assert!(state.is_dragging_divider);

    // Drag forward within normal bounds
    state.drag_divider(0.15);
    assert!((state.split_ratio - 0.65).abs() < 1e-5);

    // Drag past upper limit (0.85)
    state.drag_divider(0.50);
    assert_eq!(state.split_ratio, 0.85);

    // Drag past lower limit (0.15)
    state.drag_divider(-0.90);
    assert_eq!(state.split_ratio, 0.15);

    // Double-click reset
    state.reset_divider();
    assert_eq!(state.split_ratio, 0.50);

    state.stop_dragging_divider();
    assert!(!state.is_dragging_divider);
}

#[test]
fn test_close_tab_last_remaining_auto_spawns_untitled() {
    let mut state = AuraAppState::new();
    state.new_request();
    assert_eq!(state.tabs.len(), 1);

    // Close the only tab
    state.close_active_tab();
    // Must auto-spawn an untitled request to prevent null tab state
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs[0].title, "Untitled Request");
    assert_eq!(state.active_tab_index, 0);
}
