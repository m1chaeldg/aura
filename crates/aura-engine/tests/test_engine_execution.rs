use aura_core::model::{AuraRequest, HttpMethod, RequestSettings, ScriptConfig};
use aura_core::templating::TemplateContext;
use aura_engine::execute_request;

#[tokio::test]
async fn test_execute_request_with_scripts_and_console_logs() {
    let mut req = AuraRequest::new(
        "Mock Request",
        HttpMethod::GET,
        "https://postman-echo.com/get",
    );
    req.scripts = ScriptConfig {
        pre_request: "console.log('Pre-request script executed'); aura.variables.set('test_var', '123');".to_string(),
        post_response: "console.log('Post-response status is', response.status);".to_string(),
        tests: "describe('Status Check', () => { it('is 200', () => { expect(response.status).toBe(200); }); });".to_string(),
        assertions: Vec::new(),
    };

    let context = TemplateContext::default();
    let resp = execute_request(&req, &context)
        .await
        .expect("execute request");

    assert_eq!(resp.status, 200);
    assert!(!resp.console_logs.is_empty());
    assert!(resp
        .console_logs
        .iter()
        .any(|l| l.message.contains("Pre-request script executed")));
    assert!(resp
        .console_logs
        .iter()
        .any(|l| l.message.contains("Post-response status is 200")));
    assert_eq!(resp.assertions.len(), 1);
    assert!(resp.assertions[0].passed);
}

#[tokio::test]
async fn test_execute_request_network_error_handled_gracefully() {
    let req = AuraRequest::new(
        "Unreachable Host",
        HttpMethod::GET,
        "http://127.0.0.1:59999/unreachable",
    );
    let mut settings = RequestSettings::default();
    settings.timeout_ms = 1000;
    let mut req = req;
    req.settings = settings;

    let context = TemplateContext::default();
    let res = execute_request(&req, &context).await;
    assert!(res.is_err());
    let err_msg = res.unwrap_err().to_string();
    assert!(err_msg.contains("Network error"), "unexpected: {err_msg}");
    assert!(err_msg.contains("Possible cause:"), "unexpected: {err_msg}");
    assert!(err_msg.contains("->"), "expected cause chain in: {err_msg}");
}

#[tokio::test]
async fn test_execute_request_local_json_assertions_and_extractors() {
    use aura_core::assertions::{AssertOperator, DeclarativeAssertion};
    use aura_core::extractors::{NativeExtractor, NativeExtractorType};
    use aura_core::model::BodyType;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            let mut buf = vec![0u8; 2048];
            let _ = stream.read(&mut buf).await;
            let body = r#"{"id":7,"token":"abc-123"}"#;
            let resp = format!(
                "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(resp.as_bytes()).await;
            let _ = stream.shutdown().await;
        }
    });

    let mut req = AuraRequest::new("Create", HttpMethod::POST, format!("http://{addr}/items"));
    req.add_param("dry_run", "false");
    req.add_header("X-Client", "aura-test");
    req.set_body(BodyType::Json(r#"{"name":"widget"}"#.to_string()));
    req.scripts.assertions.push(DeclarativeAssertion::new(
        "res.status",
        AssertOperator::Eq,
        "201",
    ));
    req.extractors.push(NativeExtractor::new(
        "auth_tok",
        NativeExtractorType::JsonPath,
        "token",
    ));

    let resp = execute_request(&req, &TemplateContext::default())
        .await
        .expect("local execute_request");
    assert_eq!(resp.status, 201);
    assert_eq!(resp.assertions.len(), 1);
    assert!(resp.assertions[0].passed);
    assert_eq!(
        resp.exported_variables.get("auth_tok").map(String::as_str),
        Some("abc-123")
    );
    server.abort();
}

#[tokio::test]
async fn test_execute_request_pre_script_error_returns_status_zero() {
    let mut req = AuraRequest::new("Bad Script", HttpMethod::GET, "http://127.0.0.1:1/unused");
    req.scripts.pre_request = "throw new Error('boom');".to_string();

    let resp = execute_request(&req, &TemplateContext::default())
        .await
        .expect("pre-script error should return AuraResponse");
    assert_eq!(resp.status, 0);
    assert_eq!(resp.status_text, "Pre-request Script Error");
    assert_eq!(resp.script_errors.len(), 1);
    assert!(resp.script_errors[0].message.contains("boom"));
}
