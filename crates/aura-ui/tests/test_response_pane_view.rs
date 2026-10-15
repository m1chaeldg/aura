use aura_core::model::TestAssertion;
use aura_ui::stores::ResponseBodyMode;
use aura_ui::views::ResponsePaneView;

#[test]
fn test_pretty_format_json_response() {
    let raw = "{\"name\":\"Aura\",\"fast\":true}";
    let pretty = ResponsePaneView::format_body(raw, ResponseBodyMode::Pretty);
    assert!(pretty.contains('\n'));
    assert!(pretty.contains("  \"name\": \"Aura\""));

    let non_json = "plain text response";
    let fallback = ResponsePaneView::format_body(non_json, ResponseBodyMode::Pretty);
    assert_eq!(fallback, "plain text response");

    let raw_mode = ResponsePaneView::format_body(raw, ResponseBodyMode::Raw);
    assert_eq!(raw_mode, raw);
}

#[test]
fn test_assertion_stats_helper() {
    let assertions = vec![
        TestAssertion {
            name: "t1".into(),
            passed: true,
            message: None,
            duration_ms: 1,
        },
        TestAssertion {
            name: "t2".into(),
            passed: false,
            message: Some("expected 200".into()),
            duration_ms: 2,
        },
    ];
    let (passed, total) = ResponsePaneView::assertion_counts(&assertions);
    assert_eq!(passed, 1);
    assert_eq!(total, 2);
}
