//! OS Keychain (Windows Credential Manager, macOS Keychain, Linux Secret Service)
//! and external secret vault integrations (HashiCorp Vault KV v1/v2, AWS Secrets Manager, KeePass .kdbx).

use anyhow::{anyhow, Result};
use aura_core::model::{AuthConfig, AuraRequest, BodyType};
use aura_core::security::{resolve_aws_profile_credentials, sign_aws_sigv4, AwsSigV4Credentials};
use aura_core::templating::{extract_template_keys, TemplateContext};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::Utc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fs::File;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

const VAULT_TOKEN_KEYCHAIN_SVC: &str = "aura-vault-token";
const KEEPASS_PW_KEYCHAIN_SVC: &str = "aura-keepass-password";

fn default_cache_ttl_secs() -> u64 {
    300
}

fn default_true() -> bool {
    true
}

/// Named HashiCorp Vault connection profile.
/// The Vault token itself is stored in the OS Keychain under `aura-vault-token/<id>`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HashiCorpVaultProfile {
    pub id: String,
    pub name: String,
    pub addr: String,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default = "default_true")]
    pub verify_ssl: bool,
}

/// Named AWS Secrets Manager connection profile.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AwsVaultProfile {
    pub id: String,
    pub name: String,
    pub region: String,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub endpoint_override: Option<String>,
}

/// Named KeePass (`.kdbx`) database profile.
/// The master password is stored in the OS Keychain under `aura-keepass-password/<id>`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KeePassProfile {
    pub id: String,
    pub name: String,
    pub db_path: String,
    #[serde(default)]
    pub keyfile_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    #[serde(default)]
    pub hashicorp_profiles: Vec<HashiCorpVaultProfile>,
    #[serde(default)]
    pub aws_profiles: Vec<AwsVaultProfile>,
    #[serde(default)]
    pub keepass_profiles: Vec<KeePassProfile>,
    #[serde(default = "default_cache_ttl_secs")]
    pub cache_ttl_secs: u64,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            vault_addr: String::new(),
            vault_token: String::new(),
            vault_namespace: None,
            aws_region: String::new(),
            aws_profile: None,
            hashicorp_profiles: Vec::new(),
            aws_profiles: Vec::new(),
            keepass_profiles: Vec::new(),
            cache_ttl_secs: default_cache_ttl_secs(),
        }
    }
}

pub fn set_vault_profile_token(profile_id: &str, token: &str) -> Result<()> {
    keychain_set(VAULT_TOKEN_KEYCHAIN_SVC, profile_id, token)
}

pub fn get_vault_profile_token(profile_id: &str) -> Result<Option<String>> {
    keychain_get(VAULT_TOKEN_KEYCHAIN_SVC, profile_id)
}

pub fn delete_vault_profile_token(profile_id: &str) -> Result<()> {
    keychain_delete(VAULT_TOKEN_KEYCHAIN_SVC, profile_id)
}

pub fn set_keepass_profile_password(profile_id: &str, password: &str) -> Result<()> {
    keychain_set(KEEPASS_PW_KEYCHAIN_SVC, profile_id, password)
}

pub fn get_keepass_profile_password(profile_id: &str) -> Result<Option<String>> {
    keychain_get(KEEPASS_PW_KEYCHAIN_SVC, profile_id)
}

pub fn delete_keepass_profile_password(profile_id: &str) -> Result<()> {
    keychain_delete(KEEPASS_PW_KEYCHAIN_SVC, profile_id)
}

// --- In-memory TTL Secret Cache ---

static SECRET_CACHE: LazyLock<RwLock<HashMap<String, (String, Instant)>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Clear all cached secret values from memory.
pub fn clear_secret_cache() {
    SECRET_CACHE.write().clear();
}

/// Returns the number of currently cached secret entries (including expired entries not yet evicted).
pub fn secret_cache_len() -> usize {
    SECRET_CACHE.read().len()
}

fn cache_get(key: &str, ttl_secs: u64) -> Option<String> {
    if ttl_secs == 0 {
        return None;
    }
    let guard = SECRET_CACHE.read();
    if let Some((val, inserted_at)) = guard.get(key) {
        if inserted_at.elapsed() < Duration::from_secs(ttl_secs) {
            return Some(val.clone());
        }
    }
    None
}

fn cache_put(key: String, val: String, ttl_secs: u64) {
    if ttl_secs == 0 {
        return;
    }
    SECRET_CACHE.write().insert(key, (val, Instant::now()));
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

/// Test connectivity & token validity against a HashiCorp Vault server.
pub async fn test_hashicorp_vault_connection(
    vault_addr: &str,
    vault_token: &str,
    namespace: Option<&str>,
    verify_ssl: bool,
) -> Result<String> {
    let base = vault_addr.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err(anyhow!("HashiCorp Vault address is required"));
    }
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(!verify_ssl)
        .timeout(Duration::from_secs(10))
        .build()?;
    let url = format!("{base}/v1/auth/token/lookup-self");
    let mut req = client.get(&url).header("X-Vault-Token", vault_token);
    if let Some(ns) = namespace.map(str::trim).filter(|s| !s.is_empty()) {
        req = req.header("X-Vault-Namespace", ns);
    }
    let resp = req.send().await?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Vault auth check failed (HTTP {}): {body}", status.as_u16()));
    }
    Ok("HashiCorp Vault connection & token verified".to_string())
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

/// Test AWS Secrets Manager credentials and region configuration.
pub fn test_aws_credentials(region: &str, profile: Option<&str>) -> Result<String> {
    let eff_region = if region.trim().is_empty() {
        "us-east-1"
    } else {
        region.trim()
    };
    if resolve_aws_profile_credentials(
        profile.unwrap_or("default"),
        None,
        eff_region,
        "secretsmanager",
    )
    .is_ok()
    {
        return Ok(format!(
            "AWS profile '{}' resolved for region {eff_region}",
            profile.unwrap_or("default")
        ));
    }
    if std::env::var("AWS_ACCESS_KEY_ID").is_ok() && std::env::var("AWS_SECRET_ACCESS_KEY").is_ok() {
        return Ok(format!(
            "AWS environment credentials resolved for region {eff_region}"
        ));
    }
    Err(anyhow!(
        "No AWS credentials found in profile '{}' or AWS_ACCESS_KEY_ID/AWS_SECRET_ACCESS_KEY",
        profile.unwrap_or("default")
    ))
}

fn find_keepass_entry_field(
    group: keepass::db::GroupRef<'_>,
    segments: &[&str],
    field_name: &str,
) -> Option<String> {
    if segments.is_empty() {
        return None;
    }
    if segments.len() == 1 {
        let target = segments[0];
        for entry in group.entries() {
            if entry.get_title().is_some_and(|t| t == target) {
                return entry.get(field_name).map(str::to_string);
            }
        }
        for sub in group.groups() {
            if let Some(found) = find_keepass_entry_field(sub, segments, field_name) {
                return Some(found);
            }
        }
        return None;
    }

    let (head, tail) = (segments[0], &segments[1..]);
    for sub in group.groups() {
        if sub.name == head {
            if let Some(found) = find_keepass_entry_field(sub, tail, field_name) {
                return Some(found);
            }
        }
    }
    None
}

fn open_keepass_db(
    db_path: &str,
    password: Option<&str>,
    keyfile_path: Option<&str>,
) -> Result<keepass::Database> {
    let mut file = File::open(db_path)
        .map_err(|e| anyhow!("Failed to open KeePass database '{db_path}': {e}"))?;
    let mut key = keepass::DatabaseKey::new();
    if let Some(pw) = password.filter(|s| !s.is_empty()) {
        key = key.with_password(pw);
    }
    if let Some(kf) = keyfile_path.map(str::trim).filter(|s| !s.is_empty()) {
        let mut kf_file = File::open(kf)
            .map_err(|e| anyhow!("Failed to open KeePass keyfile '{kf}': {e}"))?;
        key = key
            .with_keyfile(&mut kf_file)
            .map_err(|e| anyhow!("Invalid KeePass keyfile '{kf}': {e}"))?;
    }
    keepass::Database::open(&mut file, key)
        .map_err(|e| anyhow!("Failed to unlock KeePass database '{db_path}': {e}"))
}

/// Fetch a secret field from a KeePass (`.kdbx`) database.
/// `entry_path` can be `"EntryTitle"` or `"Group/Subgroup/EntryTitle"`.
/// `field` defaults to `"Password"` if `None` or empty.
pub async fn fetch_keepass_secret(
    db_path: &str,
    password: Option<&str>,
    keyfile_path: Option<&str>,
    entry_path: &str,
    field: Option<&str>,
) -> Result<String> {
    let db_path = db_path.trim().to_string();
    if db_path.is_empty() {
        return Err(anyhow!("KeePass database path is not configured"));
    }
    let password = password.map(str::to_string);
    let keyfile_path = keyfile_path.map(str::to_string);
    let entry_path = entry_path.trim().trim_matches('/').to_string();
    let field_name = field
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Password")
        .to_string();

    tokio::task::spawn_blocking(move || {
        let db = open_keepass_db(&db_path, password.as_deref(), keyfile_path.as_deref())?;
        let segments: Vec<&str> = entry_path
            .split('/')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        find_keepass_entry_field(db.root(), &segments, &field_name).ok_or_else(|| {
            anyhow!("KeePass entry '{entry_path}' or field '{field_name}' not found in '{db_path}'")
        })
    })
    .await
    .map_err(|e| anyhow!("KeePass worker task failed: {e}"))?
}

/// Test opening a KeePass `.kdbx` database with the provided credentials.
pub async fn test_keepass_connection(
    db_path: &str,
    password: Option<&str>,
    keyfile_path: Option<&str>,
) -> Result<String> {
    let db_path = db_path.trim().to_string();
    if db_path.is_empty() {
        return Err(anyhow!("KeePass database path is required"));
    }
    let password = password.map(str::to_string);
    let keyfile_path = keyfile_path.map(str::to_string);
    tokio::task::spawn_blocking(move || {
        let db = open_keepass_db(&db_path, password.as_deref(), keyfile_path.as_deref())?;
        Ok(format!(
            "Unlocked KeePass database (root group: '{}')",
            db.root().name
        ))
    })
    .await
    .map_err(|e| anyhow!("KeePass worker task failed: {e}"))?
}

fn split_profile_prefix(rest: &str) -> (Option<&str>, &str) {
    if let Some((prof, target)) = rest.split_once('@') {
        if !prof.trim().is_empty() && !prof.contains('/') && !prof.contains('#') {
            return (Some(prof.trim()), target);
        }
    }
    (None, rest)
}

fn resolve_hashicorp_profile<'a>(
    vault_cfg: &'a VaultConfig,
    profile_name: Option<&str>,
    fallback_verify_ssl: bool,
) -> Result<(String, String, Option<String>, bool)> {
    if let Some(name) = profile_name {
        if let Some(p) = vault_cfg
            .hashicorp_profiles
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name) || p.id == name)
        {
            let token = get_vault_profile_token(&p.id)?
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| vault_cfg.vault_token.clone());
            return Ok((p.addr.clone(), token, p.namespace.clone(), p.verify_ssl));
        }
        return Err(anyhow!("HashiCorp Vault profile '{name}' not found"));
    }

    if let Some(p) = vault_cfg.hashicorp_profiles.first() {
        let token = get_vault_profile_token(&p.id)?
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| vault_cfg.vault_token.clone());
        return Ok((p.addr.clone(), token, p.namespace.clone(), p.verify_ssl));
    }

    Ok((
        vault_cfg.vault_addr.clone(),
        vault_cfg.vault_token.clone(),
        vault_cfg.vault_namespace.clone(),
        fallback_verify_ssl,
    ))
}

fn resolve_aws_profile<'a>(
    vault_cfg: &'a VaultConfig,
    profile_name: Option<&str>,
) -> Result<(String, Option<String>, Option<String>)> {
    if let Some(name) = profile_name {
        if let Some(p) = vault_cfg
            .aws_profiles
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name) || p.id == name)
        {
            return Ok((
                p.region.clone(),
                p.profile.clone(),
                p.endpoint_override.clone(),
            ));
        }
        return Err(anyhow!("AWS Vault profile '{name}' not found"));
    }
    if let Some(p) = vault_cfg.aws_profiles.first() {
        return Ok((
            p.region.clone(),
            p.profile.clone(),
            p.endpoint_override.clone(),
        ));
    }
    Ok((
        vault_cfg.aws_region.clone(),
        vault_cfg.aws_profile.clone(),
        None,
    ))
}

fn resolve_keepass_profile(
    vault_cfg: &VaultConfig,
    profile_name: Option<&str>,
) -> Result<(String, Option<String>, Option<String>)> {
    let profile = if let Some(name) = profile_name {
        vault_cfg
            .keepass_profiles
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name) || p.id == name)
            .ok_or_else(|| anyhow!("KeePass profile '{name}' not found"))?
    } else {
        vault_cfg
            .keepass_profiles
            .first()
            .ok_or_else(|| anyhow!("No KeePass (.kdbx) profile configured"))?
    };

    let pw = get_keepass_profile_password(&profile.id)?;
    Ok((profile.db_path.clone(), pw, profile.keyfile_path.clone()))
}

/// Resolve a secret reference URI:
/// - `keyring://<service>/<key>` -> OS Keychain / Credential Manager
/// - `vault://[<profile>@]<path>#<field>` -> HashiCorp Vault KV
/// - `aws-sm://[<profile>@]<secret_id>#<json_key>` -> AWS Secrets Manager
/// - `keepass://[<profile>@]<entry_path>#<field>` -> KeePass `.kdbx`
pub async fn resolve_secret_reference(
    raw: &str,
    vault_cfg: &VaultConfig,
    verify_ssl: bool,
) -> Result<String> {
    if !is_secret_uri(raw) {
        return Ok(raw.to_string());
    }

    if let Some(cached) = cache_get(raw, vault_cfg.cache_ttl_secs) {
        return Ok(cached);
    }

    let resolved = if let Some(rest) = raw.strip_prefix("keyring://") {
        let (service, key) = rest.split_once('/').unwrap_or(("aura", rest));
        let svc = service.to_string();
        let k = key.to_string();
        tokio::task::spawn_blocking(move || keychain_get(&svc, &k))
            .await
            .map_err(|e| anyhow!("Keychain worker failed: {e}"))??
            .ok_or_else(|| anyhow!("Keychain entry not found for '{service}/{key}'"))?
    } else if let Some(rest) = raw.strip_prefix("vault://") {
        let (prof, target) = split_profile_prefix(rest);
        let (path, field) = match target.split_once('#') {
            Some((p, f)) => (p, Some(f)),
            None => (target, None),
        };
        let (addr, token, ns, prof_verify_ssl) =
            resolve_hashicorp_profile(vault_cfg, prof, verify_ssl)?;
        fetch_hashicorp_vault_secret(&addr, &token, ns.as_deref(), path, field, prof_verify_ssl)
            .await?
    } else if let Some(rest) = raw.strip_prefix("aws-sm://") {
        let (prof, target) = split_profile_prefix(rest);
        let (secret_id, key) = match target.split_once('#') {
            Some((id, k)) => (id, Some(k)),
            None => (target, None),
        };
        let (region, aws_prof, endpoint_override) = resolve_aws_profile(vault_cfg, prof)?;
        fetch_aws_secrets_manager(
            secret_id,
            key,
            &region,
            aws_prof.as_deref(),
            endpoint_override.as_deref(),
            verify_ssl,
        )
        .await?
    } else if let Some(rest) = raw.strip_prefix("keepass://") {
        let (prof, target) = split_profile_prefix(rest);
        let (entry_path, field) = match target.split_once('#') {
            Some((p, f)) => (p, Some(f)),
            None => (target, None),
        };
        let (db_path, pw, keyfile) = resolve_keepass_profile(vault_cfg, prof)?;
        fetch_keepass_secret(&db_path, pw.as_deref(), keyfile.as_deref(), entry_path, field)
            .await?
    } else {
        raw.to_string()
    };

    cache_put(raw.to_string(), resolved.clone(), vault_cfg.cache_ttl_secs);
    Ok(resolved)
}

pub fn is_secret_uri(val: &str) -> bool {
    let v = val.trim();
    v.starts_with("keyring://")
        || v.starts_with("vault://")
        || v.starts_with("aws-sm://")
        || v.starts_with("keepass://")
}

/// Resolve all secret URI references inside a `TemplateContext` in-place.
/// Fails fast on the first resolution error and returns all resolved secret values
/// so callers can register them in `SecretScrubber`.
pub async fn resolve_template_secrets(
    ctx: &mut TemplateContext,
    vault_cfg: &VaultConfig,
    verify_ssl: bool,
) -> Result<Vec<String>> {
    let mut resolved_secrets = Vec::new();
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
                let resolved = resolve_secret_reference(&raw, vault_cfg, verify_ssl)
                    .await
                    .map_err(|e| anyhow!("Failed to resolve secret '{key}' ({raw}): {e}"))?;
                resolved_secrets.push(resolved.clone());
                map.insert(key, resolved);
            }
        }
    }
    Ok(resolved_secrets)
}

fn collect_inline_secret_uris_from_auth(auth: &AuthConfig, out: &mut Vec<String>) {
    match auth {
        AuthConfig::Bearer { token } => out.extend(extract_template_keys(token)),
        AuthConfig::Basic { username, password } => {
            out.extend(extract_template_keys(username));
            out.extend(extract_template_keys(password));
        }
        AuthConfig::ApiKey { key, value, .. } => {
            out.extend(extract_template_keys(key));
            out.extend(extract_template_keys(value));
        }
        AuthConfig::OAuth2 {
            access_token,
            client_id,
            client_secret,
            ..
        } => {
            out.extend(extract_template_keys(access_token));
            if let Some(cid) = client_id {
                out.extend(extract_template_keys(cid));
            }
            if let Some(csec) = client_secret {
                out.extend(extract_template_keys(csec));
            }
        }
        AuthConfig::AwsSigV4 {
            access_key,
            secret_key,
            session_token,
            ..
        } => {
            out.extend(extract_template_keys(access_key));
            out.extend(extract_template_keys(secret_key));
            if let Some(st) = session_token {
                out.extend(extract_template_keys(st));
            }
        }
        AuthConfig::Digest {
            username, password, ..
        }
        | AuthConfig::Ntlm {
            username, password, ..
        }
        | AuthConfig::Wsse {
            username, password, ..
        } => {
            out.extend(extract_template_keys(username));
            out.extend(extract_template_keys(password));
        }
        AuthConfig::OAuth1 {
            consumer_key,
            consumer_secret,
            token,
            token_secret,
            ..
        } => {
            out.extend(extract_template_keys(consumer_key));
            out.extend(extract_template_keys(consumer_secret));
            out.extend(extract_template_keys(token));
            out.extend(extract_template_keys(token_secret));
        }
        AuthConfig::Hawk { id, key, .. } => {
            out.extend(extract_template_keys(id));
            out.extend(extract_template_keys(key));
        }
        AuthConfig::AkamaiEdgeGrid {
            client_token,
            client_secret,
            access_token,
            ..
        } => {
            out.extend(extract_template_keys(client_token));
            out.extend(extract_template_keys(client_secret));
            out.extend(extract_template_keys(access_token));
        }
        AuthConfig::Inherit | AuthConfig::None => {}
    }
}

/// Scan a request for inline `{{vault://...}}`, `{{aws-sm://...}}`, `{{keepass://...}}`, `{{keyring://...}}`
/// tokens, resolve them, store them in `ctx.request_vars`, and return resolved secret values for scrubbing.
pub async fn resolve_inline_secrets_in_request(
    req: &AuraRequest,
    ctx: &mut TemplateContext,
    vault_cfg: &VaultConfig,
) -> Result<Vec<String>> {
    let mut candidate_keys = Vec::new();
    candidate_keys.extend(extract_template_keys(&req.url));
    for p in req.params.iter().filter(|p| p.enabled) {
        candidate_keys.extend(extract_template_keys(&p.key));
        candidate_keys.extend(extract_template_keys(&p.value));
    }
    for h in req.headers.iter().filter(|h| h.enabled) {
        candidate_keys.extend(extract_template_keys(&h.key));
        candidate_keys.extend(extract_template_keys(&h.value));
    }
    collect_inline_secret_uris_from_auth(&req.auth, &mut candidate_keys);
    match &req.body {
        BodyType::Json(s) | BodyType::Text(s) | BodyType::Xml(s) | BodyType::Raw { content: s, .. } => {
            candidate_keys.extend(extract_template_keys(s));
        }
        BodyType::Form(map) => {
            for (k, v) in map {
                candidate_keys.extend(extract_template_keys(k));
                candidate_keys.extend(extract_template_keys(v));
            }
        }
        BodyType::FormUrlEncoded(items) => {
            for i in items.iter().filter(|i| i.enabled) {
                candidate_keys.extend(extract_template_keys(&i.key));
                candidate_keys.extend(extract_template_keys(&i.value));
            }
        }
        BodyType::Multipart(items) => {
            for i in items {
                candidate_keys.extend(extract_template_keys(&i.name));
                if let Some(ref val) = i.value {
                    candidate_keys.extend(extract_template_keys(val));
                }
            }
        }
        BodyType::GraphQL { query, variables } => {
            candidate_keys.extend(extract_template_keys(query));
            candidate_keys.extend(extract_template_keys(variables));
        }
        BodyType::None | BodyType::Binary { .. } => {}
    }

    let mut resolved_secrets = Vec::new();
    for uri in candidate_keys {
        if !is_secret_uri(&uri) || ctx.request_vars.contains_key(&uri) {
            continue;
        }
        let val = resolve_secret_reference(&uri, vault_cfg, req.settings.verify_ssl)
            .await
            .map_err(|e| anyhow!("Failed to resolve inline secret '{uri}': {e}"))?;
        resolved_secrets.push(val.clone());
        ctx.request_vars.insert(uri, val);
    }
    Ok(resolved_secrets)
}
