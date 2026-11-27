use aura_core::codegen::{generate_snippet, CodegenLanguage};
use aura_core::cookies::CookieJar;
use aura_core::interceptor::CapturedTransaction;
use aura_core::model::{
    AuthConfig, AuraRequest, BodyType, HttpMethod, ParamItem, ProxyConfig, ProxyProtocol,
    RequestTimingBreakdown,
};
use aura_core::templating::TemplateContext;
use std::collections::HashMap;

#[test]
fn test_cookie_jar_parse_scope_expiry_and_order() {
    let mut jar = CookieJar::new();
    let applied = jar.apply_set_cookie_headers(
        "https://api.example.com/v1/auth/login",
        &[
            "sid=root_tok; Domain=example.com; Path=/; HttpOnly; SameSite=Lax".to_string(),
            "api_tok=v1_tok; Path=/v1; Secure".to_string(),
            "host_only=exact; Path=/v1/auth".to_string(),
        ],
    );
    assert_eq!(applied.len(), 3);

    // Matching https://api.example.com/v1/auth/me receives all 3 sorted longest-path first
    let hdr = jar
        .cookie_header_for_url("https://api.example.com/v1/auth/me")
        .expect("cookies match");
    assert_eq!(hdr, "host_only=exact; api_tok=v1_tok; sid=root_tok");

    // Plain http skips Secure cookie (`api_tok`) and deeper path (`host_only`)
    let http_hdr = jar
        .cookie_header_for_url("http://sub.example.com/other")
        .expect("domain cookie matches subdomain");
    assert_eq!(http_hdr, "sid=root_tok");

    // Max-Age=0 deletes cookie
    jar.apply_set_cookie_headers(
        "https://api.example.com/v1/auth/logout",
        &["sid=deleted; Domain=example.com; Path=/; Max-Age=0".to_string()],
    );
    assert!(jar
        .cookie_header_for_url("http://sub.example.com/other")
        .is_none());

    jar.clear_domain("api.example.com");
    assert!(jar.cookies.is_empty());
}

#[test]
fn test_proxy_config_url_and_bypass_matching() {
    let proxy = ProxyConfig {
        enabled: true,
        protocol: ProxyProtocol::Socks5,
        host: "proxy.corp.local".into(),
        port: 1080,
        auth_enabled: true,
        username: "alice".into(),
        password: "secret".into(),
        bypass_list: vec![
            "localhost".into(),
            "127.0.0.1".into(),
            "*.internal.local".into(),
        ],
    };
    assert_eq!(
        proxy.effective_url().as_deref(),
        Some("socks5://alice:secret@proxy.corp.local:1080")
    );
    assert!(proxy.should_bypass("http://localhost:3000/test"));
    assert!(proxy.should_bypass("https://db.internal.local/status"));
    assert!(!proxy.should_bypass("https://api.github.com/users"));
}

#[test]
fn test_captured_transaction_to_request_and_timing_phases() {
    let tx = CapturedTransaction {
        id: "tx-1".into(),
        timestamp: serde_json::from_str("\"2026-01-01T00:00:00Z\"").unwrap(),
        method: HttpMethod::POST,
        url: "https://api.example.com/orders".into(),
        request_headers: vec![
            ("Host".into(), "api.example.com".into()),
            ("Content-Type".into(), "application/json".into()),
            ("X-Trace".into(), "abc".into()),
        ],
        request_body: Some(r#"{"item":"book"}"#.into()),
        status: 201,
        response_headers: vec![],
        response_body: Some(r#"{"id":1}"#.into()),
        duration_ms: 44,
        is_https: true,
    };
    let req = tx.to_aura_request();
    assert_eq!(req.method, HttpMethod::POST);
    assert_eq!(req.url, "https://api.example.com/orders");
    assert!(req.headers.iter().any(|h| h.key == "X-Trace"));
    assert!(matches!(req.body, BodyType::Json(_)));

    let timing = RequestTimingBreakdown {
        dns_lookup_ms: 5,
        tcp_connect_ms: 8,
        tls_handshake_ms: 12,
        ttfb_ms: 25,
        content_download_ms: 4,
        total_ms: 54,
    };
    assert_eq!(timing.phases().len(), 5);
}

#[test]
fn test_codegen_all_languages_with_templates_auth_and_body() {
    let mut req = AuraRequest::new(
        "Create Item",
        HttpMethod::POST,
        "https://{{host}}/api/items",
    );
    req.params.push(ParamItem::new("env", "{{stage}}"));
    req.add_header("X-Client", "AuraTest");
    req.auth = AuthConfig::Bearer {
        token: "{{api_token}}".into(),
    };
    req.body = BodyType::Json(r#"{"name":"widget"}"#.into());

    let mut ctx = TemplateContext::default();
    ctx.environment_vars = HashMap::from([
        ("host".into(), "example.com".into()),
        ("stage".into(), "prod".into()),
        ("api_token".into(), "tok_live_123".into()),
    ]);

    for lang in CodegenLanguage::ALL {
        let code = generate_snippet(&req, Some(&ctx), lang);
        assert!(
            code.contains("https://example.com/api/items?env=prod")
                || (lang == CodegenLanguage::HttpRaw && code.contains("POST /api/items?env=prod HTTP/1.1")),
            "Missing URL in {lang:?}: {code}"
        );
        assert!(
            code.contains("Bearer tok_live_123"),
            "Missing auth header in {lang:?}: {code}"
        );
        assert!(
            code.contains("widget"),
            "Missing body payload in {lang:?}: {code}"
        );
    }

    // Raw template mode keeps `{{host}}` unresolved when ctx is None
    let unresolved = generate_snippet(&req, None, CodegenLanguage::Curl);
    assert!(unresolved.contains("https://{{host}}/api/items"));
}
