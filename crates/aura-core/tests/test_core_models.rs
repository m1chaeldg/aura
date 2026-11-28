use aura_core::assertions::{AssertOperator, DeclarativeAssertion};
use aura_core::model::{
    AssertionResult, AuraFileRequest, AuraRequest, AuraResponse, AuthConfig, BodyType,
    ExecutionReport, FlowControl, HeaderItem, HttpMethod, ParamItem, ProxyConfig, ProxyProtocol,
    RequestProtocol, RequestTimingBreakdown, ThinkTimeConfig, VariableValue, Workspace,
    pick_weighted_index,
};
use aura_core::templating::TemplateContext;
use chrono::Utc;
use std::collections::HashMap;

#[test]
fn test_core_smoke() {
    let req = AuraRequest::new("Smoke", HttpMethod::GET, "https://httpbin.org/get");
    assert_eq!(req.name, "Smoke");
    let mut ctx = TemplateContext::new();
    ctx.environment_vars
        .insert("host".to_string(), "httpbin.org".to_string());
    assert_eq!(
        ctx.render("https://{{host}}/get"),
        "https://httpbin.org/get"
    );
}

#[test]
fn test_request_protocol_parsing_and_display() {
    assert_eq!(RequestProtocol::Http.as_str(), "HTTP");
    assert_eq!(RequestProtocol::Graphql.as_str(), "GraphQL");
    assert_eq!(RequestProtocol::Websocket.as_str(), "WS");
    assert_eq!(RequestProtocol::Sse.as_str(), "SSE");
    assert_eq!(RequestProtocol::Grpc.as_str(), "gRPC");
    assert_eq!(RequestProtocol::Soap.as_str(), "SOAP");

    assert_eq!(RequestProtocol::from_str_lenient("gql"), RequestProtocol::Graphql);
    assert_eq!(RequestProtocol::from_str_lenient("WSS"), RequestProtocol::Websocket);
    assert_eq!(RequestProtocol::from_str_lenient("event-stream"), RequestProtocol::Sse);
    assert_eq!(RequestProtocol::from_str_lenient("grpcs"), RequestProtocol::Grpc);
    assert_eq!(RequestProtocol::from_str_lenient("wsdl"), RequestProtocol::Soap);
    assert_eq!(RequestProtocol::from_str_lenient("unknown"), RequestProtocol::Http);
}

#[test]
fn test_http_method_display_and_colors() {
    let methods = [
        (HttpMethod::GET, "GET"),
        (HttpMethod::POST, "POST"),
        (HttpMethod::PUT, "PUT"),
        (HttpMethod::PATCH, "PATCH"),
        (HttpMethod::DELETE, "DELETE"),
        (HttpMethod::HEAD, "HEAD"),
        (HttpMethod::OPTIONS, "OPTIONS"),
    ];
    for (method, label) in methods {
        assert_eq!(method.as_str(), label);
        assert_eq!(method.to_string(), label);
        assert!(method.badge_color().starts_with('#'));
    }
}

#[test]
fn test_param_and_header_items() {
    let p = ParamItem::new("page", "1");
    assert!(p.enabled);
    assert_eq!(p.key, "page");
    assert_eq!(p.value, "1");

    let p_dis = ParamItem::disabled("debug", "true");
    assert!(!p_dis.enabled);

    let h = HeaderItem::new("Accept", "application/json");
    assert!(h.enabled);
    let h_dis = HeaderItem::disabled("X-Debug", "1");
    assert!(!h_dis.enabled);
}

#[test]
fn test_body_type_defaults_and_content() {
    assert_eq!(BodyType::default_for_type("none"), BodyType::None);
    let json = BodyType::default_for_type("json");
    assert_eq!(json.type_name(), "JSON");
    assert!(json.content_str().contains("\"key\""));

    let gql = BodyType::default_for_type("gql");
    assert_eq!(gql.type_name(), "GraphQL");
    assert!(gql.content_str().starts_with("query"));

    assert_eq!(
        BodyType::default_for_type("form_urlencoded").type_name(),
        "FormUrlEncoded"
    );
    assert_eq!(
        BodyType::default_for_type("multipart/form-data").type_name(),
        "Multipart"
    );
    assert_eq!(BodyType::default_for_type("unknown"), BodyType::None);
}

#[test]
fn test_auth_config_defaults_and_names() {
    let cases = [
        ("inherit", "Inherit"),
        ("none", "None"),
        ("bearer", "Bearer Token"),
        ("basic", "Basic Auth"),
        ("api_key", "API Key"),
        ("aws", "AWS SigV4"),
        ("oauth2", "OAuth 2.0"),
        ("digest", "Digest Auth"),
        ("ntlm", "NTLM"),
        ("oauth1", "OAuth 1.0"),
        ("hawk", "Hawk"),
        ("edgegrid", "Akamai EdgeGrid"),
        ("wsse", "WSSE"),
    ];
    for (input, expected_name) in cases {
        let cfg = AuthConfig::default_for_type(input);
        assert_eq!(cfg.type_name(), expected_name);
    }
    assert_eq!(AuthConfig::default_for_type("something_else"), AuthConfig::None);
}

#[test]
fn test_proxy_config_effective_url_and_bypass() {
    let mut proxy = ProxyConfig::default();
    assert_eq!(proxy.effective_url(), None);

    proxy.enabled = true;
    proxy.host = "proxy.local".to_string();
    proxy.port = 8888;
    proxy.protocol = ProxyProtocol::Socks5;
    assert_eq!(
        proxy.effective_url().as_deref(),
        Some("socks5://proxy.local:8888")
    );

    proxy.auth_enabled = true;
    proxy.username = "alice".to_string();
    proxy.password = "secret".to_string();
    assert_eq!(
        proxy.effective_url().as_deref(),
        Some("socks5://alice:secret@proxy.local:8888")
    );

    proxy.bypass_list = vec![
        "localhost".to_string(),
        "*.internal.corp".to_string(),
        ".svc.local".to_string(),
        "10.0.0.1:8080".to_string(),
    ];
    assert!(proxy.should_bypass("http://localhost:3000/api"));
    assert!(proxy.should_bypass("https://api.internal.corp/v1"));
    assert!(proxy.should_bypass("https://internal.corp/v1"));
    assert!(proxy.should_bypass("http://auth.svc.local/login"));
    assert!(proxy.should_bypass("http://10.0.0.1:8080/health"));
    assert!(!proxy.should_bypass("https://example.com/"));
    assert!(!proxy.should_bypass("not-a-valid-url"));
}

#[test]
fn test_aura_request_mutations() {
    let mut req = AuraRequest::default();
    assert_eq!(req.name, "New Request");
    assert_eq!(req.method, HttpMethod::GET);

    req.add_header("X-Test", "1");
    assert_eq!(req.toggle_header(0), Some(false));
    assert_eq!(req.toggle_header(99), None);
    assert!(req.remove_header(0).is_some());
    assert!(req.remove_header(0).is_none());

    req.add_param("q", "rust");
    assert_eq!(req.toggle_param(0), Some(false));
    assert_eq!(req.toggle_param(99), None);
    assert!(req.remove_param(0).is_some());
    assert!(req.remove_param(0).is_none());

    req.set_body(BodyType::Text("hello".to_string()));
    assert_eq!(req.body.content_str(), "hello");

    req.set_auth(AuthConfig::Bearer {
        token: "tok".to_string(),
    });
    assert_eq!(req.auth.type_name(), "Bearer Token");
}

#[test]
fn test_response_helpers_and_timing() {
    let timing = RequestTimingBreakdown {
        dns_lookup_ms: 5,
        tcp_connect_ms: 10,
        tls_handshake_ms: 20,
        ttfb_ms: 30,
        content_download_ms: 15,
        total_ms: 80,
    };
    let phases = timing.phases();
    assert_eq!(phases[0], ("DNS Lookup", 5));
    assert_eq!(phases[4], ("Download", 15));

    let mut resp = AuraResponse {
        status: 201,
        status_text: "Created".to_string(),
        duration_ms: 80,
        size_bytes: 512,
        headers: HashMap::new(),
        body: String::new(),
        timestamp: Utc::now(),
        assertions: Vec::new(),
        console_logs: Vec::new(),
        script_errors: Vec::new(),
        sse_events: Vec::new(),
        stream_messages: Vec::new(),
        timing: Some(timing),
        captured_cookies: Vec::new(),
        next_request: None,
        stop_chain: false,
        exported_variables: HashMap::new(),
    };
    assert_eq!(resp.formatted_size(), "512 B");
    assert_eq!(
        resp.status_pill_color(),
        aura_core::design::colors::STATUS_2XX
    );
    resp.size_bytes = 2048;
    assert_eq!(resp.formatted_size(), "2.00 KB");
    resp.size_bytes = 2 * 1024 * 1024;
    assert_eq!(resp.formatted_size(), "2.00 MB");
    resp.status = 404;
    assert_eq!(
        resp.status_pill_color(),
        aura_core::design::colors::STATUS_4XX
    );
}

#[test]
fn test_think_time_flow_control_and_weighted_index() {
    assert_eq!(ThinkTimeConfig::None.sample_duration(), None);
    assert_eq!(
        ThinkTimeConfig::Uniform {
            min_ms: 0,
            max_ms: 0
        }
        .sample_duration(),
        None
    );
    assert_eq!(
        ThinkTimeConfig::Uniform {
            min_ms: 25,
            max_ms: 25
        }
        .sample_duration(),
        Some(std::time::Duration::from_millis(25))
    );

    let flow = FlowControl {
        delay_ms: Some(10),
        jitter_min_ms: Some(5),
        jitter_max_ms: Some(5),
        ..Default::default()
    };
    assert_eq!(
        flow.calculate_pacing_delay(),
        Some(std::time::Duration::from_millis(15))
    );

    assert_eq!(pick_weighted_index(&[]), None);
    assert_eq!(pick_weighted_index(&[0, 0]), None);
    let idx = pick_weighted_index(&[0, 10, 0]);
    assert_eq!(idx, Some(1));
}

#[test]
fn test_aura_file_request_round_trip() {
    let mut req = AuraRequest::new("Get User", HttpMethod::POST, "https://api.example.com/users");
    req.protocol = RequestProtocol::Graphql;
    req.headers.push(HeaderItem::new("X-Active", "yes"));
    req.headers.push(HeaderItem::disabled("X-Muted", "no"));
    req.params.push(ParamItem::new("v", "1"));
    req.params.push(ParamItem::disabled("debug", "1"));
    req.variables.insert("env".to_string(), "prod".to_string());
    req.disabled_variables
        .insert("old".to_string(), "val".to_string());
    req.auth = AuthConfig::None;
    req.scripts.pre_request = "console.log('hi')".to_string();
    req.scripts.assertions.push(DeclarativeAssertion::new(
        "res.status",
        AssertOperator::Eq,
        "200",
    ));

    let file_req = AuraFileRequest::from(&req);
    assert_eq!(file_req.protocol, Some(RequestProtocol::Graphql));
    assert_eq!(file_req.headers.get("X-Active").map(String::as_str), Some("yes"));
    assert_eq!(file_req.headers.get("X-Muted~").map(String::as_str), Some("no"));
    assert_eq!(file_req.query_params.get("debug~").map(String::as_str), Some("1"));
    assert_eq!(file_req.variables.get("old~").map(String::as_str), Some("val"));
    assert_eq!(file_req.auth, Some(AuthConfig::None));

    let restored = AuraRequest::from(&file_req);
    assert_eq!(restored.name, "Get User");
    assert_eq!(restored.method, HttpMethod::POST);
    assert_eq!(restored.protocol, RequestProtocol::Graphql);
    assert_eq!(restored.headers.len(), 2);
    assert!(!restored.headers[1].enabled);
    assert_eq!(restored.disabled_variables.get("old").map(String::as_str), Some("val"));
    assert_eq!(restored.auth, AuthConfig::None);
    assert_eq!(restored.scripts.pre_request, "console.log('hi')");
    assert_eq!(restored.scripts.assertions.len(), 1);
}

#[test]
fn test_variable_value_workspace_and_execution_report() {
    let v_plain = VariableValue::new("abc");
    assert!(!v_plain.is_secret);
    assert!(v_plain.keyring_ref.is_none());

    let v_secret = VariableValue::secret("xyz");
    assert!(v_secret.is_secret);

    let v_kr = VariableValue::from_keyring("aura/token");
    assert!(v_kr.is_secret);
    assert_eq!(v_kr.keyring_ref.as_deref(), Some("aura/token"));

    let ws = Workspace::new("My Workspace", "/tmp/ws");
    assert_eq!(ws.name, "My Workspace");
    assert!(ws.collections.is_empty());

    let mut report = ExecutionReport {
        request_name: "Test".to_string(),
        method: "GET".to_string(),
        url: "https://example.com".to_string(),
        status_code: 200,
        duration_ms: 10,
        response_headers: Default::default(),
        response_body: String::new(),
        assertions: vec![AssertionResult {
            name: "ok".to_string(),
            passed: true,
            error_message: None,
        }],
        logs: Vec::new(),
        error: None,
    };
    assert!(report.all_passed());
    assert_eq!(report.exit_code(), 0);

    report.assertions.push(AssertionResult {
        name: "fail".to_string(),
        passed: false,
        error_message: Some("bad".to_string()),
    });
    assert!(!report.all_passed());
    assert_eq!(report.exit_code(), 1);

    report.error = Some("network".to_string());
    assert_eq!(report.exit_code(), 3);
}
