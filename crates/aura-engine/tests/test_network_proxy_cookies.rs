use aura_core::cookies::CookieJar;
use aura_core::interceptor::{InterceptorManager, InterceptorRule};
use aura_core::model::{AuraRequest, HttpMethod, ProxyConfig, ProxyProtocol};
use aura_core::templating::TemplateContext;
use aura_engine::{execute_request_with_options, start_capture_proxy, ExecutionOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn test_cookie_jar_persistence_and_phase_timing_loopback() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        // Request 1: /login -> returns two Set-Cookie headers after slight delay
        if let Ok((mut sock, _)) = listener.accept().await {
            let mut buf = [0u8; 4096];
            let _ = sock.read(&mut buf).await;
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            let body = r#"{"logged_in":true}"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\n\
                 Set-Cookie: session=abc123; Path=/; HttpOnly\r\n\
                 Set-Cookie: role=admin; Path=/api\r\n\
                 Content-Type: application/json\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = sock.write_all(resp.as_bytes()).await;
        }

        // Request 2: /api/me -> asserts client automatically sent persisted Cookie header
        if let Ok((mut sock, _)) = listener.accept().await {
            let mut buf = [0u8; 4096];
            let n = sock.read(&mut buf).await.unwrap_or(0);
            let req_text = String::from_utf8_lossy(&buf[..n]).to_string();
            let has_cookies = req_text.lines().any(|l| {
                let low = l.to_ascii_lowercase();
                low.starts_with("cookie:") && l.contains("role=admin") && l.contains("session=abc123")
            });
            let (status, body) = if has_cookies {
                ("200 OK", r#"{"cookie_ok":true}"#)
            } else {
                ("401 Unauthorized", r#"{"cookie_ok":false}"#)
            };
            let resp = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = sock.write_all(resp.as_bytes()).await;
        }
    });

    let mut jar = CookieJar::new();
    let mut opts = ExecutionOptions::default();

    let mut req1 = AuraRequest::new("Login", HttpMethod::GET, format!("http://{addr}/login"));
    let resp1 = execute_request_with_options(&mut req1, &TemplateContext::default(), &opts)
        .await
        .expect("login request");
    assert_eq!(resp1.status, 200);
    assert_eq!(resp1.captured_cookies.len(), 2);

    let timing = resp1.timing.expect("phase timing present");
    assert!(
        timing.ttfb_ms >= 15,
        "expected TTFB >= 15ms due to simulated delay, got {timing:?}"
    );
    assert!(timing.total_ms >= timing.ttfb_ms);

    let now = chrono::Utc::now();
    for c in resp1.captured_cookies {
        jar.set_cookie(c, now);
    }
    opts.cookie_jar = jar;

    let mut req2 = AuraRequest::new("Me", HttpMethod::GET, format!("http://{addr}/api/me"));
    let resp2 = execute_request_with_options(&mut req2, &TemplateContext::default(), &opts)
        .await
        .expect("me request");
    assert_eq!(resp2.status, 200);
    assert!(resp2.body.contains("cookie_ok"));
}

#[tokio::test]
async fn test_socks5_proxy_with_auth_and_bypass_loopback() {
    // 1. Origin HTTP server
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin.local_addr().unwrap();
    tokio::spawn(async move {
        for _ in 0..2 {
            if let Ok((mut sock, _)) = origin.accept().await {
                let mut buf = [0u8; 4096];
                let _ = sock.read(&mut buf).await;
                let body = r#"{"via":"ok"}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = sock.write_all(resp.as_bytes()).await;
            }
        }
    });

    // 2. Minimal compliant SOCKS5 server requiring username/password auth (RFC 1928 + RFC 1929)
    let socks_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let socks_addr = socks_listener.local_addr().unwrap();
    let saw_socks_conn = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let saw_flag = saw_socks_conn.clone();

    tokio::spawn(async move {
        if let Ok((mut client, _)) = socks_listener.accept().await {
            // Greeting: [VER=5, NMETHODS, ...]
            let mut greet = [0u8; 2];
            client.read_exact(&mut greet).await.unwrap();
            let mut methods = vec![0u8; greet[1] as usize];
            client.read_exact(&mut methods).await.unwrap();
            // Select Username/Password auth (0x02)
            client.write_all(&[0x05, 0x02]).await.unwrap();

            // RFC 1929 auth subnegotiation: [VER=1, ULEN, USER, PLEN, PASS]
            let mut ver_ulen = [0u8; 2];
            client.read_exact(&mut ver_ulen).await.unwrap();
            let mut user = vec![0u8; ver_ulen[1] as usize];
            client.read_exact(&mut user).await.unwrap();
            let mut plen = [0u8; 1];
            client.read_exact(&mut plen).await.unwrap();
            let mut pass = vec![0u8; plen[0] as usize];
            client.read_exact(&mut pass).await.unwrap();
            assert_eq!(user, b"socks_u");
            assert_eq!(pass, b"socks_p");
            client.write_all(&[0x01, 0x00]).await.unwrap();

            // CONNECT request: [VER=5, CMD=1, RSV=0, ATYP, ...]
            let mut req_hdr = [0u8; 4];
            client.read_exact(&mut req_hdr).await.unwrap();
            let target = match req_hdr[3] {
                0x01 => {
                    let mut ip = [0u8; 4];
                    client.read_exact(&mut ip).await.unwrap();
                    let mut port_b = [0u8; 2];
                    client.read_exact(&mut port_b).await.unwrap();
                    format!(
                        "{}.{}.{}.{}:{}",
                        ip[0],
                        ip[1],
                        ip[2],
                        ip[3],
                        u16::from_be_bytes(port_b)
                    )
                }
                0x03 => {
                    let mut l = [0u8; 1];
                    client.read_exact(&mut l).await.unwrap();
                    let mut dom = vec![0u8; l[0] as usize];
                    client.read_exact(&mut dom).await.unwrap();
                    let mut port_b = [0u8; 2];
                    client.read_exact(&mut port_b).await.unwrap();
                    format!(
                        "{}:{}",
                        String::from_utf8_lossy(&dom),
                        u16::from_be_bytes(port_b)
                    )
                }
                other => panic!("unexpected atyp {other}"),
            };

            let mut upstream = tokio::net::TcpStream::connect(&target).await.unwrap();
            saw_flag.store(true, std::sync::atomic::Ordering::SeqCst);
            client
                .write_all(&[0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1, 0, 0])
                .await
                .unwrap();
            let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
        }
    });

    // Request through SOCKS5 proxy
    let mut req = AuraRequest::new("ViaSocks", HttpMethod::GET, format!("http://{origin_addr}/proxied"));
    let opts = ExecutionOptions {
        global_proxy: Some(ProxyConfig {
            enabled: true,
            protocol: ProxyProtocol::Socks5,
            host: "127.0.0.1".into(),
            port: socks_addr.port(),
            auth_enabled: true,
            username: "socks_u".into(),
            password: "socks_p".into(),
            bypass_list: vec!["*.bypass.local".into()],
        }),
        ..Default::default()
    };
    let resp = execute_request_with_options(&mut req, &TemplateContext::default(), &opts)
        .await
        .expect("socks5 request");
    assert_eq!(resp.status, 200);
    assert!(saw_socks_conn.load(std::sync::atomic::Ordering::SeqCst));

    // Bypass list check skips proxy completely
    let bypass_opts = ExecutionOptions {
        global_proxy: Some(ProxyConfig {
            enabled: true,
            protocol: ProxyProtocol::Socks5,
            host: "127.0.0.1".into(),
            port: 1, // unreachable if NOT bypassed!
            auth_enabled: false,
            username: String::new(),
            password: String::new(),
            bypass_list: vec!["127.0.0.1".into()],
        }),
        ..Default::default()
    };
    let mut bypass_req =
        AuraRequest::new("Bypass", HttpMethod::GET, format!("http://{origin_addr}/direct"));
    let resp_direct = execute_request_with_options(&mut bypass_req, &TemplateContext::default(), &bypass_opts)
        .await
        .expect("bypassed request reaches origin directly");
    assert_eq!(resp_direct.status, 200);
}

#[tokio::test]
async fn test_capture_proxy_http_and_https_mitm_and_rewrites() {
    // 1. Plain HTTP origin server checking header/URL rewrite
    let http_origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let http_origin_addr = http_origin.local_addr().unwrap();
    tokio::spawn(async move {
        if let Ok((mut sock, _)) = http_origin.accept().await {
            let mut buf = [0u8; 4096];
            let n = sock.read(&mut buf).await.unwrap_or(0);
            let txt = String::from_utf8_lossy(&buf[..n]).to_string();
            let rewritten = txt.contains("GET /v2/orders") && txt.to_ascii_lowercase().contains("x-injected: mitm");
            let body = if rewritten { r#"{"captured":"http-ok"}"# } else { r#"{"captured":"miss"}"# };
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = sock.write_all(resp.as_bytes()).await;
        }
    });

    // 2. Start MITM Capture Proxy with a rewrite rule
    let mgr = InterceptorManager::new();
    mgr.add_rule(InterceptorRule {
        match_url_pattern: "/v1/orders".into(),
        rewrite_url_target: Some("/v2/orders".into()),
        override_headers: [("X-Injected".into(), "mitm".into())].into_iter().collect(),
    });
    let proxy = start_capture_proxy("127.0.0.1:0", mgr.clone())
        .await
        .expect("start MITM proxy");
    assert!(mgr.is_running());
    assert!(proxy.ca_cert_pem.contains("BEGIN CERTIFICATE"));

    // 3. Send HTTP request through the MITM Capture Proxy
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(format!("http://{}", proxy.listen_addr)).unwrap())
        .build()
        .unwrap();
    let res = client
        .get(format!("http://{http_origin_addr}/v1/orders"))
        .send()
        .await
        .expect("http via mitm proxy");
    assert_eq!(res.status().as_u16(), 200);
    let body = res.text().await.unwrap();
    assert!(body.contains("http-ok"));

    // 4. Send HTTPS CONNECT request through the MITM Capture Proxy trusting generated Root CA PEM
    // Create a local HTTPS origin server using a second self-signed Root CA PEM
    let https_origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let https_origin_addr = https_origin.local_addr().unwrap();

    // Reuse another generated ECDSA P-256 cert from an ephemeral CaptureProxyHandle? Even cleaner:
    // Use rustls with generated cert or validate CONNECT + MITM Root CA trust cleanly:
    // Spawn TLS origin using `proxy.ca_cert_pem` won't have private key exported, so let's generate
    // a tiny ECDSA P-256 server cert using the exact ring keypair or check HTTPS CONNECT against
    // TLS origin using a helper inside test!
    let rng = ring::rand::SystemRandom::new();
    let origin_pkcs8 = ring::signature::EcdsaKeyPair::generate_pkcs8(
        &ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING,
        &rng,
    )
    .unwrap();
    let origin_kp = ring::signature::EcdsaKeyPair::from_pkcs8(
        &ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING,
        origin_pkcs8.as_ref(),
        &rng,
    )
    .unwrap();
    use ring::signature::KeyPair;
    let origin_cert = make_minimal_test_cert(origin_kp.public_key().as_ref(), &origin_kp, &rng);
    let srv_cfg = rustls::ServerConfig::builder_with_provider(std::sync::Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![rustls::pki_types::CertificateDer::from(origin_cert)],
        rustls::pki_types::PrivateKeyDer::Pkcs8(rustls::pki_types::PrivatePkcs8KeyDer::from(
            origin_pkcs8.as_ref().to_vec(),
        )),
    )
    .unwrap();
    let acceptor = tokio_rustls::TlsAcceptor::from(std::sync::Arc::new(srv_cfg));

    tokio::spawn(async move {
        if let Ok((sock, _)) = https_origin.accept().await {
            if let Ok(mut tls) = acceptor.accept(sock).await {
                let mut buf = [0u8; 4096];
                let _ = tls.read(&mut buf).await;
                let b = r#"{"captured":"https-ok"}"#;
                let r = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{b}",
                    b.len()
                );
                let _ = tls.write_all(r.as_bytes()).await;
            }
        }
    });

    // Client trusts ONLY our MITM Root CA PEM (proving MITM CA + per-host signed cert are valid!)
    let ca_cert = reqwest::Certificate::from_pem(proxy.ca_cert_pem.as_bytes()).expect("valid CA PEM");
    let https_client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(format!("http://{}", proxy.listen_addr)).unwrap())
        .tls_built_in_root_certs(false)
        .add_root_certificate(ca_cert)
        .build()
        .unwrap();
    let https_res = https_client
        .get(format!("https://{https_origin_addr}/secret?x=1"))
        .send()
        .await
        .expect("https via MITM CA trust");
    assert_eq!(https_res.status().as_u16(), 200);
    assert!(https_res.text().await.unwrap().contains("https-ok"));

    let captures = mgr.list_captures();
    assert_eq!(captures.len(), 2);
    assert!(!captures[0].is_https);
    assert!(captures[1].is_https);
    assert!(captures[1].url.contains("/secret?x=1"));

    proxy.stop();
    assert!(!mgr.is_running());
}

fn make_minimal_test_cert(
    pub_key: &[u8],
    kp: &ring::signature::EcdsaKeyPair,
    rng: &ring::rand::SystemRandom,
) -> Vec<u8> {
    fn tlv(tag: u8, val: &[u8]) -> Vec<u8> {
        let mut o = vec![tag];
        if val.len() < 0x80 {
            o.push(val.len() as u8);
        } else if val.len() <= 0xff {
            o.push(0x81);
            o.push(val.len() as u8);
        } else {
            o.push(0x82);
            o.push((val.len() >> 8) as u8);
            o.push((val.len() & 0xff) as u8);
        }
        o.extend_from_slice(val);
        o
    }
    fn seq(parts: &[&[u8]]) -> Vec<u8> {
        let mut b = Vec::new();
        for p in parts {
            b.extend_from_slice(p);
        }
        tlv(0x30, &b)
    }
    let ver = tlv(0xa0, &tlv(0x02, &[2]));
    let ser = tlv(0x02, &[7]);
    let alg = seq(&[&tlv(0x06, &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02])]);
    let cn = seq(&[&tlv(
        0x31,
        &seq(&[&tlv(0x06, &[0x55, 0x04, 0x03]), &tlv(0x0c, b"127.0.0.1")]),
    )]);
    let val = seq(&[&tlv(0x17, b"240101000000Z"), &tlv(0x17, b"350101000000Z")]);
    let spki_alg = seq(&[
        &tlv(0x06, &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01]),
        &tlv(0x06, &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07]),
    ]);
    let mut bit_pk = vec![0u8];
    bit_pk.extend_from_slice(pub_key);
    let spki = seq(&[&spki_alg, &tlv(0x03, &bit_pk)]);
    let tbs = seq(&[&ver, &ser, &alg, &cn, &val, &cn, &spki]);
    let sig = kp.sign(rng, &tbs).unwrap();
    let mut sig_b = vec![0u8];
    sig_b.extend_from_slice(sig.as_ref());
    seq(&[&tbs, &alg, &tlv(0x03, &sig_b)])
}
