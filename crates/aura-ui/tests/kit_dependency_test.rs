#[test]
fn test_gpui_kit_dependency_present() {
    assert!(std::any::type_name::<gpui_kit::component::tab::Tab>().contains("Tab"));
}
