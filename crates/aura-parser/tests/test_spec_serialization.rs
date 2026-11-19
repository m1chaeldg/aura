use aura_core::model::{AuraRequest, AuthConfig, BodyType, HeaderItem, HttpMethod, ParamItem};
use aura_parser::{parse_request, request_to_spec};

#[test]
fn test_request_to_spec_basic_get() {
    let mut req = AuraRequest::new("Get Users", HttpMethod::GET, "https://postman-echo.com/get");
    req.headers.push(HeaderItem::new("Accept", "application/json"));
    req.params.push(ParamItem::new("limit", "10"));

    let spec = request_to_spec(&req);
    assert!(spec.contains("version = \"1.0\""));
    assert!(spec.contains("name = \"Get Users\""));
    assert!(spec.contains("method = \"GET\""));
    assert!(spec.contains("url = \"https://postman-echo.com/get\""));
    assert!(spec.contains("[headers]"));
    assert!(spec.contains("Accept = \"application/json\""));
    assert!(spec.contains("[params.query]"));
    assert!(spec.contains("limit = \"10\""));

    // Round-trip parse
    let parsed = parse_request(&spec).expect("Should parse generated spec");
    assert_eq!(parsed.name, "Get Users");
    assert_eq!(parsed.method, HttpMethod::GET);
    assert_eq!(parsed.url, "https://postman-echo.com/get");
    assert_eq!(parsed.headers.len(), 1);
    assert_eq!(parsed.headers[0].key, "Accept");
    assert_eq!(parsed.headers[0].value, "application/json");
    assert_eq!(parsed.params.len(), 1);
    assert_eq!(parsed.params[0].key, "limit");
    assert_eq!(parsed.params[0].value, "10");
}

#[test]
fn test_request_to_spec_post_json_with_scripts_and_auth() {
    let mut req = AuraRequest::new("Create Item", HttpMethod::POST, "https://postman-echo.com/post");
    req.auth = AuthConfig::Bearer {
        token: "secret-token-123".to_string(),
    };
    req.body = BodyType::Json("{\"name\": \"test item\", \"count\": 42}".to_string());
    req.scripts.pre_request = "console.log(\"pre\");".to_string();
    req.scripts.post_response = "aura.test(\"ok\", () => {});".to_string();

    let spec = request_to_spec(&req);
    assert!(spec.contains("[auth]"));
    assert!(spec.contains("type = \"bearer\""));
    assert!(spec.contains("token = \"secret-token-123\""));
    assert!(spec.contains("[body.json]"));
    assert!(spec.contains("```json"));
    assert!(spec.contains("\"name\": \"test item\""));
    assert!(spec.contains("[scripts.pre-request]"));
    assert!(spec.contains("```javascript"));
    assert!(spec.contains("console.log(\"pre\");"));
    assert!(spec.contains("[scripts.post-response]"));
    assert!(spec.contains("aura.test(\"ok\", () => {});"));

    // Round-trip parse
    let parsed = parse_request(&spec).expect("Should parse complex post spec");
    assert_eq!(parsed.name, "Create Item");
    assert_eq!(parsed.method, HttpMethod::POST);
    assert_eq!(
        parsed.auth,
        AuthConfig::Bearer {
            token: "secret-token-123".to_string()
        }
    );
    match &parsed.body {
        BodyType::Json(s) => assert!(s.contains("\"name\": \"test item\"")),
        other => panic!("Expected BodyType::Json, got {:?}", other),
    }
    assert_eq!(parsed.scripts.pre_request, "console.log(\"pre\");");
    assert_eq!(parsed.scripts.post_response, "aura.test(\"ok\", () => {});");
}

#[test]
fn test_request_to_spec_disabled_headers_and_params() {
    let mut req = AuraRequest::new("Check Tildes", HttpMethod::GET, "https://example.com");
    req.headers.push(HeaderItem::disabled("X-Disabled", "val"));
    req.params.push(ParamItem::disabled("inactive_param", "0"));

    let spec = request_to_spec(&req);
    assert!(spec.contains("X-Disabled~ = \"val\""));
    assert!(spec.contains("inactive_param~ = \"0\""));

    let parsed = parse_request(&spec).expect("Parse disabled keys");
    assert_eq!(parsed.headers.len(), 1);
    assert_eq!(parsed.headers[0].key, "X-Disabled");
    assert!(!parsed.headers[0].enabled);
    assert_eq!(parsed.params.len(), 1);
    assert_eq!(parsed.params[0].key, "inactive_param");
    assert!(!parsed.params[0].enabled);
}
