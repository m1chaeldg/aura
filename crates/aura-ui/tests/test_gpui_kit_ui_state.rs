use aura_core::model::{AuraRequest, Collection, HttpMethod};
use aura_ui::app_state::AuraAppState;
use aura_ui::stores::LayoutStore;
use aura_ui::views::SidebarView;

#[test]
fn test_layout_store_sidebar_collapse_toggles() {
    let mut store = LayoutStore::new();
    assert!(!store.is_sidebar_collapsed);

    store.toggle_sidebar();
    assert!(store.is_sidebar_collapsed);

    store.toggle_sidebar();
    assert!(!store.is_sidebar_collapsed);
}

#[test]
fn test_app_state_sidebar_collapse_and_drawer_toggle() {
    let mut state = AuraAppState::new();
    assert!(!state.is_sidebar_collapsed);
    assert!(!state.is_log_viewer_open);

    state.toggle_sidebar();
    assert!(state.is_sidebar_collapsed);

    state.toggle_log_viewer();
    assert!(state.is_log_viewer_open);
}

#[test]
fn test_sidebar_view_builds_tree_items_from_collections() {
    let mut state = AuraAppState::new();
    let mut col = Collection {
        id: "api".to_string(),
        name: "API".to_string(),
        requests: Vec::new(),
        folders: Vec::new(),
        variables: Default::default(),
        auth: Default::default(),
    };
    col.requests
        .push(AuraRequest::new("Health", HttpMethod::GET, "https://example.com/health"));
    state.collections.push(col);

    let (items, meta) = SidebarView::build_tree_items(&state);
    assert_eq!(items.len(), 1);
    assert_eq!(meta.len(), 2);
}

#[test]
fn test_closing_last_default_tab_leaves_empty_tabs() {
    let mut state = AuraAppState::new();
    state.new_request();
    assert_eq!(state.tabs.len(), 1);
    let id = state.tabs[0].id.clone();

    state.close_tab(&id);
    assert!(state.tabs.is_empty());
    assert_eq!(state.active_tab_index, 0);
}
