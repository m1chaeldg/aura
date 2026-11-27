use aura_core::assertions::{AssertOperator, DeclarativeAssertion};
use aura_core::model::{AuraRequest, BodyType, HeaderItem, HttpMethod, ScriptConfig};
use aura_core::templating::TemplateContext;
use aura_engine::execute_request;
use aura_engine::runner::{run_collection, RunnerConfig};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn spawn_mock_server(
    handler: impl Fn(String) -> (u16, Vec<(String, String)>, String) + Send + Sync + 'static,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");
    let handler = Arc::new(handler);

    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let handler = Arc::clone(&handler);
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                if n == 0 {
                    return;
                }
                let raw = String::from_utf8_lossy(&buf[..n]).to_string();
                let (status, extra_headers, body) = handler(raw);
                let mut resp = format!(
                    "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
                    body.len()
                );
                for (k, v) in extra_headers {
                    resp.push_str(&format!("{k}: {v}\r\n"));
                }
                resp.push_str("\r\n");
                resp.push_str(&body);
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.shutdown().await;
            });
        }
    });

    (base_url, handle)
}

#[tokio::test]
async fn test_pre_and_post_script_async_fetch_and_send_request() {
    let audit_hits = Arc::new(Mutex::new(Vec::<String>::new()));
    let audit_hits_srv = Arc::clone(&audit_hits);

    let (base_url, _srv) = spawn_mock_server(move |raw| {
        let first_line = raw.lines().next().unwrap_or("");
        if first_line.starts_with("GET /token") {
            return (
                200,
                vec![],
                r#"{"access_token":"secret-99"}"#.to_string(),
            );
        }
        if first_line.starts_with("POST /audit") {
            audit_hits_srv.lock().unwrap().push(raw);
            return (200, vec![], r#"{"recorded":true}"#.to_string());
        }
        if first_line.starts_with("GET /me") {
            let auth_line = raw
                .lines()
                .find(|l| l.to_ascii_lowercase().starts_with("authorization:"))
                .unwrap_or("")
                .to_string();
            let auth_val = auth_line
                .split_once(':')
                .map(|(_, v)| v.trim().to_string())
                .unwrap_or_default();
            let json_body = serde_json::json!({
                "seen_auth": auth_val,
                "user": { "id": 77, "name": "Ada" }
            })
            .to_string();
            return (200, vec![], json_body);
        }
        (404, vec![], r#"{"error":"not found"}"#.to_string())
    })
    .await;

    let mut req = AuraRequest::new("Async Scripted Request", HttpMethod::GET, format!("{base_url}/me"));
    req.headers.push(HeaderItem::new("Authorization", "Bearer {{dyn_token}}"));
    req.scripts = ScriptConfig {
        pre_request: format!(
            r#"
            const res = await fetch("{base_url}/token");
            const json = await res.json();
            aura.variables.set("dyn_token", json.access_token);
            "#
        ),
        post_response: format!(
            r#"
            const audit = await aura.sendRequest({{
                url: "{base_url}/audit",
                method: "POST",
                headers: {{ "content-type": "application/json" }},
                body: {{ checked: response.status }}
            }});
            console.log("audit-status=" + audit.status);
            "#
        ),
        tests: r#"
            describe("Main & Sub-Request", () => {
                it("injected pre-request fetched token", () => {
                    const data = response.json();
                    expect(data.seen_auth).toBe("Bearer secret-99");
                });
            });
        "#
        .to_string(),
        assertions: vec![
            DeclarativeAssertion::new("res.status", AssertOperator::Eq, "200"),
            DeclarativeAssertion::new("res.body.user.id", AssertOperator::IsNumber, ""),
            DeclarativeAssertion::new("res.body.seen_auth", AssertOperator::Contains, "secret-99"),
        ],
    };

    let ctx = TemplateContext::default();
    let resp = execute_request(&req, &ctx)
        .await
        .expect("scripted request completes");

    assert_eq!(resp.status, 200);
    assert!(
        resp.script_errors.is_empty(),
        "unexpected script errors: {:?}",
        resp.script_errors
    );
    // 3 declarative + 1 JS test
    assert_eq!(resp.assertions.len(), 4);
    assert!(
        resp.assertions.iter().all(|a| a.passed),
        "expected all assertions to pass: {:#?}",
        resp.assertions
    );
    assert!(
        resp.console_logs
            .iter()
            .any(|l| l.message.contains("audit-status=200")),
        "missing audit console log: {:?}",
        resp.console_logs
    );
    assert_eq!(audit_hits.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn test_collection_runner_chaining_branch_and_stop() {
    let paths_hit = Arc::new(Mutex::new(Vec::<String>::new()));
    let paths_hit_srv = Arc::clone(&paths_hit);

    let (base_url, _srv) = spawn_mock_server(move |raw| {
        let first_line = raw.lines().next().unwrap_or("").to_string();
        paths_hit_srv.lock().unwrap().push(first_line.clone());

        if first_line.starts_with("POST /login") {
            return (
                200,
                vec![("Set-Cookie".to_string(), "sid=cookie42; Path=/".to_string())],
                r#"{"token":"tok_abc"}"#.to_string(),
            );
        }
        if first_line.starts_with("GET /profile") {
            let has_cookie = raw.to_ascii_lowercase().contains("cookie:")
                && raw.contains("sid=cookie42");
            let json = serde_json::json!({
                "path": first_line,
                "cookie_present": has_cookie
            })
            .to_string();
            return (200, vec![], json);
        }
        if first_line.starts_with("GET /bypassed") {
            return (200, vec![], r#"{"should_not_run":true}"#.to_string());
        }
        (404, vec![], "{}".to_string())
    })
    .await;

    let mut req_login = AuraRequest::new("01 Login", HttpMethod::POST, format!("{base_url}/login"));
    req_login.body = BodyType::Json("{}".to_string());
    req_login.scripts.post_response = r#"
        const data = response.json();
        aura.variables.set("session_tok", data.token);
        aura.setNextRequest("03 Profile");
    "#
    .to_string();

    let req_bypassed = AuraRequest::new(
        "02 Bypassed",
        HttpMethod::GET,
        format!("{base_url}/bypassed"),
    );

    let mut req_profile = AuraRequest::new(
        "03 Profile",
        HttpMethod::GET,
        format!("{base_url}/profile?tok={{{{session_tok}}}}"),
    );
    req_profile.scripts.post_response = r#"
        aura.setNextRequest(null);
    "#
    .to_string();
    req_profile.scripts.assertions = vec![
        DeclarativeAssertion::new("res.status", AssertOperator::Eq, "200"),
        DeclarativeAssertion::new("res.body.cookie_present", AssertOperator::Eq, "true"),
        DeclarativeAssertion::new("res.body.path", AssertOperator::Contains, "tok=tok_abc"),
    ];

    let req_trailing = AuraRequest::new(
        "04 After Stop",
        HttpMethod::GET,
        format!("{base_url}/bypassed"),
    );

    let report = run_collection(
        vec![req_login, req_bypassed, req_profile, req_trailing],
        TemplateContext::default(),
        RunnerConfig {
            total_iterations: 1,
            concurrency: 1,
            delay_ms: 0,
            ..Default::default()
        },
        None,
    )
    .await;

    assert_eq!(report.total_requests, 2);
    assert_eq!(report.successful_requests, 2);
    assert_eq!(report.failed_requests, 0);

    let executed_names: Vec<&str> = report
        .execution_log
        .iter()
        .map(|r| r.request_name.as_str())
        .collect();
    assert_eq!(executed_names, vec!["01 Login", "03 Profile"]);

    let server_requests = paths_hit.lock().unwrap();
    assert_eq!(server_requests.len(), 2);
    assert!(server_requests[0].starts_with("POST /login"));
    assert!(server_requests[1].starts_with("GET /profile?tok=tok_abc"));
}
