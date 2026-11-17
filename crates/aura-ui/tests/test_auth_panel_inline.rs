use aura_core::model::AuthConfig;
use aura_ui::app_state::{ActiveInputTarget, AuraAppState};

#[test]
fn test_auth_preset_selection_and_inline_editing() {
    let mut state = AuraAppState::new();
    state.new_request();

    // 1. Select Bearer Token - inputs are ready immediately
    state.set_auth_type("Bearer Token");
    assert!(matches!(
        state.tabs[0].request.auth,
        AuthConfig::Bearer { .. }
    ));

    state.set_active_input(Some(ActiveInputTarget::AuthBearerToken));
    for c in "token123".chars() {
        state.type_char_into_active(c);
    }
    match &state.tabs[0].request.auth {
        AuthConfig::Bearer { token } => assert_eq!(token, "token123"),
        _ => panic!("Expected Bearer token"),
    }

    // 2. Select Basic Auth - Username & Password ready immediately
    state.set_auth_type("Basic Auth");
    state.set_active_input(Some(ActiveInputTarget::AuthBasicUsername));
    for c in "admin".chars() {
        state.type_char_into_active(c);
    }
    state.tab_next_input(false); // Tab moves to Password
    assert_eq!(
        state.active_input_target,
        Some(ActiveInputTarget::AuthBasicPassword)
    );
    for c in "pass456".chars() {
        state.type_char_into_active(c);
    }
    match &state.tabs[0].request.auth {
        AuthConfig::Basic { username, password } => {
            assert_eq!(username, "admin");
            assert_eq!(password, "pass456");
        }
        _ => panic!("Expected Basic auth"),
    }

    // 3. Select API Key - Key Name & Key Value ready immediately
    state.set_auth_type("API Key");
    state.update_api_key(String::new(), String::new(), true);
    state.set_active_input(Some(ActiveInputTarget::AuthApiKeyName));
    for c in "X-App-Key".chars() {
        state.type_char_into_active(c);
    }
    state.tab_next_input(false); // Tab moves to Value
    assert_eq!(
        state.active_input_target,
        Some(ActiveInputTarget::AuthApiKeyValue)
    );
    for c in "secret-xyz".chars() {
        state.type_char_into_active(c);
    }
    match &state.tabs[0].request.auth {
        AuthConfig::ApiKey {
            key,
            value,
            in_header,
        } => {
            assert_eq!(key, "X-App-Key");
            assert_eq!(value, "secret-xyz");
            assert!(in_header);
        }
        _ => panic!("Expected API Key auth"),
    }
}
