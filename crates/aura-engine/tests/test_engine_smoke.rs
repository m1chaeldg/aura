use aura_core::model::{AuraRequest, HttpMethod};
use aura_core::templating::TemplateContext;
use aura_engine::execute_request;

#[tokio::test]
async fn test_engine_smoke() {
    let req = AuraRequest::new("Smoke", HttpMethod::GET, "https://postman-echo.com/get");
    let ctx = TemplateContext::default();
    let result = execute_request(&req, &ctx).await;
    assert!(result.is_ok());
}
