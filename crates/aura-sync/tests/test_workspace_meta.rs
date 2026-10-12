use aura_sync::workspace_meta::{WorkspaceMetadata, WorkspaceOrderItem};
use std::fs;
use std::path::{Path, PathBuf};

struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!("aura_test_{}", uuid::Uuid::new_v4()));
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

#[test]
fn test_workspace_metadata_serde_schema() {
    let json_sample = r#"{
      "version": 1,
      "name": "My Workspace",
      "order": [
        {
          "type": "folder",
          "name": "Auth",
          "order": [
            { "type": "request", "name": "Login.aura" },
            { "type": "request", "name": "Refresh.aura" }
          ]
        },
        {
          "type": "folder",
          "name": "Users",
          "order": [
            { "type": "request", "name": "Get Profile.aura" }
          ]
        },
        { "type": "request", "name": "Health Check.aura" }
      ]
    }"#;

    let meta: WorkspaceMetadata = serde_json::from_str(json_sample).expect("valid parse");
    assert_eq!(meta.version, 1);
    assert_eq!(meta.name, "My Workspace");
    assert_eq!(meta.order.len(), 3);

    let serialized = serde_json::to_string_pretty(&meta).expect("serialize");
    let back: WorkspaceMetadata = serde_json::from_str(&serialized).expect("deserialize back");
    assert_eq!(meta, back);
}

#[test]
fn test_reconcile_with_disk_discovers_new_files_and_cleans_orphans() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();

    // Create folders and files
    fs::create_dir_all(root.join("Auth")).unwrap();
    fs::write(root.join("Auth").join("Login.aura"), "").unwrap();
    fs::write(root.join("Health.aura"), "").unwrap();

    // Initial reconcile with no existing metadata
    let meta = WorkspaceMetadata::reconcile_with_disk(root, None).expect("reconcile");
    assert_eq!(meta.version, 1);
    assert_eq!(meta.order.len(), 2);

    // Save to disk
    meta.save_to_disk(root).expect("save");
    assert!(root.join("workspace.json").exists());

    // External change: add Users/Profile.aura and delete Health.aura
    fs::create_dir_all(root.join("Users")).unwrap();
    fs::write(root.join("Users").join("Profile.aura"), "").unwrap();
    fs::remove_file(root.join("Health.aura")).unwrap();

    // Reconcile again with existing metadata
    let updated = WorkspaceMetadata::reconcile_with_disk(root, Some(meta)).expect("reconcile update");
    // Health.aura should be gone; Users should be added; Auth should remain
    let has_health = updated.order.iter().any(|item| matches!(item, WorkspaceOrderItem::Request { name } if name == "Health.aura"));
    let has_users = updated.order.iter().any(|item| matches!(item, WorkspaceOrderItem::Folder { name, .. } if name == "Users"));
    assert!(!has_health);
    assert!(has_users);
}

#[test]
fn test_reconcile_empty_or_nonexistent_directory_does_not_crash() {
    // Review Focus #1
    let tmp = tempdir().expect("tempdir");
    let missing_path = tmp.path().join("does_not_exist");
    let meta = WorkspaceMetadata::reconcile_with_disk(&missing_path, None).expect("reconcile missing");
    assert_eq!(meta.version, 1);
    assert!(meta.order.is_empty());
}

#[test]
fn test_workspace_meta_item_mutations() {
    let mut meta = WorkspaceMetadata::new("Mutations");
    meta.order = vec![
        WorkspaceOrderItem::Folder {
            name: "Auth".to_string(),
            order: vec![
                WorkspaceOrderItem::Request { name: "Login.aura".to_string() },
                WorkspaceOrderItem::Request { name: "Signup.aura".to_string() },
            ],
        },
        WorkspaceOrderItem::Request { name: "Root.aura".to_string() },
    ];

    // Reorder inside Auth
    meta.reorder_item(Path::new("Auth"), 0, 1).expect("reorder");
    if let WorkspaceOrderItem::Folder { order, .. } = &meta.order[0] {
        assert_eq!(order[0].name(), "Signup.aura");
        assert_eq!(order[1].name(), "Login.aura");
    }

    // Rename
    meta.rename_item(Path::new("Auth/Login.aura"), "SignIn.aura").expect("rename");
    if let WorkspaceOrderItem::Folder { order, .. } = &meta.order[0] {
        assert_eq!(order[1].name(), "SignIn.aura");
    }

    // Move from Auth to root
    meta.move_item(Path::new("Auth/SignIn.aura"), Path::new(""), 0).expect("move");
    assert_eq!(meta.order[0].name(), "SignIn.aura");

    // Remove
    meta.remove_item(Path::new("Root.aura")).expect("remove");
    assert_eq!(meta.order.len(), 2);
}
