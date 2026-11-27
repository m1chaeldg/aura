//! Real local HTTP/HTTPS MITM capture proxy server backed by ephemeral/exported ECDSA P-256 Root CA
//! and per-host dynamic TLS certificates, recording browser/mobile traffic into `InterceptorManager`.

use anyhow::{anyhow, Result};
use aura_core::interceptor::{CapturedTransaction, InterceptorManager};
use aura_core::model::HttpMethod;
use chrono::Utc;
use parking_lot::RwLock;
use ring::rand::SystemRandom;
use ring::signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_ASN1_SIGNING};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use tokio_util::sync::CancellationToken;

/// Handle to a running HTTP/HTTPS MITM capture proxy server.
#[derive(Clone, Debug)]
pub struct CaptureProxyHandle {
    pub listen_addr: String,
    pub ca_cert_pem: String,
    cancel: CancellationToken,
    manager: InterceptorManager,
}

impl CaptureProxyHandle {
    pub fn stop(&self) {
        self.cancel.cancel();
        self.manager
            .set_server_status(false, self.listen_addr.clone(), self.ca_cert_pem.clone());
    }
}

struct MitmAuthority {
    ca_cert_der: Vec<u8>,
    ca_cert_pem: String,
    ca_pkcs8: Vec<u8>,
    ca_key_pair: EcdsaKeyPair,
    rng: SystemRandom,
    leaf_cache: RwLock<HashMap<String, Arc<rustls::ServerConfig>>>,
}

impl MitmAuthority {
    fn new() -> Result<Self> {
        let rng = SystemRandom::new();
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng)
            .map_err(|_| anyhow!("Failed to generate Root CA ECDSA P-256 keypair"))?;
        let ca_pkcs8 = pkcs8.as_ref().to_vec();
        let ca_key_pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &ca_pkcs8, &rng)
            .map_err(|_| anyhow!("Failed to parse generated Root CA keypair"))?;

        let ca_cert_der = build_signed_x509_cert(
            "Aura Proxy Root CA",
            "Aura Proxy Root CA",
            None,
            ca_key_pair.public_key().as_ref(),
            &ca_key_pair,
            &rng,
            true,
            1,
        )?;
        let ca_cert_pem = wrap_pem("CERTIFICATE", &ca_cert_der);

        Ok(Self {
            ca_cert_der,
            ca_cert_pem,
            ca_pkcs8,
            ca_key_pair,
            rng,
            leaf_cache: RwLock::new(HashMap::new()),
        })
    }

    fn server_config_for_host(&self, host: &str) -> Result<Arc<rustls::ServerConfig>> {
        if let Some(existing) = self.leaf_cache.read().get(host) {
            return Ok(Arc::clone(existing));
        }

        let leaf_pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &self.rng)
            .map_err(|_| anyhow!("Failed to generate leaf ECDSA keypair"))?;
        let leaf_key_pair = EcdsaKeyPair::from_pkcs8(
            &ECDSA_P256_SHA256_ASN1_SIGNING,
            leaf_pkcs8.as_ref(),
            &self.rng,
        )
        .map_err(|_| anyhow!("Failed to parse leaf ECDSA keypair"))?;

        let serial = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| (d.as_nanos() & 0x7FFF_FFFF_FFFF_FFFF) as u64 | 2)
            .unwrap_or(2);
        let leaf_cert_der = build_signed_x509_cert(
            "Aura Proxy Root CA",
            host,
            Some(host),
            leaf_key_pair.public_key().as_ref(),
            &self.ca_key_pair,
            &self.rng,
            false,
            serial,
        )?;

        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let cert_chain = vec![
            CertificateDer::from(leaf_cert_der),
            CertificateDer::from(self.ca_cert_der.clone()),
        ];
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(leaf_pkcs8.as_ref().to_vec()));
        let cfg = rustls::ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|e| anyhow!("TLS server config error: {e}"))?
            .with_no_client_auth()
            .with_single_cert(cert_chain, key)
            .map_err(|e| anyhow!("TLS leaf cert error for '{host}': {e}"))?;
        let arc_cfg = Arc::new(cfg);
        self.leaf_cache
            .write()
            .insert(host.to_string(), Arc::clone(&arc_cfg));
        let _ = &self.ca_pkcs8;
        Ok(arc_cfg)
    }
}

/// Start a real HTTP/HTTPS MITM capture proxy server on `bind_addr` (e.g. `"127.0.0.1:8888"` or `"127.0.0.1:0"`).
pub async fn start_capture_proxy(
    bind_addr: &str,
    manager: InterceptorManager,
) -> Result<CaptureProxyHandle> {
    let authority = Arc::new(MitmAuthority::new()?);
    let listener = TcpListener::bind(bind_addr).await?;
    let actual_addr = listener.local_addr()?.to_string();
    let cancel = CancellationToken::new();
    let ca_pem = authority.ca_cert_pem.clone();
    manager.set_server_status(true, actual_addr.clone(), ca_pem.clone());

    let mgr_task = manager.clone();
    let cancel_task = cancel.clone();
    let auth_task = Arc::clone(&authority);

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel_task.cancelled() => break,
                res = listener.accept() => {
                    let Ok((stream, _)) = res else { break; };
                    let auth = Arc::clone(&auth_task);
                    let mgr = mgr_task.clone();
                    tokio::spawn(async move {
                        let _ = handle_proxy_client(stream, auth, mgr).await;
                    });
                }
            }
        }
    });

    Ok(CaptureProxyHandle {
        listen_addr: actual_addr,
        ca_cert_pem: ca_pem,
        cancel,
        manager,
    })
}

struct ParsedHttpMsg {
    start_line: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

async fn read_http_message<R: AsyncRead + Unpin>(reader: &mut BufReader<R>) -> Result<Option<ParsedHttpMsg>> {
    let mut start_line = String::new();
    let n = reader.read_line(&mut start_line).await?;
    if n == 0 {
        return Ok(None);
    }
    let start_line = start_line.trim_end_matches(['\r', '\n']).to_string();
    if start_line.is_empty() {
        return Ok(None);
    }

    let mut headers = Vec::new();
    let mut content_len: Option<usize> = None;
    let mut is_chunked = false;
    loop {
        let mut line = String::new();
        let ln = reader.read_line(&mut line).await?;
        if ln == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if let Some((k, v)) = trimmed.split_once(':') {
            let key = k.trim().to_string();
            let val = v.trim().to_string();
            if key.eq_ignore_ascii_case("content-length") {
                content_len = val.parse::<usize>().ok();
            } else if key.eq_ignore_ascii_case("transfer-encoding")
                && val.to_ascii_lowercase().contains("chunked")
            {
                is_chunked = true;
            }
            headers.push((key, val));
        }
    }

    let mut body = Vec::new();
    if let Some(len) = content_len {
        if len > 0 {
            body.resize(len, 0);
            reader.read_exact(&mut body).await?;
        }
    } else if is_chunked {
        loop {
            let mut size_line = String::new();
            if reader.read_line(&mut size_line).await? == 0 {
                break;
            }
            let hex_part = size_line
                .trim()
                .split(';')
                .next()
                .unwrap_or("0")
                .trim();
            let chunk_size = usize::from_str_radix(hex_part, 16).unwrap_or(0);
            if chunk_size == 0 {
                let mut trailer = String::new();
                let _ = reader.read_line(&mut trailer).await;
                break;
            }
            let mut chunk = vec![0u8; chunk_size];
            reader.read_exact(&mut chunk).await?;
            body.extend_from_slice(&chunk);
            let mut crlf = [0u8; 2];
            let _ = reader.read_exact(&mut crlf).await;
        }
    }

    Ok(Some(ParsedHttpMsg {
        start_line,
        headers,
        body,
    }))
}

async fn handle_proxy_client(
    stream: TcpStream,
    authority: Arc<MitmAuthority>,
    manager: InterceptorManager,
) -> Result<()> {
    let mut reader = BufReader::new(stream);
    let Some(first_msg) = read_http_message(&mut reader).await? else {
        return Ok(());
    };

    let parts: Vec<&str> = first_msg.start_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Ok(());
    }
    let method = parts[0];
    let target = parts[1];

    if method.eq_ignore_ascii_case("CONNECT") {
        let (host, port) = split_host_port(target, 443);
        let srv_cfg = authority.server_config_for_host(&host)?;
        let mut raw_client = reader.into_inner();
        raw_client
            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .await?;

        let acceptor = TlsAcceptor::from(srv_cfg);
        let tls_client = acceptor.accept(raw_client).await?;
        let mut tls_reader = BufReader::new(tls_client);
        let Some(inner_req) = read_http_message(&mut tls_reader).await? else {
            return Ok(());
        };
        let mut tls_client = tls_reader.into_inner();

        let ip_parts: Vec<&str> = inner_req.start_line.split_whitespace().collect();
        if ip_parts.len() < 2 {
            return Ok(());
        }
        let inner_method = ip_parts[0];
        let inner_path = ip_parts[1];
        let full_url = if inner_path.starts_with("http://") || inner_path.starts_with("https://") {
            inner_path.to_string()
        } else if (port == 443) || host.contains(':') {
            format!("https://{host}{inner_path}")
        } else {
            format!("https://{host}:{port}{inner_path}")
        };

        forward_and_capture(
            &mut tls_client,
            inner_method,
            &full_url,
            inner_req.headers,
            inner_req.body,
            true,
            &manager,
        )
        .await?;
    } else {
        let full_url = if target.starts_with("http://") || target.starts_with("https://") {
            target.to_string()
        } else {
            let host = first_msg
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("host"))
                .map(|(_, v)| v.as_str())
                .unwrap_or("localhost");
            format!("http://{host}{target}")
        };
        let mut raw_client = reader.into_inner();
        forward_and_capture(
            &mut raw_client,
            method,
            &full_url,
            first_msg.headers,
            first_msg.body,
            false,
            &manager,
        )
        .await?;
    }

    Ok(())
}

async fn forward_and_capture<C: AsyncRead + AsyncWrite + Unpin>(
    client_stream: &mut C,
    method_str: &str,
    original_url: &str,
    req_headers: Vec<(String, String)>,
    req_body: Vec<u8>,
    is_https: bool,
    manager: &InterceptorManager,
) -> Result<()> {
    let start = Instant::now();
    let mut header_map: HashMap<String, String> = req_headers.iter().cloned().collect();
    let rewritten_url = manager.apply_rewrites(original_url, &mut header_map);

    let parsed = url::Url::parse(&rewritten_url)?;
    let up_is_https = parsed.scheme() == "https";
    let up_host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("Missing host in '{rewritten_url}'"))?
        .to_string();
    let up_port = parsed
        .port_or_known_default()
        .unwrap_or(if up_is_https { 443 } else { 80 });
    let origin_path = match parsed.query() {
        Some(q) => format!("{}?{q}", parsed.path()),
        None => parsed.path().to_string(),
    };

    let mut ordered_headers: Vec<(String, String)> = header_map.into_iter().collect();
    ordered_headers.retain(|(k, _)| {
        !k.eq_ignore_ascii_case("proxy-connection") && !k.eq_ignore_ascii_case("connection")
    });

    let mut wire = format!("{method_str} {origin_path} HTTP/1.1\r\n");
    if !ordered_headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("host")) {
        let host_hdr = match parsed.port() {
            Some(p) => format!("{up_host}:{p}"),
            None => up_host.clone(),
        };
        wire.push_str(&format!("Host: {host_hdr}\r\n"));
    }
    for (k, v) in &ordered_headers {
        if k.eq_ignore_ascii_case("content-length") {
            continue;
        }
        wire.push_str(&format!("{k}: {v}\r\n"));
    }
    if !req_body.is_empty() {
        wire.push_str(&format!("Content-Length: {}\r\n", req_body.len()));
    }
    wire.push_str("Connection: close\r\n\r\n");

    let tcp = tokio::time::timeout(
        Duration::from_secs(15),
        TcpStream::connect((up_host.as_str(), up_port)),
    )
    .await??;

    let resp_msg = if up_is_https {
        // Upstream TLS allows self-signed loopback or standard endpoints cleanly in proxy mode
        let connector: TlsConnector = crate::tls::build_tls_connector(false, vec![b"http/1.1".to_vec()])?;
        let sni = ServerName::try_from(up_host.clone())
            .map_err(|_| anyhow!("Invalid TLS server name '{up_host}'"))?;
        let mut tls_up = connector.connect(sni, tcp).await?;
        tls_up.write_all(wire.as_bytes()).await?;
        tls_up.write_all(&req_body).await?;
        let mut r = BufReader::new(tls_up);
        read_http_response_until_close(&mut r).await?
    } else {
        let mut plain_up = tcp;
        plain_up.write_all(wire.as_bytes()).await?;
        plain_up.write_all(&req_body).await?;
        let mut r = BufReader::new(plain_up);
        read_http_response_until_close(&mut r).await?
    };

    let status_code = resp_msg
        .start_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(502);

    // Relay response cleanly back to the browser / client
    let mut resp_wire = format!("{}\r\n", resp_msg.start_line);
    for (k, v) in &resp_msg.headers {
        if k.eq_ignore_ascii_case("transfer-encoding")
            || k.eq_ignore_ascii_case("content-length")
            || k.eq_ignore_ascii_case("connection")
        {
            continue;
        }
        resp_wire.push_str(&format!("{k}: {v}\r\n"));
    }
    resp_wire.push_str(&format!("Content-Length: {}\r\nConnection: close\r\n\r\n", resp_msg.body.len()));
    client_stream.write_all(resp_wire.as_bytes()).await?;
    client_stream.write_all(&resp_msg.body).await?;

    let http_method = match method_str.to_ascii_uppercase().as_str() {
        "POST" => HttpMethod::POST,
        "PUT" => HttpMethod::PUT,
        "PATCH" => HttpMethod::PATCH,
        "DELETE" => HttpMethod::DELETE,
        "HEAD" => HttpMethod::HEAD,
        "OPTIONS" => HttpMethod::OPTIONS,
        _ => HttpMethod::GET,
    };

    manager.record_capture(CapturedTransaction {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: Utc::now(),
        method: http_method,
        url: rewritten_url,
        request_headers: ordered_headers,
        request_body: (!req_body.is_empty())
            .then(|| String::from_utf8_lossy(&req_body).to_string()),
        status: status_code,
        response_headers: resp_msg.headers,
        response_body: (!resp_msg.body.is_empty())
            .then(|| String::from_utf8_lossy(&resp_msg.body).to_string()),
        duration_ms: start.elapsed().as_millis() as u64,
        is_https,
    });

    Ok(())
}

async fn read_http_response_until_close<R: AsyncRead + Unpin>(
    reader: &mut BufReader<R>,
) -> Result<ParsedHttpMsg> {
    let mut start_line = String::new();
    if reader.read_line(&mut start_line).await? == 0 {
        return Err(anyhow!("Upstream closed connection before response status"));
    }
    let start_line = start_line.trim_end_matches(['\r', '\n']).to_string();

    let mut headers = Vec::new();
    let mut content_len: Option<usize> = None;
    let mut is_chunked = false;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).await?;
        if n == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if let Some((k, v)) = trimmed.split_once(':') {
            let key = k.trim().to_string();
            let val = v.trim().to_string();
            if key.eq_ignore_ascii_case("content-length") {
                content_len = val.parse::<usize>().ok();
            } else if key.eq_ignore_ascii_case("transfer-encoding")
                && val.to_ascii_lowercase().contains("chunked")
            {
                is_chunked = true;
            }
            headers.push((key, val));
        }
    }

    let mut body = Vec::new();
    if let Some(len) = content_len {
        if len > 0 {
            body.resize(len, 0);
            reader.read_exact(&mut body).await?;
        }
    } else if is_chunked {
        loop {
            let mut size_line = String::new();
            if reader.read_line(&mut size_line).await? == 0 {
                break;
            }
            let hex_part = size_line.trim().split(';').next().unwrap_or("0").trim();
            let chunk_size = usize::from_str_radix(hex_part, 16).unwrap_or(0);
            if chunk_size == 0 {
                break;
            }
            let mut chunk = vec![0u8; chunk_size];
            reader.read_exact(&mut chunk).await?;
            body.extend_from_slice(&chunk);
            let mut crlf = [0u8; 2];
            let _ = reader.read_exact(&mut crlf).await;
        }
    } else {
        reader.read_to_end(&mut body).await?;
    }

    Ok(ParsedHttpMsg {
        start_line,
        headers,
        body,
    })
}

fn split_host_port(authority: &str, default_port: u16) -> (String, u16) {
    if let Some((h, p)) = authority.rsplit_once(':') {
        if let Ok(port) = p.parse::<u16>() {
            return (
                h.trim_start_matches('[').trim_end_matches(']').to_string(),
                port,
            );
        }
    }
    (authority.to_string(), default_port)
}

// --- Pure-Rust DER X.509 v3 ECDSA P-256 Certificate Builder ---

fn wrap_pem(label: &str, der: &[u8]) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    let b64 = STANDARD.encode(der);
    let mut out = format!("-----BEGIN {label}-----\n");
    for chunk in b64.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(chunk).unwrap_or(""));
        out.push('\n');
    }
    out.push_str(&format!("-----END {label}-----\n"));
    out
}

fn der_len(len: usize) -> Vec<u8> {
    if len < 0x80 {
        vec![len as u8]
    } else if len <= 0xff {
        vec![0x81, len as u8]
    } else {
        vec![0x82, (len >> 8) as u8, (len & 0xff) as u8]
    }
}

fn der_tlv(tag: u8, val: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    out.extend_from_slice(&der_len(val.len()));
    out.extend_from_slice(val);
    out
}

fn der_seq(items: &[&[u8]]) -> Vec<u8> {
    let mut body = Vec::new();
    for item in items {
        body.extend_from_slice(item);
    }
    der_tlv(0x30, &body)
}

fn der_set(items: &[&[u8]]) -> Vec<u8> {
    let mut body = Vec::new();
    for item in items {
        body.extend_from_slice(item);
    }
    der_tlv(0x31, &body)
}

fn der_int_u64(v: u64) -> Vec<u8> {
    let raw = v.to_be_bytes();
    let mut start = 0;
    while start + 1 < raw.len() && raw[start] == 0 {
        start += 1;
    }
    let slice = &raw[start..];
    let mut val = Vec::new();
    if slice[0] & 0x80 != 0 {
        val.push(0);
    }
    val.extend_from_slice(slice);
    der_tlv(0x02, &val)
}

fn der_bit_string(bytes: &[u8]) -> Vec<u8> {
    let mut body = vec![0x00]; // 0 unused bits
    body.extend_from_slice(bytes);
    der_tlv(0x03, &body)
}

fn der_name_cn(cn: &str) -> Vec<u8> {
    let oid_cn = der_tlv(0x06, &[0x55, 0x04, 0x03]);
    let val_utf8 = der_tlv(0x0c, cn.as_bytes());
    let atv = der_seq(&[&oid_cn, &val_utf8]);
    let rdn = der_set(&[&atv]);
    der_seq(&[&rdn])
}

#[allow(clippy::too_many_arguments)]
fn build_signed_x509_cert(
    issuer_cn: &str,
    subject_cn: &str,
    san_host: Option<&str>,
    subject_pub_ec_point: &[u8],
    signing_key: &EcdsaKeyPair,
    rng: &SystemRandom,
    is_ca: bool,
    serial: u64,
) -> Result<Vec<u8>> {
    // Version [0] EXPLICIT INTEGER 2 (v3)
    let ver = der_tlv(0xa0, &der_int_u64(2));
    let ser = der_int_u64(serial);

    // SignatureAlgorithm: ecdsa-with-SHA256 (1.2.840.10045.4.3.2)
    let ecdsa_sha256_alg = der_seq(&[&der_tlv(
        0x06,
        &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02],
    )]);

    let issuer = der_name_cn(issuer_cn);
    let subject = der_name_cn(subject_cn);

    let not_before = der_tlv(0x17, b"240101000000Z");
    let not_after = der_tlv(0x17, b"350101000000Z");
    let validity = der_seq(&[&not_before, &not_after]);

    // SubjectPublicKeyInfo: id-ecPublicKey (1.2.840.10045.2.1) + prime256v1 (1.2.840.10045.3.1.7)
    let spki_alg = der_seq(&[
        &der_tlv(0x06, &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01]),
        &der_tlv(0x06, &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07]),
    ]);
    let spki = der_seq(&[&spki_alg, &der_bit_string(subject_pub_ec_point)]);

    // Extensions [3] EXPLICIT SEQUENCE
    let mut ext_list: Vec<Vec<u8>> = Vec::new();
    // BasicConstraints (2.5.29.19)
    let bc_val = if is_ca {
        der_seq(&[&der_tlv(0x01, &[0xff])])
    } else {
        der_seq(&[])
    };
    ext_list.push(der_seq(&[
        &der_tlv(0x06, &[0x55, 0x1d, 0x13]),
        &der_tlv(0x01, &[0xff]),
        &der_tlv(0x04, &bc_val),
    ]));

    if is_ca {
        // KeyUsage: digitalSignature | keyCertSign | cRLSign
        let ku_bits = der_tlv(0x03, &[0x01, 0x86]);
        ext_list.push(der_seq(&[
            &der_tlv(0x06, &[0x55, 0x1d, 0x0f]),
            &der_tlv(0x01, &[0xff]),
            &der_tlv(0x04, &ku_bits),
        ]));
    } else if let Some(host) = san_host {
        // SubjectAltName (2.5.29.17)
        let san_entry = if let Ok(ip) = host.parse::<IpAddr>() {
            match ip {
                IpAddr::V4(v4) => der_tlv(0x87, &v4.octets()),
                IpAddr::V6(v6) => der_tlv(0x87, &v6.octets()),
            }
        } else {
            der_tlv(0x82, host.as_bytes())
        };
        let san_seq = der_seq(&[&san_entry]);
        ext_list.push(der_seq(&[
            &der_tlv(0x06, &[0x55, 0x1d, 0x11]),
            &der_tlv(0x04, &san_seq),
        ]));
    }

    let ext_refs: Vec<&[u8]> = ext_list.iter().map(|v| v.as_slice()).collect();
    let exts_seq = der_seq(&ext_refs);
    let exts_explicit = der_tlv(0xa3, &exts_seq);

    let tbs = der_seq(&[
        &ver,
        &ser,
        &ecdsa_sha256_alg,
        &issuer,
        &validity,
        &subject,
        &spki,
        &exts_explicit,
    ]);

    let sig = signing_key
        .sign(rng, &tbs)
        .map_err(|_| anyhow!("Failed to sign X.509 certificate"))?;
    let sig_bits = der_bit_string(sig.as_ref());

    Ok(der_seq(&[&tbs, &ecdsa_sha256_alg, &sig_bits]))
}
