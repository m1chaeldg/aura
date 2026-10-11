use aura::security::{inspect_jwt, SecretScrubber};

#[test]
fn test_secret_scrubber() {
    let mut scrubber = SecretScrubber::new();
    scrubber.add_secret("super_secret_api_key_12345");
    scrubber.add_secret("password_abc");

    let log_line = "Sending request with key super_secret_api_key_12345 and password password_abc";
    let sanitized = scrubber.scrub(log_line);

    assert_eq!(
        sanitized,
        "Sending request with key [REDACTED_SECRET] and password [REDACTED_SECRET]"
    );
}

#[test]
fn test_jwt_inspector() {
    // Standard test JWT: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyLCJleHAiOjE5OTk5OTk5OTl9.signature
    let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyLCJleHAiOjE5OTk5OTk5OTl9.4peZ2Z9X6b_G58u9D9_example";
    let decoded = inspect_jwt(jwt).expect("Failed to inspect JWT");

    assert_eq!(decoded.algorithm, "HS256");
    assert_eq!(decoded.subject, Some("1234567890".to_string()));
    assert!(!decoded.is_expired);
}
