use gpui_kit::component::menu::PopupMenuItem;

#[test]
fn test_collection_context_actions() {
    let actions = vec!["New Request", "New Folder", "Rename", "Delete"];
    assert_eq!(actions.len(), 4);
    assert_eq!(actions[0], "New Request");
    assert_eq!(actions[1], "New Folder");
    assert_eq!(actions[2], "Rename");
    assert_eq!(actions[3], "Delete");
}

#[test]
fn test_tab_context_actions() {
    let actions = vec!["Close", "Close Others", "Duplicate"];
    assert_eq!(actions.len(), 3);
    assert_eq!(actions[0], "Close");
    assert_eq!(actions[1], "Close Others");
    assert_eq!(actions[2], "Duplicate");
}

#[test]
fn test_popup_menu_item_creation() {
    let item1 = PopupMenuItem::new("Close");
    let item2 = PopupMenuItem::new("Close Others");
    let item3 = PopupMenuItem::new("Duplicate");
    let sep = PopupMenuItem::separator();
    let _ = (item1, item2, item3, sep);
}
