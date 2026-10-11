use aura::model::{AuraRequest, BodyType, HeaderItem, HttpMethod, ParamItem};
use aura::sync::{request_to_toml, toml_to_request};

#[test]
fn test_lossless_bidirectional_sync() {
    let mut req = AuraRequest::new(
        "User Profile",
        HttpMethod::POST,
        "https://api.example.com/users",
    );
    req.params.push(ParamItem {
        key: "active".to_string(),
        value: "true".to_string(),
        enabled: true,
        description: None,
    });
    // Disabled parameter must serialize with trailing ~
    req.params.push(ParamItem {
        key: "debug".to_string(),
        value: "1".to_string(),
        enabled: false,
        description: None,
    });

    req.headers.push(HeaderItem {
        key: "Content-Type".to_string(),
        value: "application/json".to_string(),
        enabled: true,
        description: None,
    });
    // Disabled header
    req.headers.push(HeaderItem {
        key: "X-Trace-Id".to_string(),
        value: "999".to_string(),
        enabled: false,
        description: None,
    });

    req.body = BodyType::Json("{\"name\": \"Alice\"}".to_string());
    req.scripts.pre_request = "console.log('pre');".to_string();
    req.scripts.tests = "aura.test('ok', () => {});".to_string();

    let toml_str = request_to_toml(&req).expect("Failed to serialize to TOML");

    assert!(toml_str.contains("\"debug~\" = \"1\"") || toml_str.contains("debug~ = \"1\""));
    assert!(
        toml_str.contains("\"X-Trace-Id~\" = \"999\"")
            || toml_str.contains("X-Trace-Id~ = \"999\"")
    );
    assert!(toml_str.contains("active = \"true\"") || toml_str.contains("\"active\" = \"true\""));

    let restored = toml_to_request(&toml_str, Some(&req.id)).expect("Failed to parse TOML");

    assert_eq!(restored.id, req.id);
    assert_eq!(restored.name, req.name);
    assert_eq!(restored.method, req.method);
    assert_eq!(restored.url, req.url);

    assert_eq!(restored.params.len(), 2);
    let active_p = restored.params.iter().find(|p| p.key == "active").unwrap();
    assert!(active_p.enabled);
    let debug_p = restored.params.iter().find(|p| p.key == "debug").unwrap();
    assert!(!debug_p.enabled);

    assert_eq!(restored.headers.len(), 2);
    let trace_h = restored
        .headers
        .iter()
        .find(|h| h.key == "X-Trace-Id")
        .unwrap();
    assert!(!trace_h.enabled);

    assert_eq!(restored.body, req.body);
    assert_eq!(restored.scripts.pre_request, req.scripts.pre_request);
    assert_eq!(restored.scripts.tests, req.scripts.tests);
}
