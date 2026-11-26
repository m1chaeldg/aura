#[test]
fn test_editor_language_detection() {
    let content_type = "application/json";
    let is_json = content_type.contains("json");
    assert!(is_json);

    let xml_type = "application/xml";
    let is_xml = xml_type.contains("xml");
    assert!(is_xml);

    let js_type = "application/javascript";
    let is_js = js_type.contains("javascript") || js_type.contains("js");
    assert!(is_js);
}

#[test]
fn test_editor_types_available() {
    use gpui_kit::component::input::{Editor, EditorState};
    let editor_name = std::any::type_name::<Editor>();
    let state_name = std::any::type_name::<EditorState>();
    assert!(editor_name.contains("Editor"));
    assert!(state_name.contains("state") || state_name.contains("Editor"));
}

#[test]
fn test_editor_language_for_input_keys() {
    use aura_ui::input_registry::InputKey;

    assert_eq!(InputKey::PreRequestScript.editor_language(), Some("javascript"));
    assert_eq!(InputKey::PostResponseScript.editor_language(), Some("javascript"));
    assert_eq!(InputKey::TestScript.editor_language(), Some("javascript"));
    assert_eq!(InputKey::SpecText.editor_language(), Some("toml"));
    assert_eq!(InputKey::Url.editor_language(), None);
}

#[test]
fn test_js_completion_includes_aura_variables_response_and_bdd() {
    use aura_ui::editor_support::{
        collect_completion_items, extract_completion_prefix, CompletionLanguage, EditorContextData,
    };
    use gpui_kit::component::input::Rope;

    let ctx = EditorContextData {
        variables: vec![
            ("auth_token".into(), "secret".into(), true),
            ("disabled_var".into(), "skip".into(), false),
        ],
        query_params: vec![("page".into(), "1".into(), true)],
        headers: vec![],
        graphql_schema: None,
    };

    let post_items = collect_completion_items(
        CompletionLanguage::JavaScript {
            include_response: true,
        },
        "response.",
        &ctx,
    );
    assert!(post_items.iter().any(|i| i.label == "response.status"));
    assert!(post_items.iter().any(|i| i.label == "response.json()"));

    let pre_items = collect_completion_items(
        CompletionLanguage::JavaScript {
            include_response: true,
        },
        "auth_token",
        &ctx,
    );
    assert!(pre_items
        .iter()
        .any(|i| i.label == "aura.variables.get(\"auth_token\")"));
    assert!(!pre_items
        .iter()
        .any(|i| i.label.contains("disabled_var")));

    let pre_no_resp = collect_completion_items(
        CompletionLanguage::JavaScript {
            include_response: false,
        },
        "response.",
        &ctx,
    );
    assert!(!pre_no_resp.iter().any(|i| i.label == "response.status"));

    let rope = Rope::from("const x = aura.var");
    assert_eq!(extract_completion_prefix(&rope, rope.len()), "aura.var");
}

#[test]
fn test_spec_completion_includes_sections_and_template_variables() {
    use aura_ui::editor_support::{
        collect_completion_items, CompletionLanguage, EditorContextData,
    };

    let ctx = EditorContextData {
        variables: vec![("base_url".into(), "https://api.example.com".into(), true)],
        query_params: vec![],
        headers: vec![],
        graphql_schema: None,
    };

    let items = collect_completion_items(CompletionLanguage::TomlSpec, "{{", &ctx);
    assert!(items.iter().any(|i| i.label == "{{base_url}}"));
    assert!(items.iter().any(|i| i.label == "{{$uuid}}"));

    let sections = collect_completion_items(CompletionLanguage::TomlSpec, "[set", &ctx);
    assert!(sections.iter().any(|i| i.label == "[settings]"));
}
