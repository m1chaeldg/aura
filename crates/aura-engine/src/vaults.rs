//! OS Keychain (Windows Credential Manager, macOS Keychain, Linux Secret Service)
//! and external secret vault integrations (HashiCorp Vault KV v1/v2, AWS Secrets Manager).

use anyhow::{anyhow, Result};
use aura_core::security::{resolve_aws_profile_credentials, sign_aws_sigv4, AwsSigV4Credentials};
use aura_core::templating::TemplateContext;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::Utc;
use std::time::Duration;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VaultConfig {
    #[serde(default)]
    pub vault_addr: String,
    #[serde(default)]
    pub vault_token: String,
    #[serde(default)]
    pub vault_namespace: Option<String>,
    #[serde(default)]
    pub aws_region: String,
    #[serde(default)]
    pub aws_profile: Option<String>,
}

// --- Windows Credential Manager FFI (advapi32.dll) ---

#[cfg(target_os = "windows")]
mod win_cred {
    use super::*;
    use std::ffi::c_void;
    use std::ptr;

    const CRED_TYPE_GENERIC: u32 = 1;
    const CRED_PERSIST_LOCAL_MACHINE: u32 = 2;

    #[repr(C)]
    struct FILETIME {
        dw_low_date_time: u32,
        dw_high_date_time: u32,
    }

    #[repr(C)]
    struct CREDENTIALW {
        flags: u32,
        cred_type: u32,
        target_name: *mut u16,
        comment: *mut u16,
        last_written: FILETIME,
        credential_blob_size: u32,
        credential_blob: *mut u8,
        persist: u32,
        attribute_count: u32,
        attributes: *mut c_void,
        target_alias: *mut u16,
        user_name: *mut u16,
    }

    #[link(name = "advapi32")]
    extern "system" {
        fn CredWriteW(credential: *const CREDENTIALW, flags: u32) -> i32;
        fn CredReadW(
            target_name: *const u16,
            cred_type: u32,
            flags: u32,
            credential: *mut *mut CREDENTIALW,
        ) -> i32;
        fn CredDeleteW(target_name: *const u16, cred_type: u32, flags: u32) -> i32;
        fn CredFree(buffer: *mut c_void);
    }

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn set(target: &str, username: &str, secret: &str) -> Result<()> {
        let mut target_w = to_wide(target);
        let mut user_w = to_wide(username);
        let mut blob = secret.as_bytes().to_vec();
        let cred = CREDENTIALW {
            flags: 0,
            cred_type: CRED_TYPE_GENERIC,
            target_name: target_w.as_mut_ptr(),
            comment: ptr::null_mut(),
            last_written: FILETIME {
                dw_low_date_time: 0,
                dw_high_date_time: 0,
            },
            credential_blob_size: blob.len() as u32,
            credential_blob: if blob.is_empty() {
                ptr::null_mut()
            } else {
                blob.as_mut_ptr()
            },
            persist: CRED_PERSIST_LOCAL_MACHINE,
            attribute_count: 0,
            attributes: ptr::null_mut(),
            target_alias: ptr::null_mut(),
            user_name: user_w.as_mut_ptr(),
        };
        let ok = unsafe { CredWriteW(&cred, 0) };
        if ok == 0 {
            return Err(anyhow!(
                "Windows CredWriteW failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }

    pub fn get(target: &str) -> Result<Option<String>> {
        let target_w = to_wide(target);
        let mut ptr_cred: *mut CREDENTIALW = ptr::null_mut();
        let ok = unsafe { CredReadW(target_w.as_ptr(), CRED_TYPE_GENERIC, 0, &mut ptr_cred) };
        if ok == 0 {
            let err = std::io::Error::last_os_error();
            // ERROR_NOT_FOUND = 1168
            if err.raw_os_error() == Some(1168) {
                return Ok(None);
            }
            return Err(anyhow!("Windows CredReadW failed: {err}"));
        }
        let secret = unsafe {
            let cred = &*ptr_cred;
            if cred.credential_blob.is_null() || cred.credential_blob_size == 0 {
                String::new()
            } else {
                let slice = std::slice::from_raw_parts(
                    cred.credential_blob,
                    cred.credential_blob_size as usize,
                );
                String::from_utf8_lossy(slice).to_string()
            }
        };
        unsafe { CredFree(ptr_cred as *mut c_void) };
        Ok(Some(secret))
    }

    pub fn delete(target: &str) -> Result<()> {
        let target_w = to_wide(target);
        let ok = unsafe { CredDeleteW(target_w.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if ok == 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(1168) {
                return Ok(());
            }
            return Err(anyhow!("Windows CredDeleteW failed: {err}"));
        }
        Ok(())
    }
}

pub fn keychain_target_name(service: &str, key: &str) -> String {
    let svc = if service.trim().is_empty() {
        "aura"
    } else {
        service.trim()
    };
    format!("Aura:{svc}:{key}")
}

/// Store a secret in the OS Keychain / Credential Manager.
pub fn keychain_set(service: &str, key: &str, secret: &str) -> Result<()> {
    let target = keychain_target_name(service, key);
    #[cfg(target_os = "windows")]
    {
        return win_cred::set(&target, key, secret);
    }
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("security")
            .args([
                "add-generic-password",
                "-U",
                "-s",
                &target,
                "-a",
                key,
                "-w",
                secret,
            ])
            .status()?;
        if !status.success() {
            return Err(anyhow!("macOS security add-generic-password failed"));
        }
        return Ok(());
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        use std::io::Write;
        let mut child = std::process::Command::new("secret-tool")
            .args(["store", "--label", &target, "service", &target, "account", key])
            .stdin(std::process::Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(secret.as_bytes())?;
        }
        let status = child.wait()?;
        if !status.success() {
            return Err(anyhow!("Linux secret-tool store failed"));
        }
        return Ok(());
    }
}

/// Retrieve a secret from the OS Keychain / Credential Manager.
pub fn keychain_get(service: &str, key: &str) -> Result<Option<String>> {
    let target = keychain_target_name(service, key);
    #[cfg(target_os = "windows")]
    {
        return win_cred::get(&target);
    }
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("security")
            .args(["find-generic-password", "-s", &target, "-a", key, "-w"])
            .output()?;
        if !output.status.success() {
            return Ok(None);
        }
        return Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim_end().to_string(),
        ));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let output = std::process::Command::new("secret-tool")
            .args(["lookup", "service", &target, "account", key])
            .output()?;
        if !output.status.success() {
            return Ok(None);
        }
        return Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim_end().to_string(),
        ));
    }
}

/// Delete a secret from the OS Keychain / Credential Manager.
pub fn keychain_delete(service: &str, key: &str) -> Result<()> {
    let target = keychain_target_name(service, key);
    #[cfg(target_os = "windows")]
    {
        return win_cred::delete(&target);
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("security")
            .args(["delete-generic-password", "-s", &target, "-a", key])
            .status()?;
        return Ok(());
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("secret-tool")
            .args(["clear", "service", &target, "account", key])
            .status()?;
        return Ok(());
    }
}

/// Fetch a secret value from HashiCorp Vault (KV v1 or KV v2).
pub async fn fetch_hashicorp_vault_secret(
    vault_addr: &str,
    vault_token: &str,
    namespace: Option<&str>,
    secret_path: &str,
    field: Option<&str>,
    verify_ssl: bool,
) -> Result<String> {
    let base = vault_addr.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err(anyhow!("HashiCorp Vault address is not configured"));
    }
    let clean_path = secret_path.trim().trim_start_matches('/');
    let url = format!("{base}/v1/{clean_path}");

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(!verify_ssl)
        .timeout(Duration::from_secs(15))
        .build()?;
    let mut req = client.get(&url).header("X-Vault-Token", vault_token);
    if let Some(ns) = namespace.map(str::trim).filter(|s| !s.is_empty()) {
        req = req.header("X-Vault-Namespace", ns);
    }

    let resp = req.send().await?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!(
            "HashiCorp Vault returned HTTP {} for '{}': {}",
            status.as_u16(),
            clean_path,
            body
        ));
    }

    let json: serde_json::Value = serde_json::from_str(&body)?;
    // Support both KV v2 (data.data) and KV v1 (data)
    let data_obj = json
        .get("data")
        .and_then(|d| d.get("data").or(Some(d)))
        .ok_or_else(|| anyhow!("Vault response missing 'data' field: {body}"))?;

    if let Some(f) = field.map(str::trim).filter(|s| !s.is_empty()) {
        let val = data_obj
            .get(f)
            .ok_or_else(|| anyhow!("Vault secret '{clean_path}' missing field '{f}'"))?;
        return Ok(match val {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        });
    }

    if let Some(map) = data_obj.as_object() {
        if let Some((_, v)) = map.iter().next() {
            return Ok(match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            });
        }
    }
    Ok(data_obj.to_string())
}

/// Fetch a secret from AWS Secrets Manager (`GetSecretValue`) signed with AWS SigV4.
pub async fn fetch_aws_secrets_manager(
    secret_id: &str,
    json_key: Option<&str>,
    region: &str,
    profile: Option<&str>,
    endpoint_override: Option<&str>,
    verify_ssl: bool,
) -> Result<String> {
    let eff_region = if region.trim().is_empty() {
        "us-east-1"
    } else {
        region.trim()
    };

    let creds = match resolve_aws_profile_credentials(
        profile.unwrap_or("default"),
        None,
        eff_region,
        "secretsmanager",
    ) {
        Ok(c) => c,
        Err(_) => {
            let access_key = std::env::var("AWS_ACCESS_KEY_ID").map_err(|_| {
                anyhow!("AWS credentials not found in profile or AWS_ACCESS_KEY_ID")
            })?;
            let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY").map_err(|_| {
                anyhow!("AWS secret key not found in profile or AWS_SECRET_ACCESS_KEY")
            })?;
            let session_token = std::env::var("AWS_SESSION_TOKEN").ok();
            AwsSigV4Credentials {
                access_key,
                secret_key,
                session_token,
                region: eff_region.to_string(),
                service: "secretsmanager".to_string(),
            }
        }
    };

    let endpoint = endpoint_override
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("https://secretsmanager.{eff_region}.amazonaws.com/"));

    let payload = serde_json::json!({ "SecretId": secret_id }).to_string();
    let mut headers = vec![
        ("Content-Type".to_string(), "application/x-amz-json-1.1".to_string()),
        ("X-Amz-Target".to_string(), "secretsmanager.GetSecretValue".to_string()),
    ];
    sign_aws_sigv4(
        "POST",
        &endpoint,
        &mut headers,
        payload.as_bytes(),
        &creds,
        Utc::now(),
    )?;

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(!verify_ssl)
        .timeout(Duration::from_secs(15))
        .build()?;
    let mut req = client.post(&endpoint).body(payload);
    for (k, v) in headers {
        req = req.header(k, v);
    }

    let resp = req.send().await?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!(
            "AWS Secrets Manager returned HTTP {} for '{}': {}",
            status.as_u16(),
            secret_id,
            body
        ));
    }

    let json: serde_json::Value = serde_json::from_str(&body)?;
    let secret_str = if let Some(s) = json.get("SecretString").and_then(|v| v.as_str()) {
        s.to_string()
    } else if let Some(b64) = json.get("SecretBinary").and_then(|v| v.as_str()) {
        let bytes = STANDARD.decode(b64)?;
        String::from_utf8_lossy(&bytes).to_string()
    } else {
        return Err(anyhow!(
            "AWS Secrets Manager response missing SecretString/SecretBinary"
        ));
    };

    if let Some(key) = json_key.map(str::trim).filter(|s| !s.is_empty()) {
        let secret_json: serde_json::Value = serde_json::from_str(&secret_str).map_err(|e| {
            anyhow!("AWS SecretString for '{secret_id}' is not valid JSON for key '{key}': {e}")
        })?;
        let val = secret_json
            .get(key)
            .ok_or_else(|| anyhow!("AWS secret '{secret_id}' missing key '{key}'"))?;
        return Ok(match val {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        });
    }

    Ok(secret_str)
}

/// Resolve a secret reference URI:
/// - `keyring://<service>/<key>` -> OS Keychain / Credential Manager
/// - `vault://<path>#<field>` -> HashiCorp Vault KV
/// - `aws-sm://<secret_id>#<json_key>` -> AWS Secrets Manager
pub async fn resolve_secret_reference(
    raw: &str,
    vault_cfg: &VaultConfig,
    verify_ssl: bool,
) -> Result<String> {
    if let Some(rest) = raw.strip_prefix("keyring://") {
        let (service, key) = rest
            .split_once('/')
            .unwrap_or(("aura", rest));
        return keychain_get(service, key)?
            .ok_or_else(|| anyhow!("Keychain entry not found for '{service}/{key}'"));
    }
    if let Some(rest) = raw.strip_prefix("vault://") {
        let (path, field) = match rest.split_once('#') {
            Some((p, f)) => (p, Some(f)),
            None => (rest, None),
        };
        return fetch_hashicorp_vault_secret(
            &vault_cfg.vault_addr,
            &vault_cfg.vault_token,
            vault_cfg.vault_namespace.as_deref(),
            path,
            field,
            verify_ssl,
        )
        .await;
    }
    if let Some(rest) = raw.strip_prefix("aws-sm://") {
        let (secret_id, key) = match rest.split_once('#') {
            Some((id, k)) => (id, Some(k)),
            None => (rest, None),
        };
        return fetch_aws_secrets_manager(
            secret_id,
            key,
            &vault_cfg.aws_region,
            vault_cfg.aws_profile.as_deref(),
            None,
            verify_ssl,
        )
        .await;
    }
    Ok(raw.to_string())
}

fn is_secret_uri(val: &str) -> bool {
    val.starts_with("keyring://") || val.starts_with("vault://") || val.starts_with("aws-sm://")
}

/// Resolve all secret URI references inside a `TemplateContext` in-place.
pub async fn resolve_template_secrets(
    ctx: &mut TemplateContext,
    vault_cfg: &VaultConfig,
    verify_ssl: bool,
) {
    for map in [
        &mut ctx.environment_vars,
        &mut ctx.collection_vars,
        &mut ctx.request_vars,
    ] {
        let keys: Vec<String> = map
            .iter()
            .filter(|(_, v)| is_secret_uri(v))
            .map(|(k, _)| k.clone())
            .collect();
        for key in keys {
            if let Some(raw) = map.get(&key).cloned() {
                match resolve_secret_reference(&raw, vault_cfg, verify_ssl).await {
                    Ok(resolved) => {
                        map.insert(key, resolved);
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "aura_engine::vaults",
                            variable = %key,
                            reference = %raw,
                            error = %e,
                            "Failed to resolve secret reference"
                        );
                    }
                }
            }
        }
    }
}
