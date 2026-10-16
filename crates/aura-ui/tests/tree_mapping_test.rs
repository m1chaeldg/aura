use gpui_kit::component::sidebar::{
    Sidebar, SidebarGroup, SidebarMenu, SidebarMenuItem,
};

#[test]
fn test_sidebar_menu_construction() {
    let item = SidebarMenuItem::new("GET /users");
    let menu = SidebarMenu::new().child(item);
    let group = SidebarGroup::new("COLLECTIONS").child(menu);
    let sidebar = Sidebar::new("workspace-sidebar").child(group);
    let _ = sidebar;
}
