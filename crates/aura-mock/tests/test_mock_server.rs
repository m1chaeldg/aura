use aura_core::mock::{
    MatchOperator, MockBodyMatcher, MockDatasetConfig, MockDatasetSource, MockHeaderMatcher,
    MockPathMatcher, MockQueryMatcher, MockRequestRule, MockResponseMode, MockResponseSpec,
    MockRouteRule, MockServerConfigFile, MockUnmatchedPolicy,
};
use aura_mock::MockServer;
use std::collections::HashMap;

#[tokio::test]
async fn test_mock_server_rules_templates_crud_script_and_passthrough() {
    // 1. Start an upstream server for proxy passthrough testing
    let mut upstream_cfg = MockServerConfigFile::default();
    upstream_cfg.port = 0;
    upstream_cfg.routes.push(MockRouteRule::simple(
        "GET",
        "/upstream-ping",
        200,
        r#"{"upstream":true}"#,
    ));
    let upstream = MockServer::from_config(upstream_cfg).start().await.unwrap();
    let upstream_url = format!("http://{}", upstream.listen_addr);

    // 2. Build main mock server config
    let mut cfg = MockServerConfigFile::default();
    cfg.port = 0;
    cfg.max_body_bytes = 2048;
    cfg.unmatched = MockUnmatchedPolicy::ProxyPassthrough {
        target_base_url: upstream_url,
    };

    // Dataset with stateful CRUD
    cfg.datasets.push(MockDatasetConfig {
        name: "users".to_string(),
        source: MockDatasetSource::InlineJson {
            rows: vec![serde_json::json!({"id": "1", "name": "Alice"})],
            stateful_crud: true,
            id_field: "id".to_string(),
        },
    });

    // Template + faker + path param + header rule
    let mut tpl_rule = MockRouteRule::simple("GET", "/greet/:name", 200, "");
    tpl_rule.rule.headers.push(MockHeaderMatcher {
        name: "X-Env".to_string(),
        operator: MatchOperator::Equals,
        value: "dev".to_string(),
    });
    tpl_rule.rule.query.push(MockQueryMatcher {
        name: "lang".to_string(),
        operator: MatchOperator::Equals,
        value: "en".to_string(),
    });
    tpl_rule.response = MockResponseSpec {
        mode: MockResponseMode::Template,
        status: 200,
        headers: HashMap::from([("X-Mock-User".to_string(), "{{req.params.name}}".to_string())]),
        body: r#"{"hello":"{{req.params.name}}","lang":"{{req.query.lang}}","id":"{{$uuid}}"}"#.to_string(),
        ..Default::default()
    };
    cfg.routes.push(tpl_rule);

    // Stateful CRUD routes backed by "users" dataset
    for (method, path) in [
        ("GET", "/api/users"),
        ("GET", "/api/users/:id"),
        ("POST", "/api/users"),
        ("PATCH", "/api/users/:id"),
        ("DELETE", "/api/users/:id"),
    ] {
        let mut r = MockRouteRule::simple(method, path, 200, "");
        r.response.mode = MockResponseMode::Dataset;
        r.response.dataset_name = Some("users".to_string());
        cfg.routes.push(r);
    }

    // deno_core JS Script route with JSONPath body matcher
    let mut script_rule = MockRouteRule {
        id: "script-calc".to_string(),
        name: "Script Calc".to_string(),
        enabled: true,
        priority: 10,
        rule: MockRequestRule {
            methods: vec!["POST".to_string()],
            path: MockPathMatcher::Exact("/calc".to_string()),
            query: vec![],
            headers: vec![],
            body: Some(MockBodyMatcher::JsonPath {
                path: "$.op".to_string(),
                operator: MatchOperator::Equals,
                value: "double".to_string(),
            }),
        },
        response: MockResponseSpec {
            mode: MockResponseMode::Script,
            status: 200,
            headers: HashMap::new(),
            body: String::new(),
            script: r#"
                const payload = req.json();
                return {
                    status: 200,
                    headers: { "X-Engine": "deno_core" },
                    body: { result: payload.value * 2 }
                };
            "#
            .to_string(),
            dataset_name: None,
            delay_ms: None,
        },
    };
    cfg.routes.push(script_rule.clone());
    // Ensure unused mut warning isn't triggered
    script_rule.priority = 10;

    let handle = MockServer::from_config(cfg).start().await.unwrap();
    let base = format!("http://{}", handle.listen_addr);
    let client = reqwest::Client::new();

    // Test Template + Faker + Header/Query match
    let r1 = client
        .get(format!("{base}/greet/bob?lang=en"))
        .header("X-Env", "dev")
        .send()
        .await
        .unwrap();
    assert_eq!(r1.status(), 200);
    assert_eq!(r1.headers().get("x-mock-user").unwrap(), "bob");
    let j1: serde_json::Value = r1.json().await.unwrap();
    assert_eq!(j1["hello"], "bob");
    assert_eq!(j1["lang"], "en");
    assert!(!j1["id"].as_str().unwrap().is_empty());

    // Test Stateful CRUD (POST -> GET -> PATCH -> DELETE)
    let created: serde_json::Value = client
        .post(format!("{base}/api/users"))
        .body(r#"{"id":"2","name":"Bob"}"#)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(created["name"], "Bob");

    let fetched: serde_json::Value = client
        .get(format!("{base}/api/users/2"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fetched["name"], "Bob");

    let updated: serde_json::Value = client
        .patch(format!("{base}/api/users/2"))
        .body(r#"{"name":"Bobby"}"#)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(updated["name"], "Bobby");

    let del = client
        .delete(format!("{base}/api/users/2"))
        .send()
        .await
        .unwrap();
    assert_eq!(del.status(), 200);

    // Test deno_core Script + JSONPath body matcher
    let calc_resp = client
        .post(format!("{base}/calc"))
        .body(r#"{"op":"double","value":21}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(calc_resp.status(), 200);
    assert_eq!(calc_resp.headers().get("x-engine").unwrap(), "deno_core");
    let calc_json: serde_json::Value = calc_resp.json().await.unwrap();
    assert_eq!(calc_json["result"], 42);

    // Test ProxyPassthrough on unmatched route
    let up_resp: serde_json::Value = client
        .get(format!("{base}/upstream-ping"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(up_resp["upstream"], true);

    // Test max_body_bytes 413 enforcement
    let oversized = "x".repeat(4096);
    let big_resp = client
        .post(format!("{base}/api/users"))
        .body(oversized)
        .send()
        .await
        .unwrap();
    assert_eq!(big_resp.status(), 413);

    // Verify hit logs recorded
    assert!(!handle.hit_logs().is_empty());

    handle.stop();
    upstream.stop();
}
