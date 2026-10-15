use aura_core::model::{AuraResponse, BodyType, ConsoleMessage, TestAssertion};
use chrono::Utc;
use std::collections::HashMap;

#[test]
fn test_aura_response_with_console_logs_serialization() {
    let log = ConsoleMessage {
        level: "INFO".to_string(),
        message: "Generated user uuid".to_string(),
        timestamp: Utc::now(),
    };
    let response = AuraResponse {
        status: 200,
        status_text: "OK".to_string(),
        duration_ms: 42,
        size_bytes: 128,
        headers: HashMap::new(),
        body: "{\"status\":\"ok\"}".to_string(),
        timestamp: Utc::now(),
        assertions: vec![TestAssertion {
            name: "Status is 200".to_string(),
            passed: true,
            message: None,
            duration_ms: 1,
        }],
        console_logs: vec![log],
    };

    let serialized = serde_json::to_string(&response).expect("serialize response");
    let deserialized: AuraResponse = serde_json::from_str(&serialized).expect("deserialize response");
    assert_eq!(deserialized.status, 200);
    assert_eq!(deserialized.console_logs.len(), 1);
    assert_eq!(deserialized.console_logs[0].level, "INFO");
    assert_eq!(deserialized.console_logs[0].message, "Generated user uuid");
}

#[test]
fn test_body_type_type_name() {
    assert_eq!(BodyType::None.type_name(), "None");
    assert_eq!(BodyType::Json("{}".to_string()).type_name(), "JSON");
    assert_eq!(BodyType::Xml("<xml/>".to_string()).type_name(), "XML");
    assert_eq!(BodyType::Text("hi".to_string()).type_name(), "Text");
}
