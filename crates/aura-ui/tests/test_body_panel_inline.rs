use aura_ui::app_state::{ActiveInputTarget, AuraAppState};

#[test]
fn test_body_panel_direct_editing_and_format() {
    let mut state = AuraAppState::new();
    state.new_request();

    state.set_body_type("JSON");
    state.set_active_input(Some(ActiveInputTarget::BodyText));

    // Clear body and type fresh JSON
    state.set_body_content(String::new());
    for c in "{\"foo\":\"bar\"}".chars() {
        state.type_char_into_active(c);
    }
    assert_eq!(
        state.tabs[0].request.body.content_str(),
        "{\"foo\":\"bar\"}"
    );

    // Format JSON
    assert!(state.format_active_json_body().is_ok());
    let formatted = state.tabs[0].request.body.content_str().to_string();
    assert!(formatted.contains('\n'));
    assert!(formatted.contains("\"foo\": \"bar\""));

    // Backspace test
    state.backspace_active();
    assert_ne!(state.tabs[0].request.body.content_str(), &formatted);
}
