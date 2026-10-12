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
