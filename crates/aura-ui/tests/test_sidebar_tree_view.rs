use aura_ui::app_state::AuraAppState;
use std::path::PathBuf;

#[test]
fn test_sidebar_state_supports_tree_and_inline_rename() {
    let mut state = AuraAppState::new();
    assert!(state.inline_rename.is_none());

    let target_path = PathBuf::from("/workspace/Auth/Login.aura");
    state.start_inline_rename(target_path.clone(), "Login.aura");
    assert!(state.inline_rename.is_some());
    assert_eq!(state.inline_rename.as_ref().unwrap().path, target_path);

    state.cancel_inline_rename();
    assert!(state.inline_rename.is_none());
}

#[test]
fn test_open_space1_reconciles_and_builds_tree() {
    let mut state = AuraAppState::new();
    let root = PathBuf::from(r"c:\repo\aura\workspaces\space1");
    if root.exists() {
        state.open_workspace_sync(&root).unwrap();
        assert!(!state.workspace_store.tree.is_empty(), "Tree should not be empty");

        let folder = &state.workspace_store.tree[0];
        match folder {
            aura_ui::stores::workspace_store::WorkspaceTreeNode::Folder {
                name,
                path,
                is_expanded,
                children,
            } => {
                assert_eq!(name, "Users & Accounts");
                assert!(*is_expanded, "Folder should default to expanded on initial load");
                assert_eq!(children.len(), 4, "Folder should contain all 4 requests");

                let folder_path = path.clone();

                // Test toggle_folder to collapse
                state.toggle_folder(&folder_path);
                if let aura_ui::stores::workspace_store::WorkspaceTreeNode::Folder { is_expanded, .. } =
                    &state.workspace_store.tree[0]
                {
                    assert!(!*is_expanded, "Folder should now be collapsed");
                }

                // Test toggle_folder to re-expand
                state.toggle_folder(&folder_path);
                if let aura_ui::stores::workspace_store::WorkspaceTreeNode::Folder { is_expanded, .. } =
                    &state.workspace_store.tree[0]
                {
                    assert!(*is_expanded, "Folder should be re-expanded");
                }
            }
            _ => panic!("Expected Folder node at root of tree"),
        }
    }
}
