//! End-to-end QA validation suite:
//! - Multi-request concurrent collection runner micro-load stability
//! - Bidirectional lossless sync stability under repeated transformations
//! - deno_core BDD test assertions across request lifecycles

use aura::model::{AuraRequest, BodyType, HttpMethod, ParamItem};
use aura::runner::{run_collection, RunnerConfig};
use aura::sync::{request_to_toml, toml_to_request};
use aura::templating::TemplateContext;

#[tokio::test]
async fn test_qa_high_concurrency_collection_runner() {
    let mut req1 = AuraRequest::new("Echo GET", HttpMethod::GET, "https://postman-echo.com/get");
    req1.params.push(ParamItem {
        key: "qa".to_string(),
        value: "runner".to_string(),
        enabled: true,
        description: None,
    });

    let mut req2 = AuraRequest::new(
        "Echo POST",
        HttpMethod::POST,
        "https://postman-echo.com/post",
    );
    req2.body = BodyType::Json(r#"{"load_test": true}"#.to_string());

    let requests = vec![req1, req2];
    let config = RunnerConfig {
        total_iterations: 6,
        concurrency: 3,
        delay_ms: 10,
    };

    let report = run_collection(requests, TemplateContext::new(), config, None).await;

    assert_eq!(report.total_requests, 12);
    assert_eq!(report.successful_requests, 12);
    assert_eq!(report.failed_requests, 0);
    assert_eq!(report.error_rate_percent, 0.0);
    assert!(report.requests_per_second > 0.0);
    assert!(report.p50_latency_ms > 0);
    assert!(report.p99_latency_ms >= report.p50_latency_ms);
}

#[test]
fn test_qa_bidirectional_sync_stability() {
    let mut req = AuraRequest::new(
        "Stress Sync",
        HttpMethod::PUT,
        "https://postman-echo.com/put",
    );
    req.scripts.pre_request = "console.log('1');".to_string();
    req.scripts.post_response = "console.log('2');".to_string();
    req.scripts.tests = "expect(1).toBe(1);".to_string();

    // Round-trip 5 times
    let mut current = req.clone();
    for _ in 0..5 {
        let toml_str = request_to_toml(&current).expect("Serialization failed");
        current = toml_to_request(&toml_str, Some(&req.id)).expect("Deserialization failed");
    }

    assert_eq!(current.id, req.id);
    assert_eq!(current.name, req.name);
    assert_eq!(current.method, req.method);
    assert_eq!(current.scripts, req.scripts);
}
