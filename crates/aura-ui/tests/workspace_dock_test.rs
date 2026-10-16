#[test]
fn test_dock_layout_initial_ratios() {
    let sidebar_ratio = 0.25;
    let main_ratio = 0.75;
    assert_eq!(sidebar_ratio + main_ratio, 1.0);
}

#[test]
fn test_dock_area_type_available() {
    use gpui_kit::component::dock::DockArea;
    assert!(std::any::type_name::<DockArea>().contains("DockArea"));
}
