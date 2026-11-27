use aura_core::assertions::{evaluate_flow_condition, PerformanceAssertionConfig};
use aura_core::data_feeder::{DataEofBehavior, DataFeederConfig, DataSharingMode};
use aura_core::extractors::{NativeExtractor, NativeExtractorType};
use aura_core::model::{AuraRequest, BodyType, FlowControl, HttpMethod, ThinkTimeConfig};
use aura_core::templating::TemplateContext;
use aura_engine::runner::{
    run_collection, LoadStage, ResolvedSchedule, RunnerConfig, SlaThresholds,
    SynchronizingTimerConfig,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn spawn_fast_echo_server() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{addr}/ping");

    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let mut buf = vec![0u8; 1024];
                let _ = stream.read(&mut buf).await;
                let body = r#"{"ok":true}"#;
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

    (url, handle)
}

#[test]
fn test_resolved_schedule_ramp_up_down_and_stages() {
    let ramp_cfg = RunnerConfig {
        concurrency: 10,
        ramp_up_secs: 1.0,
        duration_secs: Some(4.0),
        ramp_down_secs: 1.0,
        ..Default::default()
    };
    let sched = ResolvedSchedule::from_config(&ramp_cfg);
    assert!(sched.is_time_boxed);
    assert_eq!(sched.max_vus, 10);
    assert_eq!(sched.active_vus_at(Duration::from_millis(100)), Some(1));
    assert_eq!(sched.active_vus_at(Duration::from_millis(500)), Some(5));
    assert_eq!(sched.active_vus_at(Duration::from_millis(1500)), Some(10));
    assert_eq!(sched.active_vus_at(Duration::from_millis(3500)), Some(5));
    assert_eq!(sched.active_vus_at(Duration::from_millis(4050)), None);

    let stage_cfg = RunnerConfig {
        stages: vec![
            LoadStage::new(2, 1.0),
            LoadStage::new(10, 2.0),
            LoadStage::new(2, 1.0),
        ],
        ..Default::default()
    };
    let s_sched = ResolvedSchedule::from_config(&stage_cfg);
    assert_eq!(s_sched.max_vus, 10);
    assert_eq!(s_sched.active_vus_at(Duration::from_millis(200)), Some(2));
    assert_eq!(s_sched.active_vus_at(Duration::from_millis(1500)), Some(10));
    assert_eq!(s_sched.active_vus_at(Duration::from_millis(3200)), Some(2));
    assert_eq!(s_sched.active_vus_at(Duration::from_millis(4100)), None);
}

#[tokio::test]
async fn test_duration_based_soak_execution_and_ramp() {
    let (url, _srv) = spawn_fast_echo_server().await;
    let req = AuraRequest::new("Soak Ping", HttpMethod::GET, url);

    let cfg = RunnerConfig {
        concurrency: 4,
        delay_ms: 25,
        ramp_up_secs: 0.16,
        duration_secs: Some(0.40),
        ramp_down_secs: 0.10,
        ..Default::default()
    };

    let report = run_collection(vec![req], TemplateContext::default(), cfg, None).await;
    assert!(
        report.total_requests >= 4,
        "expected multiple requests in duration run, got {}",
        report.total_requests
    );
    assert_eq!(report.failed_requests, 0);
    assert!(report.total_duration_ms >= 350);
    assert!(
        report.peak_vus >= 3,
        "expected VUs to ramp up near 4, peak_vus={}",
        report.peak_vus
    );
    assert!(
        report
            .execution_log
            .iter()
            .any(|r| r.active_vus < 4),
        "expected ramp-up/down phase with <4 VUs: {:?}",
        report.execution_log
    );
}

#[tokio::test]
async fn test_stepped_spike_profile_scales_vus_up_and_back_down() {
    let (url, _srv) = spawn_fast_echo_server().await;
    let req = AuraRequest::new("Spike Ping", HttpMethod::GET, url);

    let cfg = RunnerConfig {
        delay_ms: 40,
        stages: vec![
            LoadStage::new(1, 0.22),
            LoadStage::new(6, 0.25),
            LoadStage::new(1, 0.22),
        ],
        ..Default::default()
    };

    let report = run_collection(vec![req], TemplateContext::default(), cfg, None).await;
    assert_eq!(report.failed_requests, 0);
    assert_eq!(report.peak_vus, 6);

    let has_stage1 = report.execution_log.iter().any(|r| r.active_vus == 1 && r.elapsed_ms < 220);
    let has_stage2 = report.execution_log.iter().any(|r| r.active_vus == 6);
    let has_stage3 = report.execution_log.iter().any(|r| r.active_vus == 1 && r.elapsed_ms >= 470);
    assert!(has_stage1, "missing baseline stage 1 sample: {:?}", report.execution_log);
    assert!(has_stage2, "missing peak spike stage 2 sample: {:?}", report.execution_log);
    assert!(has_stage3, "missing recovery stage 3 sample: {:?}", report.execution_log);
}

#[tokio::test]
async fn test_target_throughput_shaping_constant_rps() {
    let (url, _srv) = spawn_fast_echo_server().await;
    let req = AuraRequest::new("Shaped Ping", HttpMethod::GET, url);

    // Without throttling, 6 VUs against localhost would send hundreds in 0.5s.
    // With target_rps = 12.0 over 0.5s, expected is ~6 requests (strictly capped <= 10).
    let cfg = RunnerConfig {
        concurrency: 6,
        delay_ms: 0,
        duration_secs: Some(0.50),
        target_rps: Some(12.0),
        ..Default::default()
    };

    let report = run_collection(vec![req], TemplateContext::default(), cfg, None).await;
    assert_eq!(report.failed_requests, 0);
    assert_eq!(report.target_rps, Some(12.0));
    assert!(
        (4..=10).contains(&report.total_requests),
        "expected throttled count ~6 within 4..=10, got {}",
        report.total_requests
    );
    assert!(
        report.requests_per_second <= 20.0,
        "expected shaped RPS <= 20/s, got {}",
        report.requests_per_second
    );
}

#[tokio::test]
async fn test_random_uniform_and_gaussian_timers() {
    for _ in 0..25 {
        let u = (ThinkTimeConfig::Uniform { min_ms: 40, max_ms: 80 })
            .sample_duration()
            .unwrap();
        assert!((40..=80).contains(&(u.as_millis() as u64)));

        let g = (ThinkTimeConfig::Gaussian {
            mean_ms: 50.0,
            stddev_ms: 15.0,
        })
        .sample_duration();
        if let Some(d) = g {
            assert!(d.as_millis() < 500);
        }
    }

    let (url, _srv) = spawn_fast_echo_server().await;
    let req = AuraRequest::new("Timer Ping", HttpMethod::GET, url);
    let cfg = RunnerConfig {
        total_iterations: 4,
        concurrency: 1,
        think_time: ThinkTimeConfig::Uniform {
            min_ms: 25,
            max_ms: 45,
        },
        ..Default::default()
    };
    let report = run_collection(vec![req], TemplateContext::default(), cfg, None).await;
    assert_eq!(report.total_requests, 4);
    assert!(
        report.total_duration_ms >= 90,
        "expected uniform think-time across 4 sequential iterations: {} ms",
        report.total_duration_ms
    );
}

#[tokio::test]
async fn test_synchronizing_timer_rendezvous_barrier_simultaneous_release() {
    let arrivals = Arc::new(Mutex::new(Vec::<std::time::Instant>::new()));
    let arrivals_srv = Arc::clone(&arrivals);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{addr}/burst");

    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let arrivals_srv = Arc::clone(&arrivals_srv);
            tokio::spawn(async move {
                let mut buf = vec![0u8; 512];
                let _ = stream.read(&mut buf).await;
                arrivals_srv.lock().unwrap().push(std::time::Instant::now());
                let resp = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.shutdown().await;
            });
        }
    });

    let req = AuraRequest::new("Rendezvous Burst", HttpMethod::GET, url);
    let cfg = RunnerConfig {
        total_iterations: 4,
        concurrency: 4,
        ramp_up_secs: 0.12, // Stagger VU starts; barrier aligns them back together before sending!
        sync_barrier: Some(SynchronizingTimerConfig {
            group_size: 4,
            timeout_ms: 3000,
        }),
        ..Default::default()
    };

    let report = run_collection(vec![req], TemplateContext::default(), cfg, None).await;
    assert_eq!(report.total_requests, 4);
    assert_eq!(report.successful_requests, 4);

    let times = arrivals.lock().unwrap();
    assert_eq!(times.len(), 4);
    let min_t = *times.iter().min().unwrap();
    let max_t = *times.iter().max().unwrap();
    let spread = max_t.duration_since(min_t);
    assert!(
        spread <= Duration::from_millis(180),
        "expected simultaneous barrier release within 180ms, got {:?}",
        spread
    );
}

#[tokio::test]
async fn test_once_only_and_weighted_and_loop_if_while_controllers() {
    let counts = Arc::new(Mutex::new(HashMap::<String, usize>::new()));
    let poll_counter = Arc::new(AtomicUsize::new(0));
    let counts_srv = Arc::clone(&counts);
    let poll_srv = Arc::clone(&poll_counter);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");

    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let counts_srv = Arc::clone(&counts_srv);
            let poll_srv = Arc::clone(&poll_srv);
            tokio::spawn(async move {
                let mut buf = vec![0u8; 2048];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let raw = String::from_utf8_lossy(&buf[..n]).to_string();
                let line = raw.lines().next().unwrap_or("");
                let path = line.split_whitespace().nth(1).unwrap_or("/");
                *counts_srv.lock().unwrap().entry(path.to_string()).or_default() += 1;

                let (status, body) = if path == "/login" {
                    (200, r#"{"token":"vu-tok"}"#.to_string())
                } else if path == "/poll" {
                    let c = poll_srv.fetch_add(1, Ordering::SeqCst) + 1;
                    if c < 3 {
                        (200, r#"{"ready":false}"#.to_string())
                    } else {
                        (200, r#"{"ready":true}"#.to_string())
                    }
                } else {
                    (200, format!(r#"{{"path":"{path}"}}"#))
                };

                let resp = format!(
                    "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.shutdown().await;
            });
        }
    });

    // 1. Once-Only Controller + If + Loop + While controllers
    let mut r_login = AuraRequest::new("Login Once", HttpMethod::POST, format!("{base_url}/login"));
    r_login.body = BodyType::Json("{}".to_string());
    r_login.scripts.post_response = "aura.variables.set(\"role\", \"member\");".to_string();
    r_login.flow = Some(FlowControl {
        once_per_run: true,
        ..Default::default()
    });

    let mut r_skipped = AuraRequest::new("Admin Skipped", HttpMethod::GET, format!("{base_url}/admin"));
    r_skipped.flow = Some(FlowControl {
        condition_if: Some("vars.role == admin".to_string()),
        ..Default::default()
    });

    let mut r_looped = AuraRequest::new("Looped Item", HttpMethod::GET, format!("{base_url}/item"));
    r_looped.flow = Some(FlowControl {
        condition_if: Some("vars.role == member".to_string()),
        loop_count: Some(3),
        ..Default::default()
    });

    let mut r_while = AuraRequest::new("While Poll", HttpMethod::GET, format!("{base_url}/poll"));
    r_while.flow = Some(FlowControl {
        once_per_run: true,
        loop_while: Some("res.body.ready == false".to_string()),
        max_loops: 5,
        ..Default::default()
    });

    // Run 4 iterations with 1 VU -> Login runs ONCE, Admin Skipped 0 times, Item runs 4*3=12 times, Poll runs once (looping 3 times until ready:true)
    let cfg = RunnerConfig {
        total_iterations: 4,
        concurrency: 1,
        ..Default::default()
    };
    let report = run_collection(
        vec![r_login, r_skipped, r_looped, r_while],
        TemplateContext::default(),
        cfg,
        None,
    )
    .await;
    assert_eq!(report.failed_requests, 0);

    {
        let map = counts.lock().unwrap();
        assert_eq!(map.get("/login").copied(), Some(1), "Once-Only login should run exactly once per VU");
        assert_eq!(map.get("/admin").copied(), None, "If controller should skip /admin");
        assert_eq!(map.get("/item").copied(), Some(12), "Loop count 3 * 4 iterations = 12");
        assert_eq!(map.get("/poll").copied(), Some(3), "While loop should stop on 3rd poll when ready=true");
    }

    // 2. Throughput / Weighted Controller (70% / 20% / 10%)
    counts.lock().unwrap().clear();
    let mut w_items = AuraRequest::new("Items", HttpMethod::GET, format!("{base_url}/w_items"));
    w_items.flow = Some(FlowControl { weight_percent: Some(70), ..Default::default() });
    let mut w_cart = AuraRequest::new("Cart", HttpMethod::POST, format!("{base_url}/w_cart"));
    w_cart.flow = Some(FlowControl { weight_percent: Some(20), ..Default::default() });
    let mut w_checkout = AuraRequest::new("Checkout", HttpMethod::POST, format!("{base_url}/w_checkout"));
    w_checkout.flow = Some(FlowControl { weight_percent: Some(10), ..Default::default() });

    let w_cfg = RunnerConfig {
        total_iterations: 100,
        concurrency: 4,
        weighted_routing: true,
        ..Default::default()
    };
    let w_report = run_collection(
        vec![w_items, w_cart, w_checkout],
        TemplateContext::default(),
        w_cfg,
        None,
    )
    .await;
    assert_eq!(w_report.total_requests, 100);
    let map = counts.lock().unwrap();
    let items_n = map.get("/w_items").copied().unwrap_or(0);
    let cart_n = map.get("/w_cart").copied().unwrap_or(0);
    let chk_n = map.get("/w_checkout").copied().unwrap_or(0);
    assert_eq!(items_n + cart_n + chk_n, 100);
    assert!(items_n >= 45 && items_n > cart_n && items_n > chk_n, "items={items_n} cart={cart_n} chk={chk_n}");
    assert!(cart_n > 0 && chk_n > 0, "cart={cart_n} chk={chk_n}");

    // Pure flow condition evaluator sanity check
    let mut vars = HashMap::new();
    vars.insert("tier".to_string(), "pro".to_string());
    assert!(evaluate_flow_condition("vars.tier == pro", None, &vars));
    assert!(!evaluate_flow_condition("vars.tier == free", None, &vars));
}

#[tokio::test]
async fn test_data_feeder_csv_parameterization_and_native_extractors_and_sla_gates() {
    let seen_queries = Arc::new(Mutex::new(Vec::<String>::new()));
    let slow_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let seen_srv = Arc::clone(&seen_queries);
    let slow_srv = Arc::clone(&slow_flag);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");

    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let seen_srv = Arc::clone(&seen_srv);
            let slow_srv = Arc::clone(&slow_srv);
            tokio::spawn(async move {
                if slow_srv.load(Ordering::Relaxed) {
                    tokio::time::sleep(Duration::from_millis(35)).await;
                }
                let mut buf = vec![0u8; 2048];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let raw = String::from_utf8_lossy(&buf[..n]).to_string();
                let line = raw.lines().next().unwrap_or("").to_string();
                let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
                seen_srv.lock().unwrap().push(path.clone());

                let (extra_hdrs, body) = if path == "/session" {
                    (
                        "X-Trace: tr-55\r\nSet-Cookie: sid=s99; Path=/\r\n",
                        r#"{"data":{"token":"tok_native"}}"#.to_string(),
                    )
                } else {
                    ("", r#"{"ok":true}"#.to_string())
                };
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n{extra_hdrs}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.shutdown().await;
            });
        }
    });

    // 1. Write temp CSV file and verify AllVUs + StopVuOnEof consumes exact 3 rows across 2 VUs
    let csv_path = std::env::temp_dir().join(format!("aura_feeder_{}.csv", uuid::Uuid::new_v4()));
    std::fs::write(&csv_path, "user,plan\nalice,pro\nbob,free\ncarol,team\n").unwrap();

    let req_csv = AuraRequest::new(
        "CSV Lookup",
        HttpMethod::GET,
        format!("{base_url}/u?name={{{{user}}}}&plan={{{{plan}}}}"),
    );
    let csv_cfg = RunnerConfig {
        total_iterations: 10,
        concurrency: 2,
        data_feeder: Some(DataFeederConfig {
            enabled: true,
            file_path: csv_path.display().to_string(),
            sharing_mode: DataSharingMode::AllVUs,
            eof_behavior: DataEofBehavior::StopVuOnEof,
            ..Default::default()
        }),
        ..Default::default()
    };
    let csv_report = run_collection(vec![req_csv], TemplateContext::default(), csv_cfg, None).await;
    let _ = std::fs::remove_file(&csv_path);

    assert_eq!(csv_report.total_requests, 3, "StopVuOnEof should halt after 3 CSV rows");
    assert_eq!(csv_report.successful_requests, 3);
    {
        let q = seen_queries.lock().unwrap();
        assert!(q.iter().any(|p| p == "/u?name=alice&plan=pro"));
        assert!(q.iter().any(|p| p == "/u?name=bob&plan=free"));
        assert!(q.iter().any(|p| p == "/u?name=carol&plan=team"));
    }

    // 2. Native Zero-JS Extractors correlation (/session -> /verify with token/header/cookie)
    seen_queries.lock().unwrap().clear();
    let mut r_sess = AuraRequest::new("Get Session", HttpMethod::GET, format!("{base_url}/session"));
    r_sess.extractors = vec![
        NativeExtractor::new("tok", NativeExtractorType::JsonPath, "$.data.token"),
        NativeExtractor::new("tr", NativeExtractorType::Header, "X-Trace"),
        NativeExtractor::new("c_sid", NativeExtractorType::Cookie, "sid"),
    ];
    let r_verify = AuraRequest::new(
        "Verify Native",
        HttpMethod::GET,
        format!("{base_url}/verify?tok={{{{tok}}}}&tr={{{{tr}}}}&sid={{{{c_sid}}}}"),
    );
    let ext_report = run_collection(
        vec![r_sess, r_verify],
        TemplateContext::default(),
        RunnerConfig {
            total_iterations: 1,
            concurrency: 1,
            ..Default::default()
        },
        None,
    )
    .await;
    assert_eq!(ext_report.successful_requests, 2);
    assert!(ext_report.sla_passed);
    {
        let q = seen_queries.lock().unwrap();
        assert!(
            q.iter()
                .any(|p| p == "/verify?tok=tok_native&tr=tr-55&sid=s99"),
            "missing correlated query in {q:?}"
        );
    }

    // 3. Per-Sample Performance Assertions + Global SLA Gate Failure
    slow_flag.store(true, Ordering::Relaxed);
    let r_sla = AuraRequest::new("SLA Endpoint", HttpMethod::GET, format!("{base_url}/fast"));
    let sla_cfg = RunnerConfig {
        total_iterations: 4,
        concurrency: 2,
        sample_assertions: PerformanceAssertionConfig {
            max_duration_ms: Some(15), // Mock sleeps 35ms -> fails every sample!
            min_size_bytes: None,
            max_size_bytes: None,
        },
        sla_thresholds: Some(SlaThresholds {
            max_p95_ms: Some(20),
            max_p99_ms: Some(25),
            max_error_rate_percent: Some(5.0),
        }),
        ..Default::default()
    };
    let sla_report = run_collection(vec![r_sla], TemplateContext::default(), sla_cfg, None).await;
    assert_eq!(sla_report.failed_requests, 4);
    assert!(!sla_report.sla_passed);
    assert!(
        sla_report.sla_violations.len() >= 2,
        "expected latency & error rate SLA violations: {:?}",
        sla_report.sla_violations
    );
}

