use aura_core::model::{AuraRequest, HttpMethod, RequestSettings, ScriptConfig};
use aura_core::templating::TemplateContext;
use aura_engine::execute_request;

#[tokio::test]
async fn test_execute_request_with_scripts_and_console_logs() {
    let mut req = AuraRequest::new("Mock Request", HttpMethod::GET, "https://postman-echo.com/get");
    req.scripts = ScriptConfig {
        pre_request: "console.log('Pre-request script executed'); aura.variables.set('test_var', '123');".to_string(),
        post_response: "console.log('Post-response status is', response.status);".to_string(),
        tests: "describe('Status Check', () => { it('is 200', () => { expect(response.status).toBe(200); }); });".to_string(),
    };

    let context = TemplateContext::default();
    let resp = execute_request(&req, &context).await.expect("execute request");

    assert_eq!(resp.status, 200);
    assert!(!resp.console_logs.is_empty());
    assert!(resp.console_logs.iter().any(|l| l.message.contains("Pre-request script executed")));
    assert!(resp.console_logs.iter().any(|l| l.message.contains("Post-response status is 200")));
    assert_eq!(resp.assertions.len(), 1);
    assert!(resp.assertions[0].passed);
}

#[tokio::test]
async fn test_execute_request_network_error_handled_gracefully() {
    let req = AuraRequest::new("Unreachable Host", HttpMethod::GET, "http://127.0.0.1:59999/unreachable");
    let mut settings = RequestSettings::default();
    settings.timeout_ms = 1000;
    let mut req = req;
    req.settings = settings;

    let context = TemplateContext::default();
    let res = execute_request(&req, &context).await;
    assert!(res.is_err());
    let err_msg = res.unwrap_err().to_string();
    assert!(err_msg.contains("Network error"));
}
