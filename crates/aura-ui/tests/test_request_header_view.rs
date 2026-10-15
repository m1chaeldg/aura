use aura_ui::views::RequestHeaderView;

#[test]
fn test_request_header_view_button_labels() {
    assert_eq!(RequestHeaderView::button_label(false), "Send ↵");
    assert_eq!(RequestHeaderView::button_label(true), "Cancel");
}
