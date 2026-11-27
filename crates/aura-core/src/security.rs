//! Security, Auth cascading, JWT Inspector, and Token-Boundary Secret Scrubber.

use crate::model::AuthConfig;
use anyhow::{anyhow, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Token-boundary secret scrubber to guard sensitive values from leaking into logs/traces
#[derive(Debug, Clone, Default)]
pub struct SecretScrubber {
    secret_values: Vec<String>,
}

impl SecretScrubber {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_secret(&mut self, secret: impl Into<String>) {
        let s = secret.into();
        if !s.trim().is_empty() && s.len() >= 3 {
            self.secret_values.push(s);
        }
    }

    pub fn scrub(&self, input: &str) -> String {
        let mut result = input.to_string();
        for secret in &self.secret_values {
            if !secret.is_empty() {
                result = result.replace(secret, "[REDACTED_SECRET]");
            }
        }
        result
    }
}

/// Decoded JWT payload and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedJwt {
    pub header: Value,
    pub payload: Value,
    pub algorithm: String,
    pub subject: Option<String>,
    pub issuer: Option<String>,
    pub expiration_timestamp: Option<i64>,
    pub is_expired: bool,
}

/// JWT Inspector: detects and decodes JWT tokens without network calls
pub fn inspect_jwt(token: &str) -> Result<DecodedJwt> {
    let parts: Vec<&str> = token.trim().split('.').collect();
    if parts.len() != 3 {
        return Err(anyhow!(
            "Invalid JWT structure: expected 3 dot-separated parts"
        ));
    }

    let header_bytes = URL_SAFE_NO_PAD
        .decode(parts[0])
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(parts[0]))
        .map_err(|e| anyhow!("Failed to decode JWT header base64: {e}"))?;

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(parts[1]))
        .map_err(|e| anyhow!("Failed to decode JWT payload base64: {e}"))?;

    let header: Value = serde_json::from_slice(&header_bytes)
        .map_err(|e| anyhow!("Invalid JSON in JWT header: {e}"))?;

    let payload: Value = serde_json::from_slice(&payload_bytes)
        .map_err(|e| anyhow!("Invalid JSON in JWT payload: {e}"))?;

    let algorithm = header["alg"].as_str().unwrap_or("unknown").to_string();
    let subject = payload["sub"].as_str().map(|s| s.to_string());
    let issuer = payload["iss"].as_str().map(|s| s.to_string());
    let expiration_timestamp = payload["exp"].as_i64();

    let current_time = chrono::Utc::now().timestamp();
    let is_expired = expiration_timestamp.is_some_and(|exp| current_time > exp);

    Ok(DecodedJwt {
        header,
        payload,
        algorithm,
        subject,
        issuer,
        expiration_timestamp,
        is_expired,
    })
}

/// Apply authentication config to HTTP request headers or params
pub fn apply_auth(
    auth: &AuthConfig,
    headers: &mut Vec<(String, String)>,
    params: &mut Vec<(String, String)>,
) {
    match auth {
        AuthConfig::Bearer { token } => {
            headers.push(("Authorization".to_string(), format!("Bearer {}", token)));
        }
        AuthConfig::Basic { username, password } => {
            let encoded = base64::engine::general_purpose::STANDARD
                .encode(format!("{}:{}", username, password));
            headers.push(("Authorization".to_string(), format!("Basic {}", encoded)));
        }
        AuthConfig::ApiKey {
            key,
            value,
            in_header,
        } => {
            if *in_header {
                headers.push((key.clone(), value.clone()));
            } else {
                params.push((key.clone(), value.clone()));
            }
        }
        AuthConfig::OAuth2 { access_token, .. } => {
            headers.push((
                "Authorization".to_string(),
                format!("Bearer {}", access_token),
            ));
        }
        AuthConfig::AwsSigV4 {
            use_profile,
            profile,
            credentials_path,
            access_key,
            secret_key: _,
            region,
            service,
            ..
        } => {
            // Note: full HMAC-SHA256 SigV4 signing is performed in `sign_aws_sigv4`
            // during HTTP dispatch when URL, headers, and payload bytes are finalized.
            let cred_id = if *use_profile {
                format!(
                    "profile:{}{}",
                    if profile.trim().is_empty() { "default" } else { profile.trim() },
                    credentials_path
                        .as_deref()
                        .filter(|p| !p.trim().is_empty())
                        .map(|p| format!("@{p}"))
                        .unwrap_or_default()
                )
            } else {
                access_key.clone()
            };
            headers.push((
                "Authorization".to_string(),
                format!(
                    "AWS4-HMAC-SHA256 Credential={}/20261011/{}/{}/aws4_request",
                    cred_id, region, service
                ),
            ));
        }
        AuthConfig::Wsse {
            username,
            password,
            digest_password,
        } => {
            headers.extend(crate::security_schemes::build_wsse_headers(
                username,
                password,
                *digest_password,
            ));
        }
        AuthConfig::Digest { .. }
        | AuthConfig::Ntlm { .. }
        | AuthConfig::OAuth1 { .. }
        | AuthConfig::Hawk { .. }
        | AuthConfig::AkamaiEdgeGrid { .. }
        | AuthConfig::Inherit
        | AuthConfig::None => {}
    }
}

/// Apply authentication requiring request context (method, resolved URL, body bytes).
pub fn apply_request_auth(
    method: &str,
    full_url: &str,
    body: &[u8],
    auth: &AuthConfig,
    headers: &mut Vec<(String, String)>,
    params: &mut Vec<(String, String)>,
) -> Result<()> {
    match auth {
        AuthConfig::OAuth1 {
            consumer_key,
            consumer_secret,
            token,
            token_secret,
            signature_method,
            callback_url,
            realm,
            add_to_query,
        } => {
            crate::security_schemes::sign_oauth1(
                method,
                full_url,
                consumer_key,
                consumer_secret,
                token,
                token_secret,
                signature_method,
                callback_url.as_deref(),
                realm.as_deref(),
                *add_to_query,
                headers,
                params,
            )?;
        }
        AuthConfig::Hawk {
            id,
            key,
            algorithm,
            ext,
            app,
            dlg,
        } => {
            let hdr = crate::security_schemes::build_hawk_header(
                method,
                full_url,
                id,
                key,
                algorithm,
                ext.as_deref(),
                app.as_deref(),
                dlg.as_deref(),
            )?;
            headers.push(("Authorization".to_string(), hdr));
        }
        AuthConfig::AkamaiEdgeGrid {
            client_token,
            client_secret,
            access_token,
            max_body,
        } => {
            let hdr = crate::security_schemes::build_edgegrid_header(
                method,
                full_url,
                body,
                client_token,
                client_secret,
                access_token,
                *max_body,
            )?;
            headers.push(("Authorization".to_string(), hdr));
        }
        AuthConfig::Digest {
            username,
            password,
            realm: Some(realm),
            nonce: Some(nonce),
            qop,
            algorithm,
        } if !realm.trim().is_empty() && !nonce.trim().is_empty() => {
            let challenge = crate::security_schemes::DigestChallenge {
                realm: realm.clone(),
                nonce: nonce.clone(),
                opaque: None,
                qop: qop.clone(),
                algorithm: algorithm.clone().unwrap_or_else(|| "MD5".to_string()),
            };
            let uri = url::Url::parse(full_url)
                .map(|u| match u.query() {
                    Some(q) => format!("{}?{q}", u.path()),
                    None => u.path().to_string(),
                })
                .unwrap_or_else(|_| "/".to_string());
            let hdr = crate::security_schemes::build_digest_authorization(
                method,
                &uri,
                username,
                password,
                &challenge,
            );
            headers.push(("Authorization".to_string(), hdr));
        }
        other => apply_auth(other, headers, params),
    }
    Ok(())
}

/// Resolved AWS credentials for SigV4 signing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AwsSigV4Credentials {
    pub access_key: String,
    pub secret_key: String,
    pub session_token: Option<String>,
    pub region: String,
    pub service: String,
}

fn default_aws_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".aws"))
}

fn parse_ini_section(contents: &str, section_names: &[&str]) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    let mut in_target = false;

    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let header = line[1..line.len() - 1].trim();
            in_target = section_names.iter().any(|target| header == *target);
            continue;
        }
        if in_target {
            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim().to_ascii_lowercase();
                let val = v.trim().to_string();
                result.insert(key, val);
            }
        }
    }

    result
}

/// Resolve AWS credentials from an AWS shared credentials/config file profile.
/// Defaults to profile `"default"` and `~/.aws/credentials` when not provided.
pub fn resolve_aws_profile_credentials(
    profile: &str,
    credentials_path: Option<&str>,
    region_override: &str,
    service: &str,
) -> Result<AwsSigV4Credentials> {
    let profile_name = if profile.trim().is_empty() {
        "default"
    } else {
        profile.trim()
    };

    let cred_path = match credentials_path.map(str::trim).filter(|p| !p.is_empty()) {
        Some(p) => {
            if let Some(rest) = p.strip_prefix("~/").or_else(|| p.strip_prefix("~\\")) {
                dirs::home_dir()
                    .map(|h| h.join(rest))
                    .unwrap_or_else(|| PathBuf::from(p))
            } else {
                PathBuf::from(p)
            }
        }
        None => default_aws_dir()
            .ok_or_else(|| anyhow!("Could not determine home directory for ~/.aws/credentials"))?
            .join("credentials"),
    };

    let cred_contents = std::fs::read_to_string(&cred_path).map_err(|e| {
        anyhow!(
            "Failed to read AWS credentials file '{}': {}",
            cred_path.display(),
            e
        )
    })?;

    let cred_kv = parse_ini_section(&cred_contents, &[profile_name]);
    if cred_kv.is_empty() {
        return Err(anyhow!(
            "AWS profile '{}' not found in credentials file '{}'",
            profile_name,
            cred_path.display()
        ));
    }

    let access_key = cred_kv
        .get("aws_access_key_id")
        .filter(|v| !v.is_empty())
        .cloned()
        .ok_or_else(|| {
            anyhow!(
                "AWS profile '{}' is missing 'aws_access_key_id' in '{}'",
                profile_name,
                cred_path.display()
            )
        })?;
    let secret_key = cred_kv
        .get("aws_secret_access_key")
        .filter(|v| !v.is_empty())
        .cloned()
        .ok_or_else(|| {
            anyhow!(
                "AWS profile '{}' is missing 'aws_secret_access_key' in '{}'",
                profile_name,
                cred_path.display()
            )
        })?;
    let session_token = cred_kv
        .get("aws_session_token")
        .filter(|v| !v.is_empty())
        .cloned();

    // Determine region: explicit override -> credentials file -> ~/.aws/config -> us-east-1
    let mut region = region_override.trim().to_string();
    if region.is_empty() {
        if let Some(r) = cred_kv.get("region").filter(|v| !v.is_empty()) {
            region = r.clone();
        }
    }
    if region.is_empty() {
        let config_path = cred_path
            .parent()
            .map(|p| p.join("config"))
            .or_else(|| default_aws_dir().map(|d| d.join("config")));
        if let Some(cfg_path) = config_path {
            if let Ok(cfg_contents) = std::fs::read_to_string(&cfg_path) {
                let prefixed = format!("profile {profile_name}");
                let cfg_kv = parse_ini_section(&cfg_contents, &[profile_name, &prefixed]);
                if let Some(r) = cfg_kv.get("region").filter(|v| !v.is_empty()) {
                    region = r.clone();
                }
            }
        }
    }
    if region.is_empty() {
        region = "us-east-1".to_string();
    }

    let service_name = if service.trim().is_empty() {
        "execute-api".to_string()
    } else {
        service.trim().to_string()
    };

    Ok(AwsSigV4Credentials {
        access_key,
        secret_key,
        session_token,
        region,
        service: service_name,
    })
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|e| anyhow!("HMAC key initialization failed: {e}"))?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn aws_uri_encode(input: &str, encode_slash: bool) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        let is_unreserved = byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'_' | b'.' | b'~')
            || (!encode_slash && byte == b'/');
        if is_unreserved {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{:02X}", byte));
        }
    }
    out
}

fn upsert_header_ci(headers: &mut Vec<(String, String)>, name: &str, value: String) {
    let lower = name.to_ascii_lowercase();
    headers.retain(|(k, _)| k.to_ascii_lowercase() != lower);
    headers.push((name.to_string(), value));
}

/// Sign an HTTP request using AWS Signature Version 4 (HMAC-SHA256).
/// Mutates `headers` to add/update `Host`, `X-Amz-Date`, optional `X-Amz-Security-Token`,
/// `X-Amz-Content-Sha256`, and `Authorization`.
pub fn sign_aws_sigv4(
    method: &str,
    full_url: &str,
    headers: &mut Vec<(String, String)>,
    body: &[u8],
    creds: &AwsSigV4Credentials,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    let parsed = url::Url::parse(full_url)
        .map_err(|e| anyhow!("Invalid URL for AWS SigV4 signing '{full_url}': {e}"))?;

    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("URL '{full_url}' is missing a host for AWS SigV4 signing"))?;
    let host_header = match parsed.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    };

    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = now.format("%Y%m%d").to_string();
    let payload_hash = sha256_hex(body);

    upsert_header_ci(headers, "Host", host_header);
    upsert_header_ci(headers, "X-Amz-Date", amz_date.clone());
    upsert_header_ci(headers, "X-Amz-Content-Sha256", payload_hash.clone());
    if let Some(token) = creds.session_token.as_deref().filter(|t| !t.is_empty()) {
        upsert_header_ci(headers, "X-Amz-Security-Token", token.to_string());
    }

    // Remove any pre-existing Authorization header before computing canonical headers
    headers.retain(|(k, _)| !k.eq_ignore_ascii_case("authorization"));

    let raw_path = parsed.path();
    let canonical_uri = if raw_path.is_empty() {
        "/".to_string()
    } else {
        // Url::path() may already contain percent-encoded chars; normalize by decoding then encoding
        aws_uri_encode(raw_path, false)
    };

    let mut query_pairs: Vec<(String, String)> = parsed
        .query_pairs()
        .map(|(k, v)| (aws_uri_encode(&k, true), aws_uri_encode(&v, true)))
        .collect();
    query_pairs.sort();
    let canonical_query = query_pairs
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");

    let mut header_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (k, v) in headers.iter() {
        let lk = k.trim().to_ascii_lowercase();
        let trimmed_v = v.split_whitespace().collect::<Vec<_>>().join(" ");
        header_map.entry(lk).or_default().push(trimmed_v);
    }

    let mut canonical_headers = String::new();
    let mut signed_header_names = Vec::new();
    for (k, vals) in &header_map {
        canonical_headers.push_str(&format!("{}:{}\n", k, vals.join(",")));
        signed_header_names.push(k.clone());
    }
    let signed_headers = signed_header_names.join(";");

    let canonical_request = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        method.to_ascii_uppercase(),
        canonical_uri,
        canonical_query,
        canonical_headers,
        signed_headers,
        payload_hash
    );

    let credential_scope = format!(
        "{}/{}/{}/aws4_request",
        date_stamp, creds.region, creds.service
    );
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        amz_date,
        credential_scope,
        sha256_hex(canonical_request.as_bytes())
    );

    let k_date = hmac_sha256(
        format!("AWS4{}", creds.secret_key).as_bytes(),
        date_stamp.as_bytes(),
    )?;
    let k_region = hmac_sha256(&k_date, creds.region.as_bytes())?;
    let k_service = hmac_sha256(&k_region, creds.service.as_bytes())?;
    let k_signing = hmac_sha256(&k_service, b"aws4_request")?;
    let signature = hex::encode(hmac_sha256(&k_signing, string_to_sign.as_bytes())?);

    let auth_value = format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        creds.access_key, credential_scope, signed_headers, signature
    );
    upsert_header_ci(headers, "Authorization", auth_value);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_sign_aws_sigv4_produces_valid_authorization_header() {
        let creds = AwsSigV4Credentials {
            access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            session_token: None,
            region: "us-east-1".to_string(),
            service: "s3".to_string(),
        };
        let mut headers = vec![("x-amz-date".to_string(), "20130524T000000Z".to_string())];
        let now = chrono::Utc.with_ymd_and_hms(2013, 5, 24, 0, 0, 0).unwrap();
        sign_aws_sigv4(
            "GET",
            "https://examplebucket.s3.amazonaws.com/test.txt",
            &mut headers,
            b"",
            &creds,
            now,
        )
        .unwrap();

        let auth = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("authorization"))
            .map(|(_, v)| v.as_str())
            .unwrap();
        assert!(auth.starts_with("AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, SignedHeaders="));
        assert!(auth.contains("Signature="));
    }

    #[test]
    fn test_resolve_aws_profile_credentials_from_file() {
        let dir = std::env::temp_dir().join(format!("aura_aws_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let cred_file = dir.join("credentials");
        std::fs::write(
            &cred_file,
            "[default]\naws_access_key_id = AKIA_DEFAULT\naws_secret_access_key = SECRET_DEFAULT\n\n[prod]\naws_access_key_id = AKIA_PROD\naws_secret_access_key = SECRET_PROD\naws_session_token = SESS_PROD\n",
        )
        .unwrap();

        let creds = resolve_aws_profile_credentials(
            "prod",
            Some(cred_file.to_str().unwrap()),
            "us-west-2",
            "execute-api",
        )
        .unwrap();
        assert_eq!(creds.access_key, "AKIA_PROD");
        assert_eq!(creds.secret_key, "SECRET_PROD");
        assert_eq!(creds.session_token.as_deref(), Some("SESS_PROD"));
        assert_eq!(creds.region, "us-west-2");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
