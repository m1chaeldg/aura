use aura_core::model::{AuthConfig, ClientCertificateConfig};
use aura_core::security::{apply_auth, apply_request_auth};
use aura_core::security_schemes::{
    build_digest_authorization, build_ntlm_type1_negotiate, build_ntlm_type3_authenticate,
    build_oauth2_authorize_url, generate_pkce_pair, parse_ntlm_type2_challenge, DigestChallenge,
};
use aura_engine::resolve_client_cert_for_url;

#[test]
fn test_oauth2_pkce_and_authorize_url() {
    let (verifier, challenge) = generate_pkce_pair();
    assert!(verifier.len() >= 43);
    assert_ne!(verifier, challenge);

    let url = build_oauth2_authorize_url(
        "https://auth.example.com/authorize",
        "client-123",
        "http://127.0.0.1:8765/callback",
        Some("read write"),
        "state_xyz",
        "code",
        Some(&challenge),
    )
    .expect("authorize URL");

    assert!(url.contains("client_id=client-123"));
    assert!(url.contains("code_challenge_method=S256"));
    assert!(url.contains(&format!("code_challenge={challenge}")));
    assert!(url.contains("state=state_xyz"));
}

#[test]
fn test_digest_challenge_parse_and_header() {
    let hdr = "Digest realm=\"testrealm@host.com\", qop=\"auth,auth-int\", nonce=\"dcd98b7102dd2f0e8b11d0f600bfb0c093\", opaque=\"5ccc069c403ebaf9f0171e9517f40e41\", algorithm=SHA-256";
    let challenge = DigestChallenge::parse(hdr).expect("parse digest challenge");
    assert_eq!(challenge.realm, "testrealm@host.com");
    assert_eq!(challenge.algorithm, "SHA-256");

    let auth = build_digest_authorization(
        "GET",
        "/dir/index.html",
        "Mufasa",
        "Circle Of Life",
        &challenge,
    );
    assert!(auth.starts_with("Digest "));
    assert!(auth.contains("username=\"Mufasa\""));
    assert!(auth.contains("realm=\"testrealm@host.com\""));
    assert!(auth.contains("algorithm=SHA-256"));
    assert!(auth.contains("qop=auth"));
}

#[test]
fn test_ntlmv2_type1_type2_type3_handshake() {
    let t1 = build_ntlm_type1_negotiate();
    assert!(t1.starts_with("NTLM TlRMTVNTUA"));

    // Minimal valid Type 2 message with challenge (NTLMSSP\0 + type 2 + empty target + flags + 8-byte challenge + reserved)
    // "TlRMTVNTUAACAAAAAAAAAAAAAgAAAAECAwQFBgcIAAAAAAAAAAA=" -> let's test roundtrip via known Type 2 bytes encoded in hex->b64 or string:
    // Actually `TlRMTVNTUAACAAAAAAAAAAAAB4IBAgMEBQYHCA==` is 32 bytes? Let's construct valid base64 of 40 bytes:
    // Header "NTLMSSP\0" (8) + Type=2 (4) + Target(8: zeros) + Flags(4: 07 82 02 00) + Challenge(8: 01..08) + Reserved(8: zeros) = 40 bytes ->
    // Base64: "TlRMTVNTUAACAAAAAAAAAAAAB4ICAAECAwQFBgcIAAAAAAAAAAA="
    let t2_hdr = "NTLM TlRMTVNTUAACAAAAAAAAAAAAAAAHggIAAQIDBAUGBwgAAAAAAAAAAA==";
    let parsed = parse_ntlm_type2_challenge(t2_hdr).expect("parse type2");
    assert_eq!(parsed.server_challenge, [1, 2, 3, 4, 5, 6, 7, 8]);

    let t3 = build_ntlm_type3_authenticate("alice", "secret", "CORP", "WS1", &parsed)
        .expect("build type3");
    assert!(t3.starts_with("NTLM TlRMTVNTUAAD"));
}

#[test]
fn test_oauth1_hawk_edgegrid_wsse_signing() {
    let mut headers = Vec::new();
    let mut params = Vec::new();

    // OAuth 1.0 header + query placement
    let oauth1_hdr = AuthConfig::OAuth1 {
        consumer_key: "dpf43f3p2l4k3l03".into(),
        consumer_secret: "kd94hf93k423kf44".into(),
        token: "nnch734d00sl2jdk".into(),
        token_secret: "pfkkdhi9sl3r4s00".into(),
        signature_method: "HMAC-SHA256".into(),
        callback_url: None,
        realm: Some("Photos".into()),
        add_to_query: false,
    };
    apply_request_auth(
        "GET",
        "https://photos.example.net/photos?size=original",
        b"",
        &oauth1_hdr,
        &mut headers,
        &mut params,
    )
    .unwrap();
    assert!(headers
        .iter()
        .any(|(k, v)| k == "Authorization" && v.starts_with("OAuth ") && v.contains("oauth_signature_method=\"HMAC-SHA256\"")));

    let mut q_headers = Vec::new();
    let mut q_params = Vec::new();
    let oauth1_query = AuthConfig::OAuth1 {
        consumer_key: "ck".into(),
        consumer_secret: "cs".into(),
        token: String::new(),
        token_secret: String::new(),
        signature_method: "PLAINTEXT".into(),
        callback_url: None,
        realm: None,
        add_to_query: true,
    };
    apply_request_auth(
        "POST",
        "https://api.example.com/v1/items",
        b"",
        &oauth1_query,
        &mut q_headers,
        &mut q_params,
    )
    .unwrap();
    assert!(q_headers.is_empty());
    assert!(q_params.iter().any(|(k, _)| k == "oauth_signature"));

    // Hawk
    let mut hawk_headers = Vec::new();
    let hawk = AuthConfig::Hawk {
        id: "dh37fgj492je".into(),
        key: "werxhqb98rpaxn39848xrunpaw3489ruxnpa98w4rxn".into(),
        algorithm: "sha256".into(),
        ext: Some("some-app-ext-data".into()),
        app: None,
        dlg: None,
    };
    apply_request_auth(
        "GET",
        "https://example.com:8000/resource/1?b=1&a=2",
        b"",
        &hawk,
        &mut hawk_headers,
        &mut params,
    )
    .unwrap();
    assert!(hawk_headers[0].1.starts_with("Hawk id=\"dh37fgj492je\""));
    assert!(hawk_headers[0].1.contains("ext=\"some-app-ext-data\""));

    // Akamai EdgeGrid
    let mut eg_headers = Vec::new();
    let eg = AuthConfig::AkamaiEdgeGrid {
        client_token: "akab-client".into(),
        client_secret: "secret".into(),
        access_token: "akab-access".into(),
        max_body: 131072,
    };
    apply_request_auth(
        "POST",
        "https://akab-host.luna.akamaiapis.net/diagnostic-tools/v2/ghost-locations/available",
        br#"{"hello":"world"}"#,
        &eg,
        &mut eg_headers,
        &mut params,
    )
    .unwrap();
    assert!(eg_headers[0].1.starts_with("EG1-HMAC-SHA256 client_token=akab-client;"));
    assert!(eg_headers[0].1.contains("signature="));

    // WSSE
    let mut wsse_headers = Vec::new();
    let wsse = AuthConfig::Wsse {
        username: "wsse_user".into(),
        password: "wsse_password".into(),
        digest_password: true,
    };
    apply_auth(&wsse, &mut wsse_headers, &mut params);
    assert!(wsse_headers
        .iter()
        .any(|(k, v)| k == "Authorization" && v.contains("WSSE profile=\"UsernameToken\"")));
    assert!(wsse_headers
        .iter()
        .any(|(k, v)| k == "X-WSSE" && v.contains("Username=\"wsse_user\"") && v.contains("PasswordDigest=")));
}

#[test]
fn test_client_cert_domain_and_request_precedence() {
    let domain_certs = vec![
        ClientCertificateConfig {
            enabled: true,
            domain: Some("*.internal.example.com".into()),
            cert_path: "/tmp/wildcard.pem".into(),
            key_path: None,
            passphrase: None,
        },
        ClientCertificateConfig {
            enabled: false,
            domain: Some("disabled.example.com".into()),
            cert_path: "/tmp/disabled.pem".into(),
            key_path: None,
            passphrase: None,
        },
    ];
    let matched = resolve_client_cert_for_url(
        "https://api.internal.example.com/v1/users",
        None,
        &domain_certs,
    )
    .expect("wildcard cert match");
    assert_eq!(matched.cert_path, "/tmp/wildcard.pem");

    let req_override = ClientCertificateConfig {
        enabled: true,
        domain: None,
        cert_path: "/tmp/req_override.pfx".into(),
        key_path: None,
        passphrase: Some("p@ss".into()),
    };
    let overridden = resolve_client_cert_for_url(
        "https://api.internal.example.com/v1/users",
        Some(&req_override),
        &domain_certs,
    )
    .expect("request override");
    assert_eq!(overridden.cert_path, "/tmp/req_override.pfx");
}
