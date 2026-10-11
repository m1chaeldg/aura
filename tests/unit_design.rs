use aura::design::{colors, rounded, spacing, typography};
use aura::model::HttpMethod;

#[test]
fn test_design_tokens() {
    assert_eq!(colors::PRIMARY, "#0066cc");
    assert_eq!(colors::CANVAS, "#ffffff");
    assert_eq!(colors::CANVAS_PARCHMENT, "#f5f5f7");
    assert_eq!(colors::SURFACE_BLACK, "#000000");

    assert_eq!(spacing::XS, 8.0);
    assert_eq!(spacing::SECTION, 80.0);
    assert_eq!(rounded::PILL, 9999.0);

    assert_eq!(typography::BODY.font_size, 17.0);
    assert_eq!(typography::HERO_DISPLAY.font_size, 56.0);
}

#[test]
fn test_http_method_badge_colors() {
    assert_eq!(HttpMethod::GET.badge_color(), colors::METHOD_GET);
    assert_eq!(HttpMethod::POST.badge_color(), colors::METHOD_POST);
    assert_eq!(HttpMethod::PUT.badge_color(), colors::METHOD_PUT);
    assert_eq!(HttpMethod::PATCH.badge_color(), colors::METHOD_PATCH);
    assert_eq!(HttpMethod::DELETE.badge_color(), colors::METHOD_DELETE);
}
