use aura_core::model::{AuraRequest, HttpMethod};
use aura_core::templating::TemplateContext;

#[test]
fn test_core_smoke() {
    let req = AuraRequest::new("Smoke", HttpMethod::GET, "https://httpbin.org/get");
    assert_eq!(req.name, "Smoke");
    let mut ctx = TemplateContext::new();
    ctx.environment_vars
        .insert("host".to_string(), "httpbin.org".to_string());
    assert_eq!(
        ctx.render("https://{{host}}/get"),
        "https://httpbin.org/get"
    );
}
