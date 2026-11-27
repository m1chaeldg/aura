use aura::reporting::{
    compute_aggregate_report, compute_time_series, export_samples_json, export_samples_jtl_csv,
    generate_html_report,
};
use aura::runner::{RunnerExecutionRecord, RunnerReport};
use std::fs;
use std::process::Command;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn sample_records() -> Vec<RunnerExecutionRecord> {
    vec![
        RunnerExecutionRecord {
            iteration: 1,
            request_name: "Login, \"Auth\"".into(),
            method: "POST".into(),
            status: 200,
            latency_ms: 20,
            passed: true,
            error: None,
            active_vus: 2,
            elapsed_ms: 150,
            size_bytes: 512,
        },
        RunnerExecutionRecord {
            iteration: 1,
            request_name: "Get Items".into(),
            method: "GET".into(),
            status: 200,
            latency_ms: 40,
            passed: true,
            error: None,
            active_vus: 2,
            elapsed_ms: 350,
            size_bytes: 2048,
        },
        RunnerExecutionRecord {
            iteration: 2,
            request_name: "Get Items".into(),
            method: "GET".into(),
            status: 500,
            latency_ms: 120,
            passed: false,
            error: Some("500 Internal Error".into()),
            active_vus: 4,
            elapsed_ms: 1200,
            size_bytes: 1024,
        },
        RunnerExecutionRecord {
            iteration: 2,
            request_name: "Get Items".into(),
            method: "GET".into(),
            status: 200,
            latency_ms: 80,
            passed: true,
            error: None,
            active_vus: 4,
            elapsed_ms: 1450,
            size_bytes: 2048,
        },
    ]
}

#[test]
fn test_time_series_aggregate_and_exports() {
    let recs = sample_records();

    let series = compute_time_series(&recs, Some(1000));
    assert_eq!(series.len(), 2);
    assert_eq!(series[0].sample_count, 2);
    assert_eq!(series[0].active_vus, 2);
    assert_eq!(series[0].error_rate_percent, 0.0);
    assert_eq!(series[1].sample_count, 2);
    assert_eq!(series[1].active_vus, 4);
    assert_eq!(series[1].error_rate_percent, 50.0);
    assert!(series[1].p95_ms >= series[1].p50_ms);

    let aggs = compute_aggregate_report(&recs, 2000);
    assert_eq!(aggs.len(), 2);
    assert_eq!(aggs[0].request_name, "Login, \"Auth\"");
    assert_eq!(aggs[0].samples, 1);
    assert_eq!(aggs[1].request_name, "Get Items");
    assert_eq!(aggs[1].samples, 3);
    assert_eq!(aggs[1].min_ms, 40);
    assert_eq!(aggs[1].max_ms, 120);
    assert_eq!(aggs[1].median_ms, 80);
    assert!((aggs[1].avg_ms - 80.0).abs() < 0.01);
    assert!((aggs[1].rps - 1.5).abs() < 0.01);
    assert!(aggs[1].kb_per_sec > 2.4);

    let csv = export_samples_jtl_csv(&recs);
    assert!(csv.starts_with("elapsed_ms,iteration,request_name,method,status,latency_ms,size_bytes,active_vus,passed,error"));
    assert!(csv.contains("\"Login, \"\"Auth\"\"\""));
    assert!(csv.contains("500 Internal Error"));

    let report = RunnerReport {
        total_requests: 4,
        successful_requests: 3,
        failed_requests: 1,
        error_rate_percent: 25.0,
        requests_per_second: 2.0,
        min_latency_ms: 20,
        mean_latency_ms: 65.0,
        p50_latency_ms: 80,
        p90_latency_ms: 120,
        p95_latency_ms: 120,
        p99_latency_ms: 120,
        max_latency_ms: 120,
        total_duration_ms: 2000,
        peak_vus: 4,
        target_rps: None,
        sla_passed: true,
        sla_violations: Vec::new(),
        execution_log: recs,
    };

    let json_str = export_samples_json(&report).expect("json exports cleanly");
    let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("valid json");
    assert_eq!(parsed["aggregate_endpoints"].as_array().unwrap().len(), 2);
    assert_eq!(parsed["report"]["execution_log"].as_array().unwrap().len(), 4);

    let html = generate_html_report("CI Smoke Load", &report);
    assert!(html.contains("CI Smoke Load"));
    assert!(html.contains("SLA PASSED"));
    assert!(html.contains("<svg"));
    assert!(html.contains("Per-Endpoint Aggregate Report"));
    assert!(html.contains("Get Items"));
}

#[tokio::test]
async fn test_headless_aura_load_cli_binary_and_sla_exit_codes() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{addr}/health");

    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let mut buf = vec![0u8; 512];
                let _ = stream.read(&mut buf).await;
                let body = r#"{"status":"healthy"}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.shutdown().await;
            });
        }
    });

    let tmp_dir = std::env::temp_dir().join(format!("aura_cli_load_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&tmp_dir).unwrap();
    let req_path = tmp_dir.join("health.aura");
    let csv_out = tmp_dir.join("samples.csv");
    let json_out = tmp_dir.join("report.json");
    let html_out = tmp_dir.join("summary.html");

    fs::write(
        &req_path,
        format!(
            "version = \"1.0\"\nname = \"Health Ping\"\nmethod = \"GET\"\nurl = \"{url}\"\n"
        ),
    )
    .unwrap();

    let bin = env!("CARGO_BIN_EXE_aura");
    let req_arg = req_path.to_string_lossy().to_string();
    let csv_arg = csv_out.to_string_lossy().to_string();
    let json_arg = json_out.to_string_lossy().to_string();
    let html_arg = html_out.to_string_lossy().to_string();

    let out = tokio::task::spawn_blocking(move || {
        Command::new(bin)
            .args([
                "load",
                &req_arg,
                "--vus",
                "2",
                "--iterations",
                "4",
                "--sla-p95",
                "5000",
                "--sla-error",
                "0.0",
                "--export-csv",
                &csv_arg,
                "--export-json",
                &json_arg,
                "--export-html",
                &html_arg,
            ])
            .output()
    })
    .await
    .unwrap()
    .expect("execute aura load CLI");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "expected exit code 0, got {:?}\nstdout:\n{}\nstderr:\n{}",
        out.status,
        stdout,
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("Health Ping"));
    assert!(stdout.contains("SLA GATES: PASSED"));
    assert!(csv_out.exists() && fs::read_to_string(&csv_out).unwrap().contains("Health Ping"));
    assert!(json_out.exists() && fs::read_to_string(&json_out).unwrap().contains("aggregate_endpoints"));
    assert!(html_out.exists() && fs::read_to_string(&html_out).unwrap().contains("<svg"));

    // Verify failing SLA gate exits cleanly with non-zero code (2) without crashing
    let req_arg2 = req_path.to_string_lossy().to_string();
    let fail_out = tokio::task::spawn_blocking(move || {
        Command::new(bin)
            .args([
                "load",
                &req_arg2,
                "--vus",
                "1",
                "--iterations",
                "2",
                "--max-size",
                "1", // Forces every 20-byte body response to fail size assertion -> 100% error
                "--sla-error",
                "0.0",
                "--quiet",
            ])
            .output()
    })
    .await
    .unwrap()
    .expect("execute failing aura load CLI");
    assert_eq!(fail_out.status.code(), Some(2));

    let _ = fs::remove_dir_all(&tmp_dir);
}
