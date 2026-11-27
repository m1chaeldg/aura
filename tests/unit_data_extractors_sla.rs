use aura_core::assertions::{evaluate_performance_assertions, PerformanceAssertionConfig};
use aura_core::cookies::CookieEntry;
use aura_core::data_feeder::{
    parse_csv_rows, parse_json_rows, DataEofBehavior, DataFeeder, DataSharingMode,
};
use aura_core::extractors::{
    evaluate_native_extractors, NativeExtractor, NativeExtractorType,
};
use aura_core::model::AuraResponse;
use std::collections::HashMap;
use std::time::SystemTime;

#[test]
fn test_csv_and_json_data_feeder_sharing_and_eof() {
    let csv = "username,note,role\n\"alice, jr\",first,admin\nbob,second,member\n";
    let rows = parse_csv_rows(csv, ',');
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get("username").map(String::as_str), Some("alice, jr"));
    assert_eq!(rows[0].get("role").map(String::as_str), Some("admin"));
    assert_eq!(rows[1].get("username").map(String::as_str), Some("bob"));

    let json = r#"[{"id": 101, "tier": "gold"}, {"id": 102, "tier": "silver"}]"#;
    let json_rows = parse_json_rows(json).expect("json feeder parses");
    assert_eq!(json_rows.len(), 2);
    assert_eq!(json_rows[0].get("id").map(String::as_str), Some("101"));

    // AllVUs + StopVuOnEof: rows consumed globally once across VUs
    let shared_feeder = DataFeeder::from_rows(
        rows.clone(),
        DataSharingMode::AllVUs,
        DataEofBehavior::StopVuOnEof,
    );
    let mut vu0 = 0;
    let mut vu1 = 0;
    let r_a = shared_feeder.next_row(&mut vu0).expect("row 0");
    let r_b = shared_feeder.next_row(&mut vu1).expect("row 1");
    assert_ne!(r_a.get("username"), r_b.get("username"));
    assert!(shared_feeder.next_row(&mut vu0).is_none(), "EOF stops cleanly");

    // PerVU + Recycle: each VU gets row 0 independently and wraps around
    let per_vu_feeder =
        DataFeeder::from_rows(rows, DataSharingMode::PerVU, DataEofBehavior::Recycle);
    let mut cur_a = 0;
    let mut cur_b = 0;
    assert_eq!(
        per_vu_feeder.next_row(&mut cur_a).unwrap().get("username").unwrap(),
        "alice, jr"
    );
    assert_eq!(
        per_vu_feeder.next_row(&mut cur_b).unwrap().get("username").unwrap(),
        "alice, jr"
    );
    assert_eq!(
        per_vu_feeder.next_row(&mut cur_a).unwrap().get("username").unwrap(),
        "bob"
    );
    // Wraps around cleanly
    assert_eq!(
        per_vu_feeder.next_row(&mut cur_a).unwrap().get("username").unwrap(),
        "alice, jr"
    );
}

#[test]
fn test_native_extractors_and_performance_assertions_zero_js() {
    let mut headers = HashMap::new();
    headers.insert("x-request-id".to_string(), "req-904".to_string());
    headers.insert(
        "set-cookie".to_string(),
        "sid=cookie_abc_99; Path=/; HttpOnly".to_string(),
    );

    let resp = AuraResponse {
        status: 200,
        status_text: "OK".to_string(),
        duration_ms: 45,
        size_bytes: 120,
        headers,
        body: r#"{"data":{"token":"tok_999","items":[{"id":77}]},"meta":"csrf=xyz_42;"}"#
            .to_string(),
        timestamp: SystemTime::now().into(),
        assertions: Vec::new(),
        console_logs: Vec::new(),
        script_errors: Vec::new(),
        sse_events: Vec::new(),
        stream_messages: Vec::new(),
        timing: None,
        captured_cookies: vec![CookieEntry {
            name: "sid".to_string(),
            value: "cookie_abc_99".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            expires: None,
            max_age: None,
            secure: false,
            http_only: true,
            same_site: None,
            host_only: true,
            expires_at: None,
        }],
        next_request: None,
        stop_chain: false,
        exported_variables: HashMap::new(),
    };

    let mut disabled = NativeExtractor::new(
        "should_skip",
        NativeExtractorType::JsonPath,
        "$.data.token",
    );
    disabled.enabled = false;

    let mut fallback = NativeExtractor::new(
        "missing_with_default",
        NativeExtractorType::JsonPath,
        "$.data.nonexistent",
    );
    fallback.default_value = Some("fallback_val".to_string());

    let extractors = vec![
        NativeExtractor::new("tok", NativeExtractorType::JsonPath, "$.data.token"),
        NativeExtractor::new("item_id", NativeExtractorType::JsonPath, "$.data.items[0].id"),
        NativeExtractor::new("csrf", NativeExtractorType::RegexBody, r"csrf=([a-z0-9_]+)"),
        NativeExtractor::new("req_num", NativeExtractorType::RegexHeader, r"x-request-id:req-(\d+)"),
        NativeExtractor::new("hdr", NativeExtractorType::Header, "X-Request-Id"),
        NativeExtractor::new("cookie_sid", NativeExtractorType::Cookie, "sid"),
        disabled,
        fallback,
    ];

    let extracted = evaluate_native_extractors(&extractors, &resp);
    assert_eq!(extracted.get("tok").map(String::as_str), Some("tok_999"));
    assert_eq!(extracted.get("item_id").map(String::as_str), Some("77"));
    assert_eq!(extracted.get("csrf").map(String::as_str), Some("xyz_42"));
    assert_eq!(extracted.get("req_num").map(String::as_str), Some("904"));
    assert_eq!(extracted.get("hdr").map(String::as_str), Some("req-904"));
    assert_eq!(
        extracted.get("cookie_sid").map(String::as_str),
        Some("cookie_abc_99")
    );
    assert!(!extracted.contains_key("should_skip"));
    assert_eq!(
        extracted.get("missing_with_default").map(String::as_str),
        Some("fallback_val")
    );

    // Performance assertions (Duration + Size bounds)
    let pass_perf = PerformanceAssertionConfig {
        max_duration_ms: Some(100),
        min_size_bytes: Some(50),
        max_size_bytes: Some(500),
    };
    let pass_res = evaluate_performance_assertions(&pass_perf, &resp);
    assert_eq!(pass_res.len(), 3);
    assert!(pass_res.iter().all(|a| a.passed));

    let fail_perf = PerformanceAssertionConfig {
        max_duration_ms: Some(20),
        min_size_bytes: Some(200),
        max_size_bytes: Some(80),
    };
    let fail_res = evaluate_performance_assertions(&fail_perf, &resp);
    assert_eq!(fail_res.len(), 3);
    assert!(fail_res.iter().all(|a| !a.passed));
}

#[test]
fn test_aura_parser_roundtrip_extractors_and_flow_bounds() {
    let raw = r#"version = "1.0"
name = "Extractor Flow Roundtrip"
method = "POST"
url = "https://example.com/auth"

[flow]
once_per_run = true
weight_percent = 70
loop_count = 2
condition_if = "prev.status == 200"
loop_while = "res.body.done == false"
max_loops = 5
max_duration_ms = 250
min_size_bytes = 10
max_size_bytes = 4096

[extractors]
auth_token = "jsonpath:$.data.token"
csrf = "regex:csrf=([a-z0-9]+)"
trace = "header:x-trace-id"
~ignored = "cookie:sid"
"#;

    let req = aura_parser::parse_request(raw).expect("parses cleanly");
    let flow = req.flow.as_ref().expect("flow parsed");
    assert!(flow.once_per_run);
    assert_eq!(flow.weight_percent, Some(70));
    assert_eq!(flow.loop_count, Some(2));
    assert_eq!(flow.condition_if.as_deref(), Some("prev.status == 200"));
    assert_eq!(flow.loop_while.as_deref(), Some("res.body.done == false"));
    assert_eq!(flow.max_loops, 5);
    assert_eq!(flow.max_duration_ms, Some(250));
    assert_eq!(flow.min_size_bytes, Some(10));
    assert_eq!(flow.max_size_bytes, Some(4096));

    assert_eq!(req.extractors.len(), 4);
    assert_eq!(req.extractors[0].kind, NativeExtractorType::JsonPath);
    assert_eq!(req.extractors[1].kind, NativeExtractorType::RegexBody);
    assert_eq!(req.extractors[2].kind, NativeExtractorType::Header);
    assert!(!req.extractors[3].enabled);

    let spec = aura_parser::request_to_spec(&req);
    let reparsed = aura_parser::parse_request(&spec).expect("reparses cleanly");
    assert_eq!(reparsed.flow, req.flow);
    assert_eq!(reparsed.extractors, req.extractors);
}
