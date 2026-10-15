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
