use aura_core::model::{AuraRequest, HttpMethod};
use aura_sync::{request_to_toml, toml_to_request};

#[test]
fn test_sync_smoke() {
    let req = AuraRequest::new("SyncTest", HttpMethod::POST, "https://api.example.com");
    let toml_str = request_to_toml(&req).expect("serialize to toml");
    let parsed = toml_to_request(&toml_str, None).expect("deserialize from toml");
    assert_eq!(parsed.name, "SyncTest");
}
