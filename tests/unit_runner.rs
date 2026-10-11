use aura::model::{AuraRequest, HttpMethod};
use aura::runner::{run_collection, RunnerConfig};
use aura::templating::TemplateContext;

#[tokio::test]
async fn test_collection_runner_metrics() {
    let mut requests = Vec::new();
    let req = AuraRequest::new(
        "Dummy Request",
        HttpMethod::GET,
        "https://postman-echo.com/get",
    );
    requests.push(req);

    let config = RunnerConfig {
        total_iterations: 3,
        concurrency: 1,
        delay_ms: 10,
    };

    let report = run_collection(requests, TemplateContext::new(), config, None).await;

    assert_eq!(report.total_requests, 3);
    assert_eq!(report.successful_requests, 3);
    assert_eq!(report.failed_requests, 0);
    assert_eq!(report.error_rate_percent, 0.0);
    assert!(report.min_latency_ms > 0);
    assert!(report.p50_latency_ms > 0);
    assert!(report.max_latency_ms >= report.min_latency_ms);
}
