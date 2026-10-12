use aura_core::model::{AuraRequest, HttpMethod};
use aura_ui::stores::SessionStore;
use std::path::PathBuf;

#[test]
fn test_open_request_file_prevents_duplicates() {
    // Review Focus #2
    let mut store = SessionStore::new();
    let req1 = AuraRequest::new("Login", HttpMethod::POST, "https://api.example.com/login");
    let path = PathBuf::from("/workspace/Auth/Login.aura");

    store.open_request_file(req1.clone(), path.clone());
    assert_eq!(store.tabs.len(), 1);
    assert_eq!(store.active_tab_index, 0);

    // Opening same path again must activate existing tab, not duplicate
    store.new_request(); // opens second tab
    assert_eq!(store.tabs.len(), 2);
    assert_eq!(store.active_tab_index, 1);

    store.open_request_file(req1, path);
    assert_eq!(store.tabs.len(), 2); // No new tab added
    assert_eq!(store.active_tab_index, 0); // Jumped back to tab 0
}

#[test]
fn test_handle_item_renamed_updates_tab_title_and_path() {
    let mut store = SessionStore::new();
    let req = AuraRequest::new("Login", HttpMethod::POST, "https://api.example.com/login");
    let old_path = PathBuf::from("/workspace/Auth/Login.aura");
    let new_path = PathBuf::from("/workspace/Auth/SignIn.aura");

    store.open_request_file(req, old_path.clone());
    store.handle_item_renamed(&old_path, &new_path, "SignIn.aura");

    assert_eq!(store.tabs[0].title, "SignIn.aura");
    assert_eq!(store.tabs[0].file_path, Some(new_path));
}

#[test]
fn test_handle_folder_renamed_updates_descendant_tab_paths() {
    // Review Focus #4
    let mut store = SessionStore::new();
    let req = AuraRequest::new("Login", HttpMethod::POST, "https://api.example.com/login");
    let old_folder = PathBuf::from("/workspace/Auth");
    let new_folder = PathBuf::from("/workspace/Authentication");
    let file_path = old_folder.join("Login.aura");

    store.open_request_file(req, file_path);
    store.handle_item_renamed(&old_folder, &new_folder, "Authentication");

    assert_eq!(store.tabs[0].file_path, Some(new_folder.join("Login.aura")));
}

#[test]
fn test_handle_item_deleted_closes_tab_and_spawns_fallback_if_empty() {
    // Review Focus #3
    let mut store = SessionStore::new();
    let req = AuraRequest::new("Login", HttpMethod::POST, "https://api.example.com/login");
    let path = PathBuf::from("/workspace/Auth/Login.aura");

    store.open_request_file(req, path.clone());
    assert_eq!(store.tabs.len(), 1);

    store.handle_item_deleted(&path);
    // Sole tab closed -> auto-spawns Untitled Request
    assert_eq!(store.tabs.len(), 1);
    assert_eq!(store.tabs[0].title, "Untitled Request");
    assert_eq!(store.tabs[0].file_path, None);
}
