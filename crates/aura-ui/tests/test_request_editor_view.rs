use aura_core::model::{AuthConfig, BodyType};
use aura_ui::app_state::AuraAppState;
use aura_ui::stores::EditorTab;
use aura_ui::views::RequestEditorView;

#[test]
fn test_editor_tabs_list() {
    let tabs = RequestEditorView::available_tabs();
    assert_eq!(tabs.len(), 8);
    assert_eq!(tabs[0], EditorTab::Params);
    assert_eq!(tabs[1], EditorTab::Headers);
    assert_eq!(tabs[2], EditorTab::Auth);
    assert_eq!(tabs[3], EditorTab::Body);
    assert_eq!(tabs[4], EditorTab::Variables);
    assert_eq!(tabs[5], EditorTab::Scripts);
    assert_eq!(tabs[6], EditorTab::Tests);
    assert_eq!(tabs[7], EditorTab::Settings);
}

#[test]
fn test_headers_state_manipulation() {
    let mut state = AuraAppState::new();
    state.new_request();
    assert_eq!(state.tabs.len(), 1);

    // Add headers
    state.add_header("Content-Type", "application/json");
    state.add_header("X-API-Version", "v2");

    let headers = &state.tabs[0].request.headers;
    assert_eq!(headers.len(), 2);
    assert_eq!(headers[0].key, "Content-Type");
    assert_eq!(headers[0].value, "application/json");
    assert!(headers[0].enabled);

    // Toggle header enabled/disabled
    state.toggle_header(0);
    assert!(!state.tabs[0].request.headers[0].enabled);
    state.toggle_header(0);
    assert!(state.tabs[0].request.headers[0].enabled);

    // Update header
    state.update_header(1, "X-API-Version".into(), "v3".into());
    assert_eq!(state.tabs[0].request.headers[1].value, "v3");

    // Remove header
    state.remove_header(0);
    assert_eq!(state.tabs[0].request.headers.len(), 1);
    assert_eq!(state.tabs[0].request.headers[0].key, "X-API-Version");
}

#[test]
fn test_params_state_manipulation() {
    let mut state = AuraAppState::new();
    state.new_request();

    // Add query params
    state.add_param("page", "1");
    state.add_param("limit", "50");

    let params = &state.tabs[0].request.params;
    assert_eq!(params.len(), 2);
    assert_eq!(params[0].key, "page");
    assert_eq!(params[0].value, "1");
    assert!(params[0].enabled);

    // Toggle param
    state.toggle_param(1);
    assert!(!state.tabs[0].request.params[1].enabled);

    // Update param
    state.update_param(0, "page".into(), "2".into());
    assert_eq!(state.tabs[0].request.params[0].value, "2");

    // Remove param
    state.remove_param(0);
    assert_eq!(state.tabs[0].request.params.len(), 1);
    assert_eq!(state.tabs[0].request.params[0].key, "limit");
}

#[test]
fn test_body_types_and_formatting() {
    let mut state = AuraAppState::new();
    state.new_request();

    // Switch to JSON
    state.set_body_type("JSON");
    match &state.tabs[0].request.body {
        BodyType::Json(s) => assert_eq!(s, "{\n  \"key\": \"value\"\n}"),
        _ => panic!("Expected BodyType::Json"),
    }

    // Set unformatted JSON and format it
    state.set_body_content("{\"hello\":\"world\",\"count\":42}".to_string());
    assert!(state.format_active_json_body().is_ok());
    match &state.tabs[0].request.body {
        BodyType::Json(s) => {
            assert!(s.contains('\n'));
            assert!(s.contains("\"hello\": \"world\""));
            assert!(s.contains("\"count\": 42"));
        }
        _ => panic!("Expected formatted BodyType::Json"),
    }

    // Switch to XML
    state.set_body_type("XML");
    match &state.tabs[0].request.body {
        BodyType::Xml(s) => assert!(s.contains("<root>")),
        _ => panic!("Expected BodyType::Xml"),
    }

    // Switch to Form URL-Encoded
    state.set_body_type("FormUrlEncoded");
    state.add_form_urlencoded_param("username", "alice");
    state.add_form_urlencoded_param("grant_type", "password");

    match &state.tabs[0].request.body {
        BodyType::FormUrlEncoded(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].key, "username");
            assert_eq!(items[0].value, "alice");
            assert!(items[0].enabled);
        }
        _ => panic!("Expected BodyType::FormUrlEncoded"),
    }

    // Toggle and remove form urlencoded param
    state.toggle_form_urlencoded_param(0);
    if let BodyType::FormUrlEncoded(items) = &state.tabs[0].request.body {
        assert!(!items[0].enabled);
    }
    state.remove_form_urlencoded_param(0);
    if let BodyType::FormUrlEncoded(items) = &state.tabs[0].request.body {
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].key, "grant_type");
    }

    // Switch to Multipart
    state.set_body_type("Multipart");
    state.add_multipart_field("avatar".into(), None, Some("C:/photos/me.jpg".into()), None);
    state.add_multipart_field("description".into(), Some("My profile".into()), None, None);

    match &state.tabs[0].request.body {
        BodyType::Multipart(parts) => {
            assert_eq!(parts.len(), 2);
            assert_eq!(parts[0].name, "avatar");
            assert_eq!(parts[0].file_path.as_deref(), Some("C:/photos/me.jpg"));
            assert_eq!(parts[1].name, "description");
            assert_eq!(parts[1].value.as_deref(), Some("My profile"));
        }
        _ => panic!("Expected BodyType::Multipart"),
    }

    state.remove_multipart_field(0);
    if let BodyType::Multipart(parts) = &state.tabs[0].request.body {
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].name, "description");
    }

    // Switch to None
    state.set_body_type("None");
    assert!(matches!(state.tabs[0].request.body, BodyType::None));
}

#[test]
fn test_auth_configurations() {
    let mut state = AuraAppState::new();
    state.new_request();

    // Bearer Token
    state.set_auth_type("Bearer Token");
    state.update_bearer_token("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.e30.t-IDcSemACt8x4iTMCda8Yhe3iZaWbvV5XKSTbuAn0M".into());
    match &state.tabs[0].request.auth {
        AuthConfig::Bearer { token } => assert!(token.starts_with("eyJ")),
        _ => panic!("Expected AuthConfig::Bearer"),
    }

    // Basic Auth
    state.set_auth_type("Basic Auth");
    state.update_basic_auth("developer".into(), "secret123".into());
    match &state.tabs[0].request.auth {
        AuthConfig::Basic { username, password } => {
            assert_eq!(username, "developer");
            assert_eq!(password, "secret123");
        }
        _ => panic!("Expected AuthConfig::Basic"),
    }

    // API Key
    state.set_auth_type("API Key");
    state.update_api_key("X-API-KEY".into(), "xyz-789".into(), true);
    match &state.tabs[0].request.auth {
        AuthConfig::ApiKey { key, value, in_header } => {
            assert_eq!(key, "X-API-KEY");
            assert_eq!(value, "xyz-789");
            assert!(*in_header);
        }
        _ => panic!("Expected AuthConfig::ApiKey in header"),
    }

    // Toggle API Key to query param
    state.update_api_key("api_key".into(), "xyz-789".into(), false);
    match &state.tabs[0].request.auth {
        AuthConfig::ApiKey { in_header, .. } => assert!(!*in_header),
        _ => panic!("Expected AuthConfig::ApiKey in query"),
    }

    // AWS SigV4
    state.set_auth_type("AWS SigV4");
    state.update_aws_sigv4("AKIAEXAMPLE".into(), "secretkey".into(), "us-east-1".into(), "s3".into(), None);
    match &state.tabs[0].request.auth {
        AuthConfig::AwsSigV4 { access_key, region, service, .. } => {
            assert_eq!(access_key, "AKIAEXAMPLE");
            assert_eq!(region, "us-east-1");
            assert_eq!(service, "s3");
        }
        _ => panic!("Expected AuthConfig::AwsSigV4"),
    }

    // OAuth 2.0
    state.set_auth_type("OAuth 2.0");
    state.update_oauth2("authorization_code".into(), "tok_abc".into(), Some("https://auth.example.com/token".into()), None, None);
    match &state.tabs[0].request.auth {
        AuthConfig::OAuth2 { grant_type, access_token, token_url, .. } => {
            assert_eq!(grant_type, "authorization_code");
            assert_eq!(access_token, "tok_abc");
            assert_eq!(token_url.as_deref(), Some("https://auth.example.com/token"));
        }
        _ => panic!("Expected AuthConfig::OAuth2"),
    }

    // None & Inherit
    state.set_auth_type("None");
    assert!(matches!(state.tabs[0].request.auth, AuthConfig::None));
    state.set_auth_type("Inherit");
    assert!(matches!(state.tabs[0].request.auth, AuthConfig::Inherit));
}

#[test]
fn test_dialog_workflow() {
    let mut state = AuraAppState::new();
    state.new_request();

    // 1. Open Add Header Dialog and type input
    state.open_add_header_dialog();
    assert!(state.editor_dialog.is_some());

    // Type "X-Custom" into key
    for c in "X-Custom".chars() {
        state.dialog_append_char(c);
    }
    // Switch focus to value field
    state.dialog_tab_next_field();
    // Type "CustomVal" into value
    for c in "CustomVal".chars() {
        state.dialog_append_char(c);
    }

    // Confirm dialog
    state.confirm_editor_dialog();
    assert!(state.editor_dialog.is_none());

    // Verify header was added to active tab request
    let headers = &state.tabs[0].request.headers;
    assert_eq!(headers.len(), 1);
    assert_eq!(headers[0].key, "X-Custom");
    assert_eq!(headers[0].value, "CustomVal");

    // 2. Open Edit Header Dialog, backspace, and confirm
    state.open_edit_header_dialog(0);
    assert!(state.editor_dialog.is_some());
    // Tab to value
    state.dialog_tab_next_field();
    state.dialog_backspace(); // deletes 'l'
    state.dialog_append_char('l');
    state.dialog_append_char('u');
    state.dialog_append_char('e'); // now "CustomValue"
    state.confirm_editor_dialog();

    assert_eq!(state.tabs[0].request.headers[0].value, "CustomValue");

    // 3. Open Dialog and Cancel
    state.open_add_param_dialog();
    state.dialog_append_char('a');
    state.cancel_editor_dialog();
    assert!(state.editor_dialog.is_none());
    assert_eq!(state.tabs[0].request.params.len(), 0);
}
