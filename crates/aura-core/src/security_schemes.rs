//! Pure-Rust authentication scheme builders and signers:
//! Digest Auth, NTLMv2, OAuth 1.0, Hawk, Akamai EdgeGrid, WSSE, and OAuth 2.0 PKCE helpers.

use anyhow::{anyhow, Result};
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use hmac::{Hmac, Mac};
use md5::{Digest as Md5Digest, Md5};
use sha1_smol::Sha1;
use sha2::Sha256;
use std::collections::BTreeMap;

fn random_hex(bytes_len: usize) -> String {
    let mut out = String::with_capacity(bytes_len * 2);
    while out.len() < bytes_len * 2 {
        out.push_str(&uuid::Uuid::new_v4().simple().to_string());
    }
    out.truncate(bytes_len * 2);
    out
}

fn rfc3986_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

fn hmac_sha256_bytes(key: &[u8], msg: &[u8]) -> Result<Vec<u8>> {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|e| anyhow!("HMAC-SHA256 init error: {e}"))?;
    mac.update(msg);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn hmac_sha1_bytes(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut k = [0u8; 64];
    if key.len() > 64 {
        let mut s = Sha1::new();
        s.update(key);
        let d = s.digest().bytes();
        k[..20].copy_from_slice(&d);
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for i in 0..64 {
        ipad[i] ^= k[i];
        opad[i] ^= k[i];
    }
    let mut inner = Sha1::new();
    inner.update(&ipad);
    inner.update(msg);
    let inner_digest = inner.digest().bytes();

    let mut outer = Sha1::new();
    outer.update(&opad);
    outer.update(&inner_digest);
    outer.digest().bytes().to_vec()
}

fn hmac_md5_bytes(key: &[u8], msg: &[u8]) -> Result<Vec<u8>> {
    type HmacMd5 = Hmac<Md5>;
    let mut mac =
        HmacMd5::new_from_slice(key).map_err(|e| anyhow!("HMAC-MD5 init error: {e}"))?;
    mac.update(msg);
    Ok(mac.finalize().into_bytes().to_vec())
}

// --- OAuth 2.0 PKCE + Authorization URL ---

pub fn generate_pkce_pair() -> (String, String) {
    let raw = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let verifier = URL_SAFE_NO_PAD.encode(raw.as_bytes());
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
    (verifier, challenge)
}

pub fn build_oauth2_authorize_url(
    auth_url: &str,
    client_id: &str,
    redirect_uri: &str,
    scope: Option<&str>,
    state: &str,
    response_type: &str,
    code_challenge: Option<&str>,
) -> Result<String> {
    let mut parsed = url::Url::parse(auth_url)
        .map_err(|e| anyhow!("Invalid OAuth2 authorization URL '{auth_url}': {e}"))?;
    {
        let mut qp = parsed.query_pairs_mut();
        qp.append_pair("response_type", response_type);
        qp.append_pair("client_id", client_id);
        qp.append_pair("redirect_uri", redirect_uri);
        qp.append_pair("state", state);
        if let Some(sc) = scope.map(str::trim).filter(|s| !s.is_empty()) {
            qp.append_pair("scope", sc);
        }
        if let Some(cc) = code_challenge {
            qp.append_pair("code_challenge", cc);
            qp.append_pair("code_challenge_method", "S256");
        }
    }
    Ok(parsed.to_string())
}

// --- HTTP Digest Auth (RFC 7616) ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestChallenge {
    pub realm: String,
    pub nonce: String,
    pub qop: Option<String>,
    pub opaque: Option<String>,
    pub algorithm: String,
}

impl DigestChallenge {
    pub fn parse(header: &str) -> Option<Self> {
        let stripped = header
            .trim()
            .strip_prefix("Digest ")
            .or_else(|| header.trim().strip_prefix("digest "))?;
        let mut map = BTreeMap::new();
        for part in split_quoted_kv(stripped) {
            if let Some((k, v)) = part.split_once('=') {
                map.insert(
                    k.trim().to_ascii_lowercase(),
                    v.trim().trim_matches('"').to_string(),
                );
            }
        }
        let realm = map.get("realm")?.clone();
        let nonce = map.get("nonce")?.clone();
        Some(Self {
            realm,
            nonce,
            qop: map.get("qop").cloned(),
            opaque: map.get("opaque").cloned(),
            algorithm: map
                .get("algorithm")
                .cloned()
                .unwrap_or_else(|| "MD5".to_string()),
        })
    }
}

fn split_quoted_kv(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut in_quotes = false;
    for (idx, ch) in input.char_indices() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                parts.push(&input[start..idx]);
                start = idx + 1;
            }
            _ => {}
        }
    }
    if start < input.len() {
        parts.push(&input[start..]);
    }
    parts
}

fn hash_for_digest(alg: &str, data: &str) -> String {
    if alg.to_ascii_uppercase().starts_with("SHA-256") {
        let mut h = Sha256::new();
        h.update(data.as_bytes());
        hex::encode(h.finalize())
    } else {
        let mut h = Md5::new();
        h.update(data.as_bytes());
        hex::encode(h.finalize())
    }
}

pub fn build_digest_authorization(
    method: &str,
    uri: &str,
    username: &str,
    password: &str,
    challenge: &DigestChallenge,
) -> String {
    let alg = challenge.algorithm.as_str();
    let cnonce = random_hex(8);
    let nc = "00000001";
    let qop_val = challenge
        .qop
        .as_deref()
        .and_then(|q| q.split(',').map(str::trim).find(|x| *x == "auth").or_else(|| q.split(',').next().map(str::trim)));

    let ha1_base = hash_for_digest(alg, &format!("{}:{}:{}", username, challenge.realm, password));
    let ha1 = if alg.to_ascii_lowercase().ends_with("-sess") {
        hash_for_digest(alg, &format!("{ha1_base}:{}:{cnonce}", challenge.nonce))
    } else {
        ha1_base
    };
    let ha2 = hash_for_digest(alg, &format!("{}:{uri}", method.to_ascii_uppercase()));

    let response = if let Some(q) = qop_val {
        hash_for_digest(
            alg,
            &format!("{ha1}:{}:{nc}:{cnonce}:{q}:{ha2}", challenge.nonce),
        )
    } else {
        hash_for_digest(alg, &format!("{ha1}:{}:{ha2}", challenge.nonce))
    };

    let mut fields = vec![
        format!("username=\"{username}\""),
        format!("realm=\"{}\"", challenge.realm),
        format!("nonce=\"{}\"", challenge.nonce),
        format!("uri=\"{uri}\""),
        format!("algorithm={}", challenge.algorithm),
        format!("response=\"{response}\""),
    ];
    if let Some(q) = qop_val {
        fields.push(format!("qop={q}"));
        fields.push(format!("nc={nc}"));
        fields.push(format!("cnonce=\"{cnonce}\""));
    }
    if let Some(op) = &challenge.opaque {
        fields.push(format!("opaque=\"{op}\""));
    }
    format!("Digest {}", fields.join(", "))
}

// --- NTLMv2 Challenge-Response ---

pub fn md4_digest(input: &[u8]) -> [u8; 16] {
    let mut msg = input.to_vec();
    let bit_len = (input.len() as u64) * 8;
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_le_bytes());

    let mut a: u32 = 0x67452301;
    let mut b: u32 = 0xefcdab89;
    let mut c: u32 = 0x98badcfe;
    let mut d: u32 = 0x10325476;

    for chunk in msg.chunks_exact(64) {
        let mut x = [0u32; 16];
        for (i, w) in x.iter_mut().enumerate() {
            *w = u32::from_le_bytes(chunk[i * 4..i * 4 + 4].try_into().unwrap());
        }
        let (aa, bb, cc, dd) = (a, b, c, d);
        let f = |x: u32, y: u32, z: u32| (x & y) | ((!x) & z);
        let g = |x: u32, y: u32, z: u32| (x & y) | (x & z) | (y & z);
        let h = |x: u32, y: u32, z: u32| x ^ y ^ z;

        let r1 = |a: u32, b: u32, c: u32, d: u32, k: usize, s: u32| {
            a.wrapping_add(f(b, c, d)).wrapping_add(x[k]).rotate_left(s)
        };
        let r2 = |a: u32, b: u32, c: u32, d: u32, k: usize, s: u32| {
            a.wrapping_add(g(b, c, d))
                .wrapping_add(x[k])
                .wrapping_add(0x5a827999)
                .rotate_left(s)
        };
        let r3 = |a: u32, b: u32, c: u32, d: u32, k: usize, s: u32| {
            a.wrapping_add(h(b, c, d))
                .wrapping_add(x[k])
                .wrapping_add(0x6ed9eba1)
                .rotate_left(s)
        };

        for idx in [0usize, 4, 8, 12] {
            a = r1(a, b, c, d, idx, 3);
            d = r1(d, a, b, c, idx + 1, 7);
            c = r1(c, d, a, b, idx + 2, 11);
            b = r1(b, c, d, a, idx + 3, 19);
        }
        for idx in 0..4usize {
            a = r2(a, b, c, d, idx, 3);
            d = r2(d, a, b, c, idx + 4, 5);
            c = r2(c, d, a, b, idx + 8, 9);
            b = r2(b, c, d, a, idx + 12, 13);
        }
        for k in [0usize, 2, 1, 3] {
            a = r3(a, b, c, d, k, 3);
            d = r3(d, a, b, c, k + 8, 9);
            c = r3(c, d, a, b, k + 4, 11);
            b = r3(b, c, d, a, k + 12, 15);
        }
        a = a.wrapping_add(aa);
        b = b.wrapping_add(bb);
        c = c.wrapping_add(cc);
        d = d.wrapping_add(dd);
    }

    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&a.to_le_bytes());
    out[4..8].copy_from_slice(&b.to_le_bytes());
    out[8..12].copy_from_slice(&c.to_le_bytes());
    out[12..16].copy_from_slice(&d.to_le_bytes());
    out
}

fn utf16le_bytes(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len() * 2);
    for u in s.encode_utf16() {
        out.extend_from_slice(&u.to_le_bytes());
    }
    out
}

#[derive(Debug, Clone)]
pub struct NtlmChallenge {
    pub server_challenge: [u8; 8],
    pub target_info: Vec<u8>,
}

pub fn build_ntlm_type1_negotiate() -> String {
    let mut msg = Vec::with_capacity(32);
    msg.extend_from_slice(b"NTLMSSP\0");
    msg.extend_from_slice(&1u32.to_le_bytes()); // Type 1
    let flags: u32 = 0x00000201 | 0x00080000 | 0x00008000; // Unicode | ExtendedSession | AlwaysSign
    msg.extend_from_slice(&flags.to_le_bytes());
    msg.extend_from_slice(&[0u8; 16]); // empty domain + workstation security buffers
    format!("NTLM {}", STANDARD.encode(msg))
}

pub fn parse_ntlm_type2_challenge(auth_header: &str) -> Result<NtlmChallenge> {
    let b64 = auth_header
        .trim()
        .strip_prefix("NTLM ")
        .ok_or_else(|| anyhow!("Expected NTLM Type 2 header"))?;
    let bytes = STANDARD.decode(b64.trim())?;
    if bytes.len() < 32 || &bytes[0..8] != b"NTLMSSP\0" {
        return Err(anyhow!("Malformed NTLM Type 2 challenge"));
    }
    let msg_type = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    if msg_type != 2 {
        return Err(anyhow!("Expected NTLM message type 2, got {msg_type}"));
    }
    let mut server_challenge = [0u8; 8];
    server_challenge.copy_from_slice(&bytes[24..32]);

    let target_info = if bytes.len() >= 48 {
        let len = u16::from_le_bytes(bytes[40..42].try_into().unwrap()) as usize;
        let offset = u32::from_le_bytes(bytes[44..48].try_into().unwrap()) as usize;
        if offset + len <= bytes.len() {
            bytes[offset..offset + len].to_vec()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    Ok(NtlmChallenge {
        server_challenge,
        target_info,
    })
}

pub fn build_ntlm_type3_authenticate(
    username: &str,
    password: &str,
    domain: &str,
    workstation: &str,
    challenge: &NtlmChallenge,
) -> Result<String> {
    let nt_hash = md4_digest(&utf16le_bytes(password));
    let identity = utf16le_bytes(&format!(
        "{}{}",
        username.to_uppercase(),
        domain
    ));
    let ntlmv2_key = hmac_md5_bytes(&nt_hash, &identity)?;

    let client_nonce = uuid::Uuid::new_v4().as_bytes()[..8].to_vec();
    let timestamp: u64 = ((chrono::Utc::now().timestamp() as u64) + 11_644_473_600) * 10_000_000;

    let mut blob = Vec::new();
    blob.extend_from_slice(&[0x01, 0x01, 0x00, 0x00]); // RespType, HiRespType, Reserved
    blob.extend_from_slice(&[0u8; 4]);
    blob.extend_from_slice(&timestamp.to_le_bytes());
    blob.extend_from_slice(&client_nonce);
    blob.extend_from_slice(&[0u8; 4]);
    blob.extend_from_slice(&challenge.target_info);
    blob.extend_from_slice(&[0u8; 4]);

    let mut proof_input = Vec::with_capacity(8 + blob.len());
    proof_input.extend_from_slice(&challenge.server_challenge);
    proof_input.extend_from_slice(&blob);
    let nt_proof = hmac_md5_bytes(&ntlmv2_key, &proof_input)?;

    let mut nt_response = Vec::with_capacity(nt_proof.len() + blob.len());
    nt_response.extend_from_slice(&nt_proof);
    nt_response.extend_from_slice(&blob);

    let dom_bytes = utf16le_bytes(domain);
    let user_bytes = utf16le_bytes(username);
    let ws_bytes = utf16le_bytes(workstation);
    let lm_response = vec![0u8; 24];

    let header_len = 64u32;
    let lm_off = header_len;
    let nt_off = lm_off + lm_response.len() as u32;
    let dom_off = nt_off + nt_response.len() as u32;
    let user_off = dom_off + dom_bytes.len() as u32;
    let ws_off = user_off + user_bytes.len() as u32;

    let write_sec = |buf: &mut Vec<u8>, len: usize, off: u32| {
        buf.extend_from_slice(&(len as u16).to_le_bytes());
        buf.extend_from_slice(&(len as u16).to_le_bytes());
        buf.extend_from_slice(&off.to_le_bytes());
    };

    let mut out = Vec::with_capacity(ws_off as usize + ws_bytes.len());
    out.extend_from_slice(b"NTLMSSP\0");
    out.extend_from_slice(&3u32.to_le_bytes());
    write_sec(&mut out, lm_response.len(), lm_off);
    write_sec(&mut out, nt_response.len(), nt_off);
    write_sec(&mut out, dom_bytes.len(), dom_off);
    write_sec(&mut out, user_bytes.len(), user_off);
    write_sec(&mut out, ws_bytes.len(), ws_off);
    write_sec(&mut out, 0, ws_off + ws_bytes.len() as u32);
    out.extend_from_slice(&0x00088201u32.to_le_bytes());
    out.extend_from_slice(&lm_response);
    out.extend_from_slice(&nt_response);
    out.extend_from_slice(&dom_bytes);
    out.extend_from_slice(&user_bytes);
    out.extend_from_slice(&ws_bytes);

    Ok(format!("NTLM {}", STANDARD.encode(out)))
}

// --- OAuth 1.0 (RFC 5849) ---

#[allow(clippy::too_many_arguments)]
pub fn sign_oauth1(
    method: &str,
    full_url: &str,
    consumer_key: &str,
    consumer_secret: &str,
    token: &str,
    token_secret: &str,
    signature_method: &str,
    callback_url: Option<&str>,
    realm: Option<&str>,
    add_to_query: bool,
    headers: &mut Vec<(String, String)>,
    params: &mut Vec<(String, String)>,
) -> Result<()> {
    let parsed = url::Url::parse(full_url)
        .map_err(|e| anyhow!("Invalid URL for OAuth 1.0 signing '{full_url}': {e}"))?;
    let ts = chrono::Utc::now().timestamp().to_string();
    let nonce = random_hex(8);
    let sig_method = if signature_method.trim().is_empty() {
        "HMAC-SHA1".to_string()
    } else {
        signature_method.trim().to_ascii_uppercase()
    };

    let mut oauth_params = vec![
        ("oauth_consumer_key".to_string(), consumer_key.to_string()),
        ("oauth_nonce".to_string(), nonce),
        ("oauth_signature_method".to_string(), sig_method.clone()),
        ("oauth_timestamp".to_string(), ts),
        ("oauth_version".to_string(), "1.0".to_string()),
    ];
    if !token.trim().is_empty() {
        oauth_params.push(("oauth_token".to_string(), token.to_string()));
    }
    if let Some(cb) = callback_url.map(str::trim).filter(|s| !s.is_empty()) {
        oauth_params.push(("oauth_callback".to_string(), cb.to_string()));
    }

    let mut all_sig_params: Vec<(String, String)> = parsed
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    all_sig_params.extend(params.iter().cloned());
    all_sig_params.extend(oauth_params.iter().cloned());

    let mut encoded_pairs: Vec<(String, String)> = all_sig_params
        .into_iter()
        .map(|(k, v)| (rfc3986_encode(&k), rfc3986_encode(&v)))
        .collect();
    encoded_pairs.sort();
    let param_str = encoded_pairs
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");

    let base_url = format!(
        "{}://{}{}",
        parsed.scheme(),
        parsed.host_str().unwrap_or(""),
        match parsed.port() {
            Some(p) => format!(":{p}{}", parsed.path()),
            None => parsed.path().to_string(),
        }
    );
    let base_string = format!(
        "{}&{}&{}",
        method.to_ascii_uppercase(),
        rfc3986_encode(&base_url),
        rfc3986_encode(&param_str)
    );
    let signing_key = format!(
        "{}&{}",
        rfc3986_encode(consumer_secret),
        rfc3986_encode(token_secret)
    );

    let signature = match sig_method.as_str() {
        "PLAINTEXT" => signing_key,
        "HMAC-SHA256" => {
            STANDARD.encode(hmac_sha256_bytes(signing_key.as_bytes(), base_string.as_bytes())?)
        }
        _ => STANDARD.encode(hmac_sha1_bytes(signing_key.as_bytes(), base_string.as_bytes())),
    };
    oauth_params.push(("oauth_signature".to_string(), signature));

    if add_to_query {
        params.extend(oauth_params);
    } else {
        let mut header_parts = Vec::new();
        if let Some(rlm) = realm.map(str::trim).filter(|s| !s.is_empty()) {
            header_parts.push(format!("realm=\"{}\"", rfc3986_encode(rlm)));
        }
        for (k, v) in oauth_params {
            header_parts.push(format!("{k}=\"{}\"", rfc3986_encode(&v)));
        }
        headers.push(("Authorization".to_string(), format!("OAuth {}", header_parts.join(", "))));
    }
    Ok(())
}

// --- Hawk Authentication ---

#[allow(clippy::too_many_arguments)]
pub fn build_hawk_header(
    method: &str,
    full_url: &str,
    id: &str,
    key: &str,
    algorithm: &str,
    ext: Option<&str>,
    app: Option<&str>,
    dlg: Option<&str>,
) -> Result<String> {
    let parsed = url::Url::parse(full_url)
        .map_err(|e| anyhow!("Invalid URL for Hawk signing '{full_url}': {e}"))?;
    let host = parsed.host_str().unwrap_or("");
    let port = parsed
        .port_or_known_default()
        .unwrap_or(if parsed.scheme() == "https" { 443 } else { 80 });
    let path_q = match parsed.query() {
        Some(q) => format!("{}?{q}", parsed.path()),
        None => parsed.path().to_string(),
    };
    let ts = chrono::Utc::now().timestamp().to_string();
    let nonce = random_hex(4);
    let ext_s = ext.unwrap_or("");

    let mut normalized = format!(
        "hawk.1.header\n{ts}\n{nonce}\n{}\n{path_q}\n{host}\n{port}\n\n{ext_s}\n",
        method.to_ascii_uppercase()
    );
    if let Some(a) = app.map(str::trim).filter(|s| !s.is_empty()) {
        normalized.push_str(a);
        normalized.push('\n');
        normalized.push_str(dlg.unwrap_or(""));
        normalized.push('\n');
    }

    let mac_bytes = if algorithm.eq_ignore_ascii_case("sha1") {
        hmac_sha1_bytes(key.as_bytes(), normalized.as_bytes())
    } else {
        hmac_sha256_bytes(key.as_bytes(), normalized.as_bytes())?
    };
    let mac = STANDARD.encode(mac_bytes);

    let mut parts = vec![
        format!("id=\"{id}\""),
        format!("ts=\"{ts}\""),
        format!("nonce=\"{nonce}\""),
        format!("mac=\"{mac}\""),
    ];
    if !ext_s.is_empty() {
        parts.push(format!("ext=\"{ext_s}\""));
    }
    if let Some(a) = app.map(str::trim).filter(|s| !s.is_empty()) {
        parts.push(format!("app=\"{a}\""));
    }
    if let Some(d) = dlg.map(str::trim).filter(|s| !s.is_empty()) {
        parts.push(format!("dlg=\"{d}\""));
    }
    Ok(format!("Hawk {}", parts.join(", ")))
}

// --- Akamai EdgeGrid (EG1-HMAC-SHA256) ---

pub fn build_edgegrid_header(
    method: &str,
    full_url: &str,
    body: &[u8],
    client_token: &str,
    client_secret: &str,
    access_token: &str,
    max_body: usize,
) -> Result<String> {
    let parsed = url::Url::parse(full_url)
        .map_err(|e| anyhow!("Invalid URL for EdgeGrid signing '{full_url}': {e}"))?;
    let timestamp = chrono::Utc::now().format("%Y%m%dT%H:%M:%S+0000").to_string();
    let nonce = uuid::Uuid::new_v4().to_string();

    let auth_prefix = format!(
        "EG1-HMAC-SHA256 client_token={client_token};access_token={access_token};timestamp={timestamp};nonce={nonce};"
    );

    let host = parsed.host_str().unwrap_or("");
    let path_q = match parsed.query() {
        Some(q) => format!("{}?{q}", parsed.path()),
        None => parsed.path().to_string(),
    };
    let content_hash = if method.eq_ignore_ascii_case("POST") && !body.is_empty() {
        let cap = if max_body == 0 { 131072 } else { max_body };
        let slice = &body[..body.len().min(cap)];
        let mut h = Sha256::new();
        h.update(slice);
        STANDARD.encode(h.finalize())
    } else {
        String::new()
    };

    let data_to_sign = format!(
        "{}\t{}\t{}\t{}\t\t{}\t{}",
        method.to_ascii_uppercase(),
        parsed.scheme(),
        host,
        path_q,
        content_hash,
        auth_prefix
    );
    let signing_key_bytes = hmac_sha256_bytes(client_secret.as_bytes(), timestamp.as_bytes())?;
    let signing_key_b64 = STANDARD.encode(signing_key_bytes);
    let signature = STANDARD.encode(hmac_sha256_bytes(
        signing_key_b64.as_bytes(),
        data_to_sign.as_bytes(),
    )?);
    Ok(format!("{auth_prefix}signature={signature}"))
}

// --- WSSE UsernameToken ---

pub fn build_wsse_headers(username: &str, password: &str, digest_password: bool) -> Vec<(String, String)> {
    let nonce_raw = uuid::Uuid::new_v4().as_bytes().to_vec();
    let nonce_b64 = STANDARD.encode(&nonce_raw);
    let created = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    let pass_val = if digest_password {
        let mut s = Sha1::new();
        s.update(&nonce_raw);
        s.update(created.as_bytes());
        s.update(password.as_bytes());
        STANDARD.encode(s.digest().bytes())
    } else {
        password.to_string()
    };

    let x_wsse = format!(
        "UsernameToken Username=\"{username}\", PasswordDigest=\"{pass_val}\", Nonce=\"{nonce_b64}\", Created=\"{created}\""
    );
    vec![
        (
            "Authorization".to_string(),
            "WSSE profile=\"UsernameToken\"".to_string(),
        ),
        ("X-WSSE".to_string(), x_wsse),
    ]
}
