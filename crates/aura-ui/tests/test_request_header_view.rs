use aura_ui::views::RequestHeaderView;

#[test]
fn test_request_header_view_button_labels() {
    assert_eq!(RequestHeaderView::button_label(false), "Send ↵");
    assert_eq!(RequestHeaderView::button_label(true), "Cancel");
}

#[test]
fn test_request_header_view_save_button_visibility() {
    // When the request is clean / not dirty, save button is hidden
    assert!(!RequestHeaderView::is_save_button_visible(false));

    // When the request is dirty / changed, save button is shown
    assert!(RequestHeaderView::is_save_button_visible(true));
}
