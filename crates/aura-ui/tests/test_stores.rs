use aura_core::model::{AuraRequest, HttpMethod};
use aura_ui::services::FormatService;
use aura_ui::stores::{LayoutOrientation, LayoutStore, SessionStore};

#[test]
fn test_session_store_lifecycle() {
    let mut store = SessionStore::new();
    assert_eq!(store.tabs.len(), 0);

    store.new_request();
    assert_eq!(store.tabs.len(), 1);
    assert_eq!(store.tabs[0].title, "Untitled Request");
    assert!(!store.tabs[0].is_dirty);

    // Open another request
    let req2 = AuraRequest::new("User API", HttpMethod::GET, "https://api.example.com/users");
    store.open_request(req2);
    assert_eq!(store.tabs.len(), 2);
    assert_eq!(store.active_tab_index, 1);

    // Mark dirty
    store.mark_active_tab_dirty(true);
    assert!(store.tabs[1].is_dirty);

    // Close tab with fallback
    let id0 = store.tabs[0].id.clone();
    store.close_tab(&id0);
    assert_eq!(store.tabs.len(), 1);
    assert_eq!(store.active_tab_index, 0);

    // Close remaining tab auto-spawns untitled
    let id1 = store.tabs[0].id.clone();
    store.close_tab(&id1);
    assert_eq!(store.tabs.len(), 1);
    assert_eq!(store.tabs[0].title, "Untitled Request");
}

#[test]
fn test_layout_store_divider_clamping_and_reset() {
    let mut store = LayoutStore::new();
    assert_eq!(store.split_ratio, 0.5);

    store.set_split_ratio(0.99);
    assert_eq!(store.split_ratio, 0.9);

    store.set_split_ratio(0.01);
    assert_eq!(store.split_ratio, 0.1);

    store.reset_divider();
    assert_eq!(store.split_ratio, 0.5);

    assert_eq!(store.orientation, LayoutOrientation::Horizontal);
    store.toggle_orientation();
    assert_eq!(store.orientation, LayoutOrientation::Vertical);
}

#[test]
fn test_format_service() {
    let raw = "{\"name\":\"Aura\",\"version\":\"0.1.0\"}";
    let formatted = FormatService::format_json(raw).expect("valid json format");
    assert!(formatted.contains("\n"));
}
