use aura_core::model::HttpMethod;
use aura_sync::workspace_meta::WorkspaceMetadata;
use aura_ui::stores::workspace_store::{WorkspaceStore, WorkspaceTreeNode};
use std::path::PathBuf;

#[test]
fn test_workspace_store_tree_expansion_and_mutations() {
    let mut store = WorkspaceStore::new();
    let root = PathBuf::from("/workspace");
    let folder_path = root.join("Auth");
    let req_path = folder_path.join("Login.aura");

    let tree = vec![WorkspaceTreeNode::Folder {
        name: "Auth".to_string(),
        path: folder_path.clone(),
        is_expanded: false,
        children: vec![WorkspaceTreeNode::Request {
            name: "Login.aura".to_string(),
            path: req_path.clone(),
            method: HttpMethod::POST,
        }],
    }];

    store.set_workspace(root.clone(), WorkspaceMetadata::new("Test Workspace"), tree);
    assert_eq!(store.tree.len(), 1);

    // Toggle expand
    store.toggle_folder_expanded(&folder_path);
    if let WorkspaceTreeNode::Folder { is_expanded, .. } = &store.tree[0] {
        assert!(*is_expanded);
    } else {
        panic!("expected folder");
    }

    // Rename
    let new_req_path = folder_path.join("SignIn.aura");
    store.apply_rename(&req_path, &new_req_path, "SignIn.aura");
    if let WorkspaceTreeNode::Folder { children, .. } = &store.tree[0] {
        assert_eq!(children[0].name(), "SignIn.aura");
        assert_eq!(children[0].path(), new_req_path);
    }

    // Delete
    store.apply_delete(&new_req_path);
    if let WorkspaceTreeNode::Folder { children, .. } = &store.tree[0] {
        assert!(children.is_empty());
    }
}
