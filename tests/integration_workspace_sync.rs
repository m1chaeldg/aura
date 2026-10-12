use aura_ui::app_state::AuraAppState;
use std::fs;
use std::path::{Path, PathBuf};

struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!("aura_test_e2e_{}", uuid::Uuid::new_v4()));
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
async fn test_e2e_workspace_loading_and_external_sync_cycle() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();

    // Set up disk files
    fs::create_dir_all(root.join("Auth")).unwrap();
    fs::write(root.join("Auth").join("Login.aura"), "[request]\nname = \"Login\"\nmethod = \"POST\"\nurl = \"https://echo.com\"").unwrap();

    let mut state = AuraAppState::new();
    state.open_workspace_directory(root).await.expect("open workspace");

    // Verify workspace loaded into store
    assert_eq!(state.workspace_store.tree.len(), 1);

    // Open the request into a tab
    let req_path = root.join("Auth").join("Login.aura");
    state.open_file_in_tab(&req_path).expect("open tab");
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs[0].file_path, Some(req_path.clone()));

    // Simulate external deletion of Login.aura
    fs::remove_file(&req_path).unwrap();

    // Trigger sync cycle
    state.reconcile_workspace_with_disk().await.expect("reconcile");

    // Request tab should have been closed automatically, falling back to Untitled Request
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs[0].title, "Untitled Request");
}
