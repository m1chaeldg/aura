use aura_core::model::{AuraRequest, HttpMethod};
use aura_ui::app_state::{ActiveInputTarget, AuraAppState};
use aura_ui::stores::layout_store::EditorTab;
use aura_ui::views::request_editor_view::RequestEditorView;

#[test]
fn test_spec_tab_in_editor_tabs_and_title() {
    let tabs = RequestEditorView::available_tabs();
    assert_eq!(tabs.last(), Some(&EditorTab::Spec));
    assert_eq!(RequestEditorView::tab_title(EditorTab::Spec), "Spec");
}

#[test]
fn test_workspace_tab_spec_buffer_sync_valid() {
    let mut state = AuraAppState::new();
    let initial_req = AuraRequest::new("Init Req", HttpMethod::GET, "https://postman-echo.com/get");
    state.open_request(initial_req);

    // Initial spec buffer should be populated or lazily generated
    let tab = state.active_tab_mut().expect("active tab");
    tab.ensure_spec_buffer();
    assert!(tab.spec_buffer.as_ref().unwrap().contains("version = \"1.0\""));
    assert!(tab.spec_buffer.as_ref().unwrap().contains("https://postman-echo.com/get"));
    assert!(tab.spec_error.is_none());

    // Update spec buffer with valid modified spec
    let new_spec = "version = \"1.0\"\nname = \"Updated Spec\"\nmethod = \"POST\"\nurl = \"https://postman-echo.com/post\"\n\n[headers]\nX-Custom = \"123\"\n";
    tab.update_spec_buffer(new_spec.to_string());

    assert!(tab.is_dirty);
    assert_eq!(tab.request.name, "Updated Spec");
    assert_eq!(tab.request.method, HttpMethod::POST);
    assert_eq!(tab.request.url, "https://postman-echo.com/post");
    assert_eq!(tab.request.headers.len(), 1);
    assert_eq!(tab.request.headers[0].key, "X-Custom");
    assert_eq!(tab.request.headers[0].value, "123");
    assert!(tab.spec_error.is_none());
}

#[test]
fn test_workspace_tab_spec_buffer_resilience_to_syntax_errors() {
    let mut state = AuraAppState::new();
    let initial_req = AuraRequest::new("Valid Req", HttpMethod::GET, "https://postman-echo.com/get");
    state.open_request(initial_req);

    let tab = state.active_tab_mut().expect("active tab");
    tab.ensure_spec_buffer();

    // Type invalid / broken syntax
    let broken_spec = "version = \"1.0\"\nthis is not valid toml or aura spec ::: !!!";
    tab.update_spec_buffer(broken_spec.to_string());

    assert!(tab.is_dirty);
    assert!(tab.spec_error.is_some(), "Should report syntax error in spec_error");
    // Crucial: visual model must NOT be corrupted!
    assert_eq!(tab.request.name, "Valid Req");
    assert_eq!(tab.request.url, "https://postman-echo.com/get");

    // Reverting restores spec_buffer to valid model and clears error
    tab.revert_spec_buffer();
    assert!(tab.spec_error.is_none());
    assert!(tab.spec_buffer.as_ref().unwrap().contains("Valid Req"));
}

#[test]
fn test_visual_edits_regenerate_spec_tab_on_switch() {
    let mut state = AuraAppState::new();
    let req = AuraRequest::new("Visual Test", HttpMethod::GET, "https://postman-echo.com/get");
    state.open_request(req);

    // Modify visual URL
    state.set_active_url("https://postman-echo.com/put".to_string());
    assert_eq!(state.active_request().unwrap().url, "https://postman-echo.com/put");

    // Switching to Spec tab regenerates spec buffer
    state.set_active_editor_tab(EditorTab::Spec);

    let tab = state.active_tab().unwrap();
    assert!(tab.spec_buffer.as_ref().unwrap().contains("https://postman-echo.com/put"));
}

#[test]
fn test_typing_in_spec_input_target_updates_buffer_and_request() {
    let mut state = AuraAppState::new();
    let req = AuraRequest::new("Type Test", HttpMethod::GET, "https://postman-echo.com/get");
    state.open_request(req);

    state.set_active_editor_tab(EditorTab::Spec);
    state.set_active_input(Some(ActiveInputTarget::SpecText));

    // Append newline to spec
    state.type_char_into_active('\n');
    assert!(state.is_active_tab_dirty());
    let tab = state.active_tab().unwrap();
    assert!(tab.spec_buffer.is_some());
}

#[tokio::test]
async fn test_spec_tab_edits_sync_with_disk_file() {
    let tmp_path = std::env::temp_dir().join(format!("aura_test_spec_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&tmp_path).expect("create temp dir");
    let service = aura_ui::services::workspace_service::WorkspaceService::new();
    let file_path = tmp_path.join("UserAccount.aura");

    let mut state = AuraAppState::new();
    let initial_req = AuraRequest::new("UserAccount", HttpMethod::GET, "https://postman-echo.com/get");
    state.open_request(initial_req);
    state.tabs[0].file_path = Some(file_path.clone());

    // Switch to Spec tab and update spec buffer
    state.set_active_editor_tab(EditorTab::Spec);
    let tab = state.active_tab_mut().expect("active tab");
    tab.update_spec_buffer("version = \"1.0\"\nname = \"UserAccount\"\nmethod = \"POST\"\nurl = \"https://postman-echo.com/post\"\n\n[headers]\nContent-Type = \"application/json\"\n".to_string());

    assert!(state.is_active_tab_dirty());
    assert_eq!(state.active_request().unwrap().method, HttpMethod::POST);

    // Persist to disk via save_request_file
    let req = state.active_request().unwrap().clone();
    service.save_request_file(&file_path, &req).await.expect("save request file");
    state.save_active_tab();

    assert!(!state.is_active_tab_dirty());

    // Read back file from disk and parse
    let disk_content = std::fs::read_to_string(&file_path).expect("read file");
    assert!(disk_content.contains("method = \"POST\""));
    assert!(disk_content.contains("https://postman-echo.com/post"));
    assert!(disk_content.contains("Content-Type = \"application/json\""));

    let parsed = aura_parser::parse_request(&disk_content).expect("parse saved spec");
    assert_eq!(parsed.method, HttpMethod::POST);
    assert_eq!(parsed.url, "https://postman-echo.com/post");
    assert_eq!(parsed.headers.len(), 1);
}
