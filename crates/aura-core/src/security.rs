//! Security, Auth cascading, JWT Inspector, and Token-Boundary Secret Scrubber.

use crate::model::AuthConfig;
use anyhow::{anyhow, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
        AuthConfig::AwsV4 {
            access_key,
            secret_key: _,
            region,
            service,
            ..
        } => {
            // Basic AWS auth header placeholder
            headers.push((
                "Authorization".to_string(),
                format!(
                    "AWS4-HMAC-SHA256 Credential={}/20261011/{}/{}/aws4_request",
                    access_key, region, service
                ),
            ));
        }
        AuthConfig::Inherit | AuthConfig::None => {}
    }
}
