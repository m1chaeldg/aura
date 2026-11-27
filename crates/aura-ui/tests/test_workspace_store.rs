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

#[test]
fn test_load_workspace_aura_files_with_parser() {
    let temp_dir = std::env::temp_dir().join(format!("aura_test_{}", uuid::Uuid::new_v4()));
    let collection_dir = temp_dir.join("Users");
    std::fs::create_dir_all(&collection_dir).unwrap();

    let aura_content = r#"version = "1.0"
name = "Create User Account"
method = "POST"
url = "https://api.example.com/users"

[settings]
inherit_auth = true
verify_ssl = true
follow_redirects = true
timeout_ms = 15000

[headers]
Content-Type = "application/json"
Accept = "application/json"

[body.json]
```json
{
  "username": "aura_user"
}
```
"#;

    let req_file = collection_dir.join("create_user.aura");
    std::fs::write(&req_file, aura_content).unwrap();

    let mut store = WorkspaceStore::new();
    let loaded = store.load_workspace_collections(&temp_dir).unwrap();
    assert_eq!(loaded, 1);
    assert_eq!(store.collections.len(), 1);

    let req = &store.collections[0].requests[0];
    assert_eq!(req.name, "Create User Account");
    assert_eq!(req.method, HttpMethod::POST);
    assert_eq!(req.url, "https://api.example.com/users");
    assert!(req
        .headers
        .iter()
        .any(|h| h.key == "Content-Type" && h.value == "application/json"));
    if let aura_core::model::BodyType::Json(body_str) = &req.body {
        assert!(body_str.contains("aura_user"));
    } else {
        panic!(
            "Expected Json body parsed from fenced block, got {:?}",
            req.body
        );
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_workspace_store_visible_nodes_and_keyboard_traversal() {
    use aura_ui::stores::workspace_store::WorkspaceTreeNode;

    let mut store = WorkspaceStore::new();
    let folder1 = WorkspaceTreeNode::Folder {
        name: "Auth".to_string(),
        path: PathBuf::from("/ws/Auth"),
        is_expanded: false,
        children: vec![WorkspaceTreeNode::Request {
            name: "Login.aura".to_string(),
            path: PathBuf::from("/ws/Auth/Login.aura"),
            method: HttpMethod::POST,
        }],
    };
    let req2 = WorkspaceTreeNode::Request {
        name: "Health.aura".to_string(),
        path: PathBuf::from("/ws/Health.aura"),
        method: HttpMethod::GET,
    };
    store.tree = vec![folder1, req2];

    // Collapsed folder: only 2 visible nodes (Auth, Health.aura)
    let visible = store.visible_nodes();
    assert_eq!(visible.len(), 2);
    assert_eq!(visible[0].path(), PathBuf::from("/ws/Auth").as_path());
    assert_eq!(
        visible[1].path(),
        PathBuf::from("/ws/Health.aura").as_path()
    );

    // Traversal: start at top
    store.select_node(PathBuf::from("/ws/Auth"));
    assert_eq!(
        store.selected_path.as_deref(),
        Some(PathBuf::from("/ws/Auth").as_path())
    );

    store.select_next_node();
    assert_eq!(
        store.selected_path.as_deref(),
        Some(PathBuf::from("/ws/Health.aura").as_path())
    );

    // Next at end stays clamped
    store.select_next_node();
    assert_eq!(
        store.selected_path.as_deref(),
        Some(PathBuf::from("/ws/Health.aura").as_path())
    );

    // Expand Auth folder
    store.select_node(PathBuf::from("/ws/Auth"));
    store.expand_selected();
    let visible_after_expand = store.visible_nodes();
    assert_eq!(visible_after_expand.len(), 3);
    assert_eq!(
        visible_after_expand[1].path(),
        PathBuf::from("/ws/Auth/Login.aura").as_path()
    );

    // Navigate into expanded child
    store.select_next_node();
    assert_eq!(
        store.selected_path.as_deref(),
        Some(PathBuf::from("/ws/Auth/Login.aura").as_path())
    );

    // Collapse
    store.select_node(PathBuf::from("/ws/Auth"));
    store.collapse_selected();
    assert_eq!(store.visible_nodes().len(), 2);
}

#[test]
fn test_workspace_store_clipboard_and_context_menu_state() {
    use aura_ui::stores::workspace_store::ClipboardPayload;

    let mut store = WorkspaceStore::new();
    let req_path = PathBuf::from("/ws/Auth/Login.aura");
    store.select_node(req_path.clone());
    store.copy_selected();

    match &store.clipboard_item {
        Some(ClipboardPayload::Request(p)) => assert_eq!(p, &req_path),
        _ => panic!("Expected ClipboardPayload::Request"),
    }

    store.open_context_menu(req_path.clone(), false, (120.0, 240.0));
    assert!(store.active_context_menu.is_some());
    let menu = store.active_context_menu.as_ref().unwrap();
    assert_eq!(menu.target_path, req_path);
    assert!(!menu.is_folder);
    assert_eq!(menu.position, (120.0, 240.0));

    store.close_context_menu();
    assert!(store.active_context_menu.is_none());
}

#[test]
fn test_workspace_store_excludes_environment_folders_from_collections_and_tree() {
    use aura_ui::app_state::AuraAppState;
    use aura_ui::views::SidebarView;

    let temp_dir = std::env::temp_dir().join(format!("aura_env_skip_{}", uuid::Uuid::new_v4()));
    let users_dir = temp_dir.join("Users");
    let env_dir = temp_dir.join("environment");
    let envs_dir = temp_dir.join("environments");
    std::fs::create_dir_all(&users_dir).unwrap();
    std::fs::create_dir_all(&env_dir).unwrap();
    std::fs::create_dir_all(&envs_dir).unwrap();

    std::fs::write(
        users_dir.join("profile.aura"),
        "version = \"1.0\"\nname = \"Profile\"\nmethod = \"GET\"\nurl = \"https://example.com\"\n",
    )
    .unwrap();
    std::fs::write(
        env_dir.join("Production.toml"),
        "name = \"Production\"\n\n[variables]\nAPI = \"https://prod.example.com\"\n",
    )
    .unwrap();
    std::fs::write(
        envs_dir.join("Staging.toml"),
        "name = \"Staging\"\n\n[variables]\nAPI = \"https://stg.example.com\"\n",
    )
    .unwrap();

    let mut store = WorkspaceStore::new();
    let loaded = store.load_workspace_collections(&temp_dir).unwrap();
    assert_eq!(loaded, 1);
    assert_eq!(store.collections.len(), 1);
    assert_eq!(store.collections[0].name, "Users");

    // Defense-in-depth: SidebarView::build_tree_items also filters out environment folders
    let mut state = AuraAppState::new();
    state.workspace_store.tree = vec![
        WorkspaceTreeNode::Folder {
            name: "environment".to_string(),
            path: env_dir.clone(),
            is_expanded: true,
            children: vec![WorkspaceTreeNode::Request {
                name: "Production.toml".to_string(),
                path: env_dir.join("Production.toml"),
                method: HttpMethod::GET,
            }],
        },
        WorkspaceTreeNode::Folder {
            name: "Users".to_string(),
            path: users_dir.clone(),
            is_expanded: true,
            children: Vec::new(),
        },
    ];
    let (items, meta) = SidebarView::build_tree_items(&state);
    assert_eq!(items.len(), 1);
    assert!(!meta.values().any(|m| match m {
        aura_ui::views::sidebar_view::SidebarNodeMeta::Folder { name, .. } => name == "environment",
        aura_ui::views::sidebar_view::SidebarNodeMeta::Request { name, .. } => name == "Production.toml",
    }));

    let _ = std::fs::remove_dir_all(&temp_dir);
}
