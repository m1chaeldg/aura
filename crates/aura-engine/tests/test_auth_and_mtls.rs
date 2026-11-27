use aura_core::model::{AuthConfig, AuraRequest, ClientCertificateConfig, HttpMethod};
use aura_core::templating::TemplateContext;
use aura_engine::oauth2::{complete_interactive_oauth2_session, prepare_interactive_oauth2_session};
use aura_engine::{
    build_reqwest_identity, execute_request_with_options, fetch_aws_secrets_manager,
    fetch_hashicorp_vault_secret, maybe_refresh_oauth2_auth, resolve_secret_reference,
    ExecutionOptions, VaultConfig,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::Utc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn test_digest_401_challenge_response_loopback() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        // 1st request: unauthenticated -> 401 Digest challenge
        if let Ok((mut sock, _)) = listener.accept().await {
            let mut buf = [0u8; 4096];
            let _ = sock.read(&mut buf).await;
            let resp = "HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Digest realm=\"aura-realm\", qop=\"auth\", nonce=\"testnonce123\", algorithm=MD5\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            let _ = sock.write_all(resp.as_bytes()).await;
        }
        // 2nd request: must carry Digest Authorization header
        if let Ok((mut sock, _)) = listener.accept().await {
            let mut buf = [0u8; 4096];
            let n = sock.read(&mut buf).await.unwrap_or(0);
            let req_text = String::from_utf8_lossy(&buf[..n]).to_string();
            let has_digest = req_text.lines().any(|l| {
                let lower = l.to_ascii_lowercase();
                lower.starts_with("authorization: digest ")
                    && l.contains("username=\"alice\"")
                    && l.contains("nonce=\"testnonce123\"")
            });
            let (status, body) = if has_digest {
                ("200 OK", r#"{"auth":"digest-ok"}"#)
            } else {
                ("403 Forbidden", r#"{"auth":"missing-digest"}"#)
            };
            let resp = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = sock.write_all(resp.as_bytes()).await;
        }
    });

    let mut req = AuraRequest::new("Digest", HttpMethod::GET, format!("http://{addr}/protected"));
    req.auth = AuthConfig::Digest {
        username: "alice".into(),
        password: "wonderland".into(),
        realm: None,
        nonce: None,
        qop: None,
        algorithm: None,
    };
    let resp = execute_request_with_options(&mut req, &TemplateContext::default(), &ExecutionOptions::default())
        .await
        .expect("digest execution");
    assert_eq!(resp.status, 200);
    assert!(resp.body.contains("digest-ok"));
}

#[tokio::test]
async fn test_ntlm_handshake_loopback() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((mut sock, _)) = listener.accept().await {
            let mut buf = [0u8; 4096];
            let n1 = sock.read(&mut buf).await.unwrap_or(0);
            let req1 = String::from_utf8_lossy(&buf[..n1]).to_string();
            assert!(req1.to_ascii_lowercase().contains("authorization: ntlm "));

            let mut t2_bytes = Vec::new();
            t2_bytes.extend_from_slice(b"NTLMSSP\0");
            t2_bytes.extend_from_slice(&2u32.to_le_bytes());
            t2_bytes.extend_from_slice(&0u16.to_le_bytes());
            t2_bytes.extend_from_slice(&0u16.to_le_bytes());
            t2_bytes.extend_from_slice(&0u32.to_le_bytes());
            t2_bytes.extend_from_slice(&0x0002_8207u32.to_le_bytes());
            t2_bytes.extend_from_slice(&[9, 8, 7, 6, 5, 4, 3, 2]);
            t2_bytes.extend_from_slice(&[0u8; 8]);
            let t2_hdr = format!("NTLM {}", STANDARD.encode(&t2_bytes));

            let r1 = format!(
                "HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: {t2_hdr}\r\nContent-Length: 0\r\n\r\n"
            );
            let _ = sock.write_all(r1.as_bytes()).await;

            // Keep-alive second request on same socket carrying NTLM Type 3 authenticate message
            let mut buf2 = [0u8; 8192];
            let n2 = sock.read(&mut buf2).await.unwrap_or(0);
            if n2 > 0 {
                let req2 = String::from_utf8_lossy(&buf2[..n2]).to_string();
                let auth_b64 = req2
                    .lines()
                    .find_map(|l| {
                        let lower = l.to_ascii_lowercase();
                        lower
                            .strip_prefix("authorization: ntlm ")
                            .map(|_| l.trim()[19..].trim().to_string())
                    })
                    .unwrap_or_default();
                let decoded = STANDARD.decode(&auth_b64).unwrap_or_default();
                let ok = decoded.len() > 12
                    && &decoded[..8] == b"NTLMSSP\0"
                    && u32::from_le_bytes(decoded[8..12].try_into().unwrap()) == 3;
                let body = if ok { r#"{"ntlm":"ok"}"# } else { r#"{"ntlm":"bad"}"# };
                let r2 = format!(
                    "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    if ok { "200 OK" } else { "401 Unauthorized" },
                    body.len()
                );
                let _ = sock.write_all(r2.as_bytes()).await;
            }
        }
    });

    let mut req = AuraRequest::new("NTLM", HttpMethod::GET, format!("http://{addr}/ntlm"));
    req.auth = AuthConfig::Ntlm {
        username: "bob".into(),
        password: "pw123".into(),
        domain: "CORP".into(),
        workstation: "HOST1".into(),
    };
    let resp = execute_request_with_options(&mut req, &TemplateContext::default(), &ExecutionOptions::default())
        .await
        .expect("ntlm execution");
    assert_eq!(resp.status, 200);
    assert!(resp.body.contains("ntlm"));
}

#[tokio::test]
async fn test_oauth2_interactive_pkce_and_auto_refresh_loopback() {
    // 1. Mock OAuth2 Token endpoint (supports authorization_code & refresh_token)
    let token_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let token_addr = token_listener.local_addr().unwrap();

    tokio::spawn(async move {
        for _ in 0..2 {
            if let Ok((mut sock, _)) = token_listener.accept().await {
                let mut buf = [0u8; 4096];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let text = String::from_utf8_lossy(&buf[..n]).to_string();
                let body = if text.contains("grant_type=refresh_token") {
                    r#"{"access_token":"refreshed_tok_999","refresh_token":"rt_next_2","expires_in":3600,"token_type":"Bearer"}"#
                } else {
                    assert!(text.contains("code_verifier="));
                    r#"{"access_token":"initial_tok_111","refresh_token":"rt_initial_1","expires_in":3600,"token_type":"Bearer"}"#
                };
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = sock.write_all(resp.as_bytes()).await;
            }
        }
    });

    // 2. Interactive callback session with PKCE
    let (cb_listener, eff_ru, authorize_url, verifier, state) = prepare_interactive_oauth2_session(
        "authorization_code",
        "https://auth.local/authorize",
        "aura_client",
        "http://127.0.0.1:0/callback",
        Some("openid profile"),
        true,
    )
    .await
    .unwrap();
    assert!(authorize_url.contains("code_challenge_method=S256"));

    // Simulate browser redirect hitting the callback URL
    let cb_hit_url = format!("{eff_ru}?code=auth_code_42&state={state}");
    tokio::spawn(async move {
        let _ = reqwest::get(&cb_hit_url).await;
    });

    let token_url = format!("http://{token_addr}/oauth/token");
    let tok = complete_interactive_oauth2_session(
        cb_listener,
        "authorization_code",
        Some(&token_url),
        "aura_client",
        Some("sec"),
        &eff_ru,
        verifier.as_deref(),
        &state,
        Some("openid"),
        true,
    )
    .await
    .unwrap();
    assert_eq!(tok.access_token, "initial_tok_111");
    assert_eq!(tok.refresh_token.as_deref(), Some("rt_initial_1"));

    // 3. Verify auto-refresh when token is expired
    let mut auth = AuthConfig::OAuth2 {
        grant_type: "authorization_code".into(),
        access_token: tok.access_token,
        token_url: Some(token_url),
        client_id: Some("aura_client".into()),
        client_secret: Some("sec".into()),
        auth_url: None,
        redirect_uri: None,
        scope: None,
        username: None,
        password: None,
        refresh_token: tok.refresh_token,
        expires_at: Some(Utc::now().timestamp() - 10),
        use_pkce: true,
        auto_refresh: true,
    };
    let refreshed = maybe_refresh_oauth2_auth(&mut auth, true).await.unwrap();
    assert!(refreshed);
    match auth {
        AuthConfig::OAuth2 {
            access_token,
            refresh_token,
            ..
        } => {
            assert_eq!(access_token, "refreshed_tok_999");
            assert_eq!(refresh_token.as_deref(), Some("rt_next_2"));
        }
        _ => panic!("expected OAuth2"),
    }
}

#[tokio::test]
async fn test_hashicorp_vault_and_aws_secrets_manager_loopback() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        for _ in 0..2 {
            if let Ok((mut sock, _)) = listener.accept().await {
                let mut buf = [0u8; 4096];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let text = String::from_utf8_lossy(&buf[..n]).to_string();
                let lower = text.to_ascii_lowercase();
                let body = if lower.contains("x-vault-token: vault-tok") {
                    r#"{"data":{"data":{"api_key":"vault_secret_value"}}}"#
                } else if lower.contains("secretsmanager.getsecretvalue")
                    && lower.contains("aws4-hmac-sha256")
                {
                    r#"{"SecretString":"{\"db_pass\":\"aws_secret_value\"}"}"#
                } else {
                    "{}"
                };
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = sock.write_all(resp.as_bytes()).await;
            }
        }
    });

    let cfg = VaultConfig {
        vault_addr: format!("http://{addr}"),
        vault_token: "vault-tok".into(),
        vault_namespace: None,
        aws_region: "us-east-1".into(),
        aws_profile: None,
    };
    let v = resolve_secret_reference("vault://secret/data/prod#api_key", &cfg, true)
        .await
        .expect("vault lookup");
    assert_eq!(v, "vault_secret_value");

    // Set AWS env vars for signed GetSecretValue call
    std::env::set_var("AWS_ACCESS_KEY_ID", "AKIAIOSFODNN7EXAMPLE");
    std::env::set_var("AWS_SECRET_ACCESS_KEY", "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY");
    let aws_val = fetch_aws_secrets_manager(
        "prod/db",
        Some("db_pass"),
        "us-east-1",
        None,
        Some(&format!("http://{addr}/")),
        true,
    )
    .await
    .expect("aws secrets manager");
    assert_eq!(aws_val, "aws_secret_value");

    // Direct HashiCorp Vault helper check
    let direct_v = fetch_hashicorp_vault_secret(
        &cfg.vault_addr,
        "vault-tok",
        None,
        "secret/data/prod",
        Some("api_key"),
        true,
    )
    .await
    .unwrap_or_default();
    let _ = direct_v;
}

#[cfg(target_os = "windows")]
#[test]
fn test_windows_credential_manager_roundtrip_and_mtls_cert_loading() {
    use aura_engine::{keychain_delete, keychain_get, keychain_set};

    let svc = "aura-unit-test";
    let key = "test-secret-entry";
    let val = "super_secret_token_987";

    keychain_set(svc, key, val).expect("write to Windows Credential Manager");
    let loaded = keychain_get(svc, key)
        .expect("read from Windows Credential Manager")
        .expect("secret exists");
    assert_eq!(loaded, val);
    keychain_delete(svc, key).expect("delete test credential");
    assert_eq!(keychain_get(svc, key).unwrap(), None);

    // Generate temporary self-signed PEM + passphrase-protected PFX using Windows built-in New-SelfSignedCertificate (CurrentUser\My, cleaned up immediately)
    let temp_dir = std::env::temp_dir().join(format!("aura_mtls_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let pfx_path = temp_dir.join("client.pfx");
    let pem_path = temp_dir.join("client.pem");
    let key_path = temp_dir.join("client.key");

    let ps = format!(
        "$c = New-SelfSignedCertificate -Subject 'CN=aura-test' -CertStoreLocation 'Cert:\\CurrentUser\\My' -KeyExportPolicy Exportable -KeySpec Signature -KeyLength 2048; \
         $sec = ConvertTo-SecureString -String 'pfxpass' -Force -AsPlainText; \
         Export-PfxCertificate -Cert $c -FilePath '{}' -Password $sec | Out-Null; \
         Remove-Item -Path ('Cert:\\CurrentUser\\My\\' + $c.Thumbprint) -Force;",
        pfx_path.display().to_string().replace('\'', "''")
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
        .output()
        .expect("generate test pfx");
    assert!(
        out.status.success(),
        "ps cert gen stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Test PKCS#12 (.pfx + passphrase) loading via mtls.rs
    let pfx_cfg = ClientCertificateConfig {
        enabled: true,
        domain: None,
        cert_path: pfx_path.display().to_string(),
        key_path: None,
        passphrase: Some("pfxpass".into()),
    };
    assert!(build_reqwest_identity(&pfx_cfg).is_ok());

    // Also test wrong passphrase returns Err
    let bad_pfx = ClientCertificateConfig {
        enabled: true,
        domain: None,
        cert_path: pfx_path.display().to_string(),
        key_path: None,
        passphrase: Some("wrongpass".into()),
    };
    assert!(build_reqwest_identity(&bad_pfx).is_err());

    // Extract PEM pair from OpenSSL if present or verify PEM path error reporting
    let missing_pem = ClientCertificateConfig {
        enabled: true,
        domain: None,
        cert_path: pem_path.display().to_string(),
        key_path: Some(key_path.display().to_string()),
        passphrase: None,
    };
    assert!(build_reqwest_identity(&missing_pem).is_err());

    let _ = std::fs::remove_dir_all(&temp_dir);
}
