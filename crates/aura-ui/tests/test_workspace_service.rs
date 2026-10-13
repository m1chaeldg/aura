use aura_core::model::HttpMethod;
use aura_ui::services::workspace_service::WorkspaceService;
use std::fs;
use std::path::{Path, PathBuf};

struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!("aura_test_ws_srv_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&p).expect("create temp dir");
        Self(p)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn tempdir() -> Result<TempDir, std::io::Error> {
    Ok(TempDir::new())
}

#[tokio::test]
async fn test_workspace_service_async_crud_and_metadata_sync() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    let service = WorkspaceService::new();

    // Create folder
    let folder_path = service.create_folder(root, root, "Auth").await.expect("create folder");
    assert!(folder_path.is_dir());

    // Create request
    let req_path = service.create_request(root, &folder_path, "Login", HttpMethod::POST).await.expect("create request");
    assert!(req_path.is_file());
    assert!(req_path.to_string_lossy().ends_with("Login.aura"));

    // Verify workspace.json was updated
    let meta_content = fs::read_to_string(root.join("workspace.json")).expect("read meta");
    assert!(meta_content.contains("Login.aura"));

    // Rename request
    let renamed_path = service.rename_item(root, &req_path, "SignIn.aura").await.expect("rename");
    assert!(renamed_path.exists());
    assert!(!req_path.exists());

    // Delete request
    service.delete_item(root, &renamed_path).await.expect("delete");
    assert!(!renamed_path.exists());
}

#[tokio::test]
async fn test_watcher_echo_suppression() {
    // Review Focus #5
    let service = WorkspaceService::new();
    let path = std::path::PathBuf::from("/workspace/Auth/Login.aura");

    service.suppress_path(&path);
    assert!(service.is_suppressed(&path));
}

#[tokio::test]
async fn test_watcher_external_disk_sync_flow() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();

    let mut state = aura_ui::app_state::AuraAppState::new();
    state.open_workspace_sync(root).expect("open workspace");
    assert!(state.workspace_store.tree.is_empty());

    let (cancel_token, mut rx) = state.workspace_service.start_watcher(root).expect("start watcher");

    // 1. External creation: simulate Windows Explorer pasting a folder and file
    let folder_path = root.join("Payments");
    fs::create_dir_all(&folder_path).unwrap();
    let file_path = folder_path.join("Charge.aura");
    fs::write(&file_path, "[request]\nname = \"Charge\"\nmethod = \"POST\"\nurl = \"https://api.com\"").unwrap();

    // Watcher should emit an event
    let event = tokio::time::timeout(std::time::Duration::from_secs(3), rx.recv()).await;
    assert!(event.is_ok(), "Watcher timed out waiting for external disk change event");
    assert!(event.unwrap().is_some());

    // Debounce drain simulation
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    while rx.try_recv().is_ok() {}

    // Reconcile
    state.reconcile_workspace_sync().expect("reconcile");

    // Verify Payments folder and Charge.aura appear in tree
    assert_eq!(state.workspace_store.tree.len(), 1);
    let folder = &state.workspace_store.tree[0];
    assert_eq!(folder.name(), "Payments");
    if let aura_ui::stores::workspace_store::WorkspaceTreeNode::Folder { children, .. } = folder {
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name(), "Charge.aura");
    } else {
        panic!("Expected folder");
    }

    // 2. External deletion: simulate deleting file in Windows Explorer
    fs::remove_file(&file_path).unwrap();
    let del_event = tokio::time::timeout(std::time::Duration::from_secs(3), rx.recv()).await;
    assert!(del_event.is_ok(), "Watcher timed out waiting for delete event");
    assert!(del_event.unwrap().is_some());

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    while rx.try_recv().is_ok() {}

    state.reconcile_workspace_sync().expect("reconcile delete");
    let folder_after = &state.workspace_store.tree[0];
    if let aura_ui::stores::workspace_store::WorkspaceTreeNode::Folder { children, .. } = folder_after {
        assert!(children.is_empty(), "Folder should be empty after deleting file");
    }

    cancel_token.cancel();
}

#[tokio::test]
async fn test_workspace_service_duplicate_collision_naming() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    let service = WorkspaceService::new();

    // 1. Initial request: Login.aura
    let login_path = service
        .create_request(root, root, "Login", HttpMethod::POST)
        .await
        .expect("create request");
    assert!(login_path.exists());

    // 2. First duplicate: Login (Copy).aura
    let dup1 = service.duplicate_item(root, &login_path).await.expect("dup1");
    assert_eq!(dup1.file_name().unwrap(), "Login (Copy).aura");
    assert!(dup1.exists());

    // 3. Second duplicate of original: Login (Copy 2).aura (Review Focus #1)
    let dup2 = service.duplicate_item(root, &login_path).await.expect("dup2");
    assert_eq!(dup2.file_name().unwrap(), "Login (Copy 2).aura");
    assert!(dup2.exists());

    // 4. Duplicate of a folder recursively (Review Focus #2)
    let folder = service.create_folder(root, root, "Auth").await.expect("folder");
    let _ = service
        .create_request(root, &folder, "Token", HttpMethod::GET)
        .await
        .expect("token");
    let dup_folder = service.duplicate_item(root, &folder).await.expect("dup folder");
    assert_eq!(dup_folder.file_name().unwrap(), "Auth (Copy)");
    assert!(dup_folder.join("Token.aura").exists());
}

#[tokio::test]
async fn test_workspace_service_paste_item_and_missing_source() {
    use aura_ui::stores::workspace_store::ClipboardPayload;

    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    let service = WorkspaceService::new();

    let folder_a = service.create_folder(root, root, "FolderA").await.unwrap();
    let folder_b = service.create_folder(root, root, "FolderB").await.unwrap();
    let file_a = service
        .create_request(root, &folder_a, "Endpoint", HttpMethod::GET)
        .await
        .unwrap();

    // Paste file from FolderA into FolderB
    let payload = ClipboardPayload::Request(file_a.clone());
    let pasted = service.paste_item(root, &payload, &folder_b).await.unwrap();
    assert_eq!(pasted, folder_b.join("Endpoint.aura"));
    assert!(pasted.exists());

    // Paste with missing source error (Review Focus #3)
    let fake_payload = ClipboardPayload::Request(root.join("NonExistent.aura"));
    let err = service.paste_item(root, &fake_payload, &folder_b).await;
    assert!(err.is_err(), "Expected error when pasting missing source");
}


