use aura_core::assertions::{
    evaluate_assertions, parse_assert_block, parse_assertion_line, parse_assertion_kv,
    serialize_assertion_kv, AssertOperator, DeclarativeAssertion,
};
use aura_core::model::AuraResponse;
use aura_scripting::AuraScriptRuntime;
use std::collections::HashMap;
use std::time::SystemTime;

fn sample_response() -> AuraResponse {
    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        "application/json; charset=utf-8".to_string(),
    );
    headers.insert("x-request-id".to_string(), "req-42".to_string());

    AuraResponse {
        status: 200,
        status_text: "OK".to_string(),
        duration_ms: 85,
        size_bytes: 128,
        headers,
        body: r#"{"id":42,"active":true,"tags":["alpha","beta"],"items":[{"name":"widget-1"}]}"#
            .to_string(),
        timestamp: SystemTime::now().into(),
        assertions: Vec::new(),
        console_logs: Vec::new(),
        script_errors: Vec::new(),
        sse_events: Vec::new(),
        stream_messages: Vec::new(),
        timing: None,
        captured_cookies: Vec::new(),
        next_request: None,
        stop_chain: false,
        exported_variables: HashMap::new(),
    }
}

#[test]
fn test_declarative_assertions_parsing_and_evaluation() {
    let block = r#"
        res.status == 200
        res.duration_ms < 500
        res.body.id isNumber
        res.body.active isBoolean
        res.body.tags isArray
        res.body.items[0].name contains "widget"
        res.headers.content-type contains "application/json"
        res.headers.x-request-id matches ^req-\d+$
        ~res.status == 404
    "#;

    let rules = parse_assert_block(block);
    assert_eq!(rules.len(), 9);
    assert!(!rules[8].enabled, "tilde prefix must disable rule");

    let resp = sample_response();
    let results = evaluate_assertions(&rules, &resp);
    // Disabled rule is skipped
    assert_eq!(results.len(), 8);
    assert!(
        results.iter().all(|r| r.passed),
        "all enabled rules should pass: {results:#?}"
    );

    // Verify failing rule output message
    let failing = DeclarativeAssertion::new("res.status", AssertOperator::Eq, "201");
    let fail_res = evaluate_assertions(&[failing], &resp);
    assert_eq!(fail_res.len(), 1);
    assert!(!fail_res[0].passed);
    assert!(fail_res[0].message.as_deref().unwrap_or("").contains("200"));

    // Verify KV roundtrip for [assert] TOML table syntax
    let kv_rule = parse_assertion_kv("res.body.id", "isNumber").expect("kv parses");
    assert_eq!(kv_rule.operator, AssertOperator::IsNumber);
    let (k, v) = serialize_assertion_kv(&kv_rule);
    assert_eq!(k, "res.body.id");
    assert_eq!(v, "isNumber");

    let eq_line = parse_assertion_line("res.status==200").expect("no-space syntax parses");
    assert_eq!(eq_line.operator, AssertOperator::Eq);
    assert_eq!(eq_line.expected, "200");
}

#[test]
fn test_aura_file_parser_roundtrip_with_assertions_and_examples() {
    let raw = r#"version = "1.0"
name = "Assertion & Test Combo"
method = "GET"
url = "https://postman-echo.com/get"

[tests]
```javascript
it("checks ok", () => expect(response.status).toBe(200));
```

[assert]
res.status = "== 200"
res.body.id = "isNumber"
~res.duration_ms = "< 10"
"#;

    let parsed = aura_parser::parse_request(raw).expect("sample parses cleanly");
    assert!(parsed.scripts.tests.contains("checks ok"));
    assert_eq!(parsed.scripts.assertions.len(), 3);
    assert_eq!(parsed.scripts.assertions[0].expression, "res.status");
    assert_eq!(parsed.scripts.assertions[0].operator, AssertOperator::Eq);
    assert_eq!(parsed.scripts.assertions[0].expected, "200");
    assert_eq!(
        parsed.scripts.assertions[1].operator,
        AssertOperator::IsNumber
    );
    assert!(!parsed.scripts.assertions[2].enabled);

    let serialized = aura_parser::request_to_spec(&parsed);
    let reparsed = aura_parser::parse_request(&serialized).expect("re-serialized parses cleanly");
    assert_eq!(reparsed.scripts.assertions, parsed.scripts.assertions);
    assert!(reparsed.scripts.tests.contains("checks ok"));

    // Validate the documentation sample collection files actually parse cleanly
    let login = aura_parser::parse_request(include_str!("../docs/examples/chaining/01_login.aura"))
        .expect("01_login.aura parses");
    assert_eq!(login.name, "01 Login and Extract Token");
    assert_eq!(login.scripts.assertions.len(), 3);
    assert!(login.scripts.pre_request.contains("fetch("));
    assert!(login.scripts.post_response.contains("setNextRequest"));

    let profile = aura_parser::parse_request(include_str!(
        "../docs/examples/chaining/02_fetch_profile.aura"
    ))
    .expect("02_fetch_profile.aura parses");
    assert_eq!(profile.name, "02 Fetch User Profile");
    assert_eq!(profile.scripts.assertions.len(), 3);
    assert!(profile.scripts.post_response.contains("sendRequest"));

    let fallback = aura_parser::parse_request(include_str!(
        "../docs/examples/chaining/03_fallback_error.aura"
    ))
    .expect("03_fallback_error.aura parses");
    assert_eq!(fallback.name, "03 Fallback Auth Error");
    assert_eq!(fallback.scripts.assertions.len(), 1);
}

#[test]
fn test_script_runtime_set_next_request_and_pm_aliases() {
    let mut rt = AuraScriptRuntime::new(HashMap::new()).unwrap();
    let res = rt
        .run(
            r#"
            aura.setNextRequest("Fetch User Profile");
            aura.variables.set("token", "xyz");
            "#,
        )
        .expect("script runs");
    assert_eq!(res.next_request.as_deref(), Some("Fetch User Profile"));
    assert!(!res.stop_chain);
    assert_eq!(res.variables.get("token").map(String::as_str), Some("xyz"));

    let mut rt_stop = AuraScriptRuntime::new(HashMap::new()).unwrap();
    let res_stop = rt_stop
        .run(
            r#"
            aura.setNextRequest("Something");
            aura.setNextRequest(null);
            "#,
        )
        .expect("stop script runs");
    assert!(res_stop.stop_chain);
    assert!(res_stop.next_request.is_none());

    let mut rt_pm = AuraScriptRuntime::new(HashMap::new()).unwrap();
    let res_pm = rt_pm
        .run(r#"pm.execution.setNextRequest("Step PM");"#)
        .expect("pm alias runs");
    assert_eq!(res_pm.next_request.as_deref(), Some("Step PM"));
    assert!(!res_pm.stop_chain);
}
