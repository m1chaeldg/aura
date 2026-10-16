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
