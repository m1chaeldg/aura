//! Integration tests against live Postman Echo endpoints.
//! Mandatory per AGENTS.md directives.

use aura::engine::execute_request;
use aura::model::{AuraRequest, BodyType, HeaderItem, HttpMethod, ParamItem};
use aura::templating::TemplateContext;

#[tokio::test]
async fn test_live_get_request() {
    let mut req = AuraRequest::new(
        "Live GET Echo",
        HttpMethod::GET,
        "https://postman-echo.com/get",
    );
    req.params.push(ParamItem {
        key: "foo1".to_string(),
        value: "bar1".to_string(),
        enabled: true,
        description: None,
    });
    req.params.push(ParamItem {
        key: "foo2".to_string(),
        value: "bar2".to_string(),
        enabled: true,
        description: None,
    });

    req.scripts.tests = r#"
        aura.test("status code is 200", () => {
            expect(response.status).toBe(200);
        });
        aura.test("response args contains foo1 and foo2", () => {
            const data = response.json();
            expect(data.args.foo1).toBe("bar1");
            expect(data.args.foo2).toBe("bar2");
        });
    "#
    .to_string();

    let context = TemplateContext::new();
    let resp = execute_request(&req, &context)
        .await
        .expect("Failed to execute live GET");

    assert_eq!(resp.status, 200);
    assert!(resp.duration_ms > 0);
    assert!(resp.size_bytes > 0);
    assert!(resp.body.contains("\"foo1\": \"bar1\"") || resp.body.contains("\"foo1\":\"bar1\""));
    assert!(resp.body.contains("\"foo2\": \"bar2\"") || resp.body.contains("\"foo2\":\"bar2\""));

    assert_eq!(resp.assertions.len(), 2);
    assert!(resp.assertions[0].passed);
    assert!(resp.assertions[1].passed);
}

#[tokio::test]
async fn test_live_post_request() {
    let mut req = AuraRequest::new(
        "Live POST Echo",
        HttpMethod::POST,
        "https://postman-echo.com/post",
    );
    req.headers.push(HeaderItem {
        key: "X-Aura-Client".to_string(),
        value: "v0.1.0".to_string(),
        enabled: true,
        description: None,
    });
    req.body = BodyType::Json(r#"{"client": "aura", "mode": "integration"}"#.to_string());

    req.scripts.tests = r#"
        aura.test("status code is 200", () => {
            expect(response.status).toBe(200);
        });
        aura.test("echoes json body", () => {
            const data = response.json();
            expect(data.data.client).toBe("aura");
        });
    "#
    .to_string();

    let context = TemplateContext::new();
    let resp = execute_request(&req, &context)
        .await
        .expect("Failed to execute live POST");

    assert_eq!(resp.status, 200);
    assert!(resp.body.contains("aura"));
    assert_eq!(resp.assertions.len(), 2);
    assert!(resp.assertions[0].passed);
    assert!(resp.assertions[1].passed);
}

#[tokio::test]
async fn test_live_put_request() {
    let mut req = AuraRequest::new(
        "Live PUT Echo",
        HttpMethod::PUT,
        "https://postman-echo.com/put",
    );
    req.body = BodyType::Json(r#"{"action": "update_resource"}"#.to_string());

    let context = TemplateContext::new();
    let resp = execute_request(&req, &context)
        .await
        .expect("Failed to execute live PUT");

    assert_eq!(resp.status, 200);
    assert!(resp.body.contains("update_resource"));
}

#[tokio::test]
async fn test_live_patch_request() {
    let mut req = AuraRequest::new(
        "Live PATCH Echo",
        HttpMethod::PATCH,
        "https://postman-echo.com/patch",
    );
    req.body = BodyType::Json(r#"{"patch_field": "patched_value"}"#.to_string());

    let context = TemplateContext::new();
    let resp = execute_request(&req, &context)
        .await
        .expect("Failed to execute live PATCH");

    assert_eq!(resp.status, 200);
    assert!(resp.body.contains("patched_value"));
}

#[tokio::test]
async fn test_live_delete_request() {
    let req = AuraRequest::new(
        "Live DELETE Echo",
        HttpMethod::DELETE,
        "https://postman-echo.com/delete",
    );

    let context = TemplateContext::new();
    let resp = execute_request(&req, &context)
        .await
        .expect("Failed to execute live DELETE");

    assert_eq!(resp.status, 200);
    assert!(resp.body.contains("https://postman-echo.com/delete"));
}
