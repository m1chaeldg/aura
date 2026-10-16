use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::Selectable;
use aura_core::model::{AuraRequest, HttpMethod};
use aura_ui::WorkspaceTab;

#[test]
fn test_tab_bar_tab_state_mapping() {
    let mut tab_data = WorkspaceTab::new(AuraRequest::new("Get User", HttpMethod::GET, "https://api.example.com/users"));
    tab_data.mark_dirty(true);

    let prefix_badge = tab_data.method.as_str();
    let dirty_indicator = if tab_data.is_dirty { "●" } else { "" };

    let tab = Tab::new()
        .label(tab_data.title.clone())
        .prefix(format!("{} ", prefix_badge))
        .suffix(format!(" {}", dirty_indicator))
        .selected(true);

    assert!(tab.is_selected());

    let bar = TabBar::new("workspace-tabs").child(tab);
    let _ = bar;
}
