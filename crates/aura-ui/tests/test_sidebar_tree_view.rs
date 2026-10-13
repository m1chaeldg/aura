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
                assert!(children.len() >= 5, "Folder should contain all requests and subfolders");

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

#[test]
fn test_context_menu_action_state_transitions() {
    let mut state = AuraAppState::new();
    let target = PathBuf::from("/ws/Auth/Login.aura");

    state.workspace_store.open_context_menu(target.clone(), false, (100.0, 150.0));
    assert!(state.workspace_store.active_context_menu.is_some());

    let menu = state.workspace_store.active_context_menu.as_ref().unwrap();
    assert_eq!(menu.target_path, target);
    assert_eq!(menu.position, (100.0, 150.0));

    state.workspace_store.close_context_menu();
    assert!(state.workspace_store.active_context_menu.is_none());
}

#[test]
fn test_sidebar_node_selection_and_action_menu_triggers() {
    use aura_core::model::HttpMethod;
    use aura_ui::stores::workspace_store::WorkspaceTreeNode;

    let mut state = AuraAppState::new();
    let req_path = PathBuf::from("/ws/GetUsers.aura");
    state.workspace_store.tree = vec![WorkspaceTreeNode::Request {
        name: "GetUsers.aura".to_string(),
        path: req_path.clone(),
        method: HttpMethod::GET,
    }];

    state.workspace_store.select_node(req_path.clone());
    assert_eq!(state.workspace_store.selected_path.as_ref(), Some(&req_path));
}

#[test]
fn test_sidebar_keyboard_shortcuts_resolution() {
    use aura_ui::keybindings::{Action, KeybindingPreset};

    let preset = KeybindingPreset::VsCode;

    assert_eq!(preset.resolve("up", false, false, false), Some(Action::TreeUp));
    assert_eq!(preset.resolve("down", false, false, false), Some(Action::TreeDown));
    assert_eq!(preset.resolve("right", false, false, false), Some(Action::TreeExpand));
    assert_eq!(preset.resolve("left", false, false, false), Some(Action::TreeCollapse));
    assert_eq!(preset.resolve("d", true, false, false), Some(Action::DuplicateItem));
    assert_eq!(preset.resolve("c", true, false, false), Some(Action::CopyItem));
    assert_eq!(preset.resolve("v", true, false, false), Some(Action::PasteItem));
    assert_eq!(preset.resolve("f2", false, false, false), Some(Action::RenameItem));
    assert_eq!(preset.resolve("delete", false, false, false), Some(Action::DeleteItem));
}

#[test]
fn test_sidebar_drag_placement_calculation() {
    use aura_ui::stores::workspace_store::{DragPlacement, SidebarDragState};
    use std::path::PathBuf;

    let drag = SidebarDragState {
        dragging_path: PathBuf::from("/ws/Auth/Login.aura"),
        hover_target_path: PathBuf::from("/ws/Auth/Register.aura"),
        placement: DragPlacement::Below,
    };
    assert_eq!(drag.placement, DragPlacement::Below);
}

#[tokio::test]
async fn test_sidebar_reorder_item_integration() {
    use aura_core::model::HttpMethod;
    use aura_ui::services::workspace_service::WorkspaceService;

    let tmp = std::env::temp_dir().join(format!("aura_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&tmp).unwrap();
    let root = tmp.as_path();

    let service = WorkspaceService::new();
    let r1 = service
        .create_request(root, root, "Req1", HttpMethod::GET)
        .await
        .unwrap();
    let folder = service.create_folder(root, root, "FolderA").await.unwrap();

    // Reorder r1 into folder
    service
        .reorder_item(root, &r1, &folder, 0)
        .await
        .unwrap();
    assert!(folder.join("Req1.aura").exists());
    assert!(!r1.exists());

    let _ = std::fs::remove_dir_all(&tmp);
}


