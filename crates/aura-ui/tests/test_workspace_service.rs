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
