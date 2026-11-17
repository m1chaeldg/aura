use aura_core::model::HttpMethod;
use aura_ui::app_state::{ActiveInputTarget, AuraAppState};
use aura_ui::preferences::AuraPreferences;
use aura_ui::services::workspace_service::WorkspaceService;
use aura_ui::views::RequestHeaderView;
use std::fs;
use std::path::{Path, PathBuf};

struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!("aura_test_autosave_{}", uuid::Uuid::new_v4()));
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
fn test_request_dirty_state_and_save_button_visibility() {
    let mut state = AuraAppState::new();
    state.new_request();

    // 1. Initial state: request is clean
    assert!(!state.is_active_tab_dirty());
    assert!(!RequestHeaderView::is_save_button_visible(state.is_active_tab_dirty()));

    // 2. Modifying HTTP method marks request dirty -> Save button visible
    state.cycle_method();
    assert!(state.is_active_tab_dirty());
    assert!(RequestHeaderView::is_save_button_visible(state.is_active_tab_dirty()));

    // 3. Saving active request marks request clean -> Save button hidden
    state.save_active_tab();
    assert!(!state.is_active_tab_dirty());
    assert!(!RequestHeaderView::is_save_button_visible(state.is_active_tab_dirty()));
    assert_eq!(
        state.status_message.as_deref(),
        Some("Saved: Untitled Request")
    );

    // 4. Typing into URL marks request dirty -> Save button visible
    state.set_active_input(Some(ActiveInputTarget::Url));
    state.type_char_into_active('a');
    assert!(state.is_active_tab_dirty());
    assert!(RequestHeaderView::is_save_button_visible(state.is_active_tab_dirty()));

    // 5. Backspacing in URL still keeps request dirty
    state.backspace_active();
    assert!(state.is_active_tab_dirty());
    assert!(RequestHeaderView::is_save_button_visible(state.is_active_tab_dirty()));

    // 6. Manual save clears dirty again
    state.save_active_tab();
    assert!(!state.is_active_tab_dirty());
    assert!(!RequestHeaderView::is_save_button_visible(state.is_active_tab_dirty()));
}

#[test]
fn test_delayed_auto_save_configuration_and_execution() {
    let mut state = AuraAppState::new();
    state.new_request();

    // Default configuration: enabled = true, delay = 1000ms
    assert!(state.auto_save_enabled);
    assert_eq!(state.auto_save_delay_ms, 1000);

    // Customize auto-save preferences
    state.set_auto_save_enabled(false);
    assert!(!state.auto_save_enabled);
    state.set_auto_save_enabled(true);
    assert!(state.auto_save_enabled);

    state.set_auto_save_delay_ms(1500);
    assert_eq!(state.auto_save_delay_ms, 1500);

    // When tab is clean, auto_save_active_tab does nothing and returns false
    assert!(!state.auto_save_active_tab());
    assert_eq!(state.status_message, None);

    // When tab is dirty, auto_save_active_tab clears dirty and returns true
    state.add_param("page", "1");
    assert!(state.is_active_tab_dirty());
    assert!(RequestHeaderView::is_save_button_visible(true));

    let saved = state.auto_save_active_tab();
    assert!(saved);
    assert!(!state.is_active_tab_dirty());
    assert!(!RequestHeaderView::is_save_button_visible(false));
    assert_eq!(
        state.status_message.as_deref(),
        Some("Auto-saved: Untitled Request")
    );
}

#[test]
fn test_auto_save_specific_tab_by_id() {
    let mut state = AuraAppState::new();
    state.new_request(); // Tab 0
    let tab0_id = state.tabs[0].id.clone();

    state.new_request(); // Tab 1
    state.active_tab_index = 1;

    // Mark tab 0 dirty while active tab is tab 1
    state.mark_tab_dirty(&tab0_id, true);
    assert!(state.is_tab_dirty(&tab0_id));
    assert!(!state.is_active_tab_dirty());

    // Auto save tab 0 specifically
    let saved = state.auto_save_tab(&tab0_id);
    assert!(saved);
    assert!(!state.is_tab_dirty(&tab0_id));
    assert_eq!(
        state.status_message.as_deref(),
        Some("Auto-saved: Untitled Request")
    );
}

#[tokio::test]
async fn test_workspace_service_save_request_file_persists_to_disk() {
    let tmp = tempdir().expect("tempdir");
    let service = WorkspaceService::new();
    let root = tmp.path();

    let mut req = aura_core::model::AuraRequest::new(
        "UserLogin",
        HttpMethod::POST,
        "https://api.example.com/v1/auth/login",
    );
    req.add_header("Content-Type", "application/json");

    let file_path = root.join("UserLogin.aura");

    // Save request file via WorkspaceService
    service
        .save_request_file(&file_path, &req)
        .await
        .expect("save request file");

    assert!(file_path.is_file());

    // Read back and verify contents
    let content = fs::read_to_string(&file_path).expect("read file");
    assert!(content.contains("https://api.example.com/v1/auth/login"));
    assert!(content.contains("Content-Type"));
    assert!(content.contains("POST"));
}

#[test]
fn test_preferences_persistence_includes_auto_save_settings() {
    let tmp = tempdir().expect("tempdir");
    let pref_file = tmp.path().join("preferences.json");

    let mut prefs = AuraPreferences::default();
    assert!(prefs.auto_save_enabled);
    assert_eq!(prefs.auto_save_delay_ms, 1000);

    prefs.auto_save_enabled = false;
    prefs.auto_save_delay_ms = 2500;
    prefs.save_to_file(&pref_file).expect("save prefs");

    let loaded = AuraPreferences::load_from_file(&pref_file);
    assert!(!loaded.auto_save_enabled);
    assert_eq!(loaded.auto_save_delay_ms, 2500);
}
