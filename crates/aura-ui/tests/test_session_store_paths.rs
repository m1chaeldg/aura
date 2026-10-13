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

#[tokio::test]
async fn test_app_state_delete_confirmation_and_tab_cleanup() {
    use aura_ui::app_state::AuraAppState;

    let temp_root = std::env::temp_dir().join(format!("aura_test_del_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_root).unwrap();
    let root = temp_root.as_path();

    let mut state = AuraAppState::new();
    state.open_workspace_sync(root).unwrap();

    // Create folder with two requests
    let folder = state.workspace_service.create_folder(root, root, "Nested").await.unwrap();
    let r1 = state.workspace_service.create_request(root, &folder, "Req1", HttpMethod::GET).await.unwrap();
    let r2 = state.workspace_service.create_request(root, &folder, "Req2", HttpMethod::POST).await.unwrap();

    // Open both requests in tabs
    let _ = state.open_file_in_tab(&r1);
    let _ = state.open_file_in_tab(&r2);
    assert_eq!(state.tabs.len(), 2);

    // Request deletion of folder
    state.request_delete(folder.clone());
    assert_eq!(state.pending_delete_path.as_ref(), Some(&folder));

    // Confirm deletion: disk deleted + both descendant tabs closed (Review Focus #5)
    state.confirm_delete().await.unwrap();
    assert!(!folder.exists());
    assert!(state.pending_delete_path.is_none());

    // Tabs closed and automatically replaced with fallback Untitled Request
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs[0].title, "Untitled Request");

    let _ = std::fs::remove_dir_all(&temp_root);
}


