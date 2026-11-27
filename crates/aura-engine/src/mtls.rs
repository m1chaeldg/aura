//! mTLS client certificate loading (.pem, .pfx, .p12 with optional passphrase)
//! and per-domain / per-request certificate matching.

use anyhow::{anyhow, Result};
use aura_core::model::ClientCertificateConfig;
use std::path::Path;

/// Match the most specific enabled client certificate for a target URL:
/// 1. Per-request override if enabled and has a non-empty `cert_path`
/// 2. Domain-scoped certificates matching the URL host (exact or `*.domain` suffix)
pub fn resolve_client_cert_for_url<'a>(
    url_str: &str,
    request_cert: Option<&'a ClientCertificateConfig>,
    domain_certs: &'a [ClientCertificateConfig],
) -> Option<&'a ClientCertificateConfig> {
    if let Some(rc) = request_cert {
        if rc.enabled && !rc.cert_path.trim().is_empty() {
            return Some(rc);
        }
    }

    let parsed = url::Url::parse(url_str).ok()?;
    let host = parsed.host_str()?.to_ascii_lowercase();
    let host_port = match parsed.port() {
        Some(p) => format!("{host}:{p}"),
        None => host.clone(),
    };

    domain_certs.iter().find(|cfg| {
        if !cfg.enabled || cfg.cert_path.trim().is_empty() {
            return false;
        }
        let Some(dom) = cfg.domain.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
            return false;
        };
        let pattern = dom.to_ascii_lowercase();
        if let Some(suffix) = pattern.strip_prefix("*.") {
            host == suffix || host.ends_with(&format!(".{suffix}"))
        } else {
            host == pattern || host_port == pattern
        }
    })
}

/// Load a client certificate configuration (`.pem`/`.crt` with optional `.key`, or `.pfx`/`.p12`
/// with optional passphrase) into a `reqwest::Identity`.
pub fn build_reqwest_identity(cfg: &ClientCertificateConfig) -> Result<reqwest::Identity> {
    let cert_path = cfg.cert_path.trim();
    if cert_path.is_empty() {
        return Err(anyhow!("Client certificate path is empty"));
    }

    let ext = Path::new(cert_path)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    let pem_bytes = if matches!(ext.as_str(), "pfx" | "p12") {
        export_pkcs12_to_pem(cert_path, cfg.passphrase.as_deref())?
    } else {
        let mut combined = std::fs::read(cert_path)
            .map_err(|e| anyhow!("Failed to read client certificate '{cert_path}': {e}"))?;
        if let Some(key_path) = cfg.key_path.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            let key_bytes = std::fs::read(key_path)
                .map_err(|e| anyhow!("Failed to read client private key '{key_path}': {e}"))?;
            combined.push(b'\n');
            combined.extend_from_slice(&key_bytes);
        }
        combined
    };

    reqwest::Identity::from_pem(&pem_bytes)
        .map_err(|e| anyhow!("Failed to parse client TLS identity from '{cert_path}': {e}"))
}

fn export_pkcs12_to_pem(path: &str, passphrase: Option<&str>) -> Result<Vec<u8>> {
    let pass = passphrase.unwrap_or("");
    // 1. Try openssl CLI if available (cross-platform)
    if let Ok(output) = std::process::Command::new("openssl")
        .args([
            "pkcs12",
            "-in",
            path,
            "-nodes",
            "-passin",
            &format!("pass:{pass}"),
        ])
        .output()
    {
        if output.status.success() && !output.stdout.is_empty() {
            return Ok(output.stdout);
        }
    }

    // 2. Windows PowerShell (.NET Framework 4.8+ / .NET Core compatible) X509Certificate2 exporter
    #[cfg(target_os = "windows")]
    {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;

        // Also probe common Windows OpenSSL locations before PowerShell fallback
        for candidate in [
            "C:\\Program Files\\Git\\usr\\bin\\openssl.exe",
            "C:\\Program Files\\Git\\mingw64\\bin\\openssl.exe",
        ] {
            if std::path::Path::new(candidate).exists() {
                if let Ok(output) = std::process::Command::new(candidate)
                    .args([
                        "pkcs12",
                        "-in",
                        path,
                        "-nodes",
                        "-passin",
                        &format!("pass:{pass}"),
                    ])
                    .output()
                {
                    if output.status.success() && !output.stdout.is_empty() {
                        return Ok(output.stdout);
                    }
                }
            }
        }

        let escaped_path = path.replace('\'', "''");
        let escaped_pass = pass.replace('\'', "''");
        let ps_script = format!(
            "$cert = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2('{escaped_path}', '{escaped_pass}', [System.Security.Cryptography.X509Certificates.X509KeyStorageFlags]::Exportable); \
             $certB64 = [Convert]::ToBase64String($cert.Export([System.Security.Cryptography.X509Certificates.X509ContentType]::Cert)); \
             $rsa = [System.Security.Cryptography.X509Certificates.RSACertificateExtensions]::GetRSAPrivateKey($cert); \
             if ($null -eq $rsa) {{ throw 'No RSA private key found in archive' }}; \
             $p = $rsa.ExportParameters($true); \
             $obj = @{{ \
               cert = $certB64; \
               n = [Convert]::ToBase64String($p.Modulus); \
               e = [Convert]::ToBase64String($p.Exponent); \
               d = [Convert]::ToBase64String($p.D); \
               p = [Convert]::ToBase64String($p.P); \
               q = [Convert]::ToBase64String($p.Q); \
               dp = [Convert]::ToBase64String($p.DP); \
               dq = [Convert]::ToBase64String($p.DQ); \
               iq = [Convert]::ToBase64String($p.InverseQ); \
             }}; \
             $obj | ConvertTo-Json -Compress"
        );
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
            .output()
            .map_err(|e| anyhow!("Failed to invoke PowerShell for PKCS#12 conversion: {e}"))?;
        if !output.status.success() || output.stdout.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "Failed to decode PKCS#12 archive '{path}' (check passphrase or install OpenSSL): {stderr}"
            ));
        }

        let json: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|e| {
            anyhow!("Failed to parse Windows PKCS#12 JSON output: {e}")
        })?;
        let get_b64 = |k: &str| -> Result<Vec<u8>> {
            let s = json
                .get(k)
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing RSA field '{k}' from PKCS#12 export"))?;
            Ok(STANDARD.decode(s)?)
        };

        let cert_der = get_b64("cert")?;
        let pkcs1_der = encode_rsa_pkcs1_der(
            &get_b64("n")?,
            &get_b64("e")?,
            &get_b64("d")?,
            &get_b64("p")?,
            &get_b64("q")?,
            &get_b64("dp")?,
            &get_b64("dq")?,
            &get_b64("iq")?,
        );

        let mut pem = String::new();
        pem.push_str(&wrap_pem("CERTIFICATE", &STANDARD.encode(&cert_der)));
        pem.push_str(&wrap_pem("RSA PRIVATE KEY", &STANDARD.encode(&pkcs1_der)));
        return Ok(pem.into_bytes());
    }

    #[cfg(not(target_os = "windows"))]
    Err(anyhow!(
        "PKCS#12 (.pfx/.p12) conversion requires 'openssl' CLI in PATH on this OS"
    ))
}

#[cfg(target_os = "windows")]
fn wrap_pem(label: &str, b64: &str) -> String {
    let mut out = format!("-----BEGIN {label}-----\n");
    for chunk in b64.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(chunk).unwrap_or(""));
        out.push('\n');
    }
    out.push_str(&format!("-----END {label}-----\n"));
    out
}

#[cfg(target_os = "windows")]
fn der_len(len: usize) -> Vec<u8> {
    if len < 0x80 {
        vec![len as u8]
    } else if len <= 0xff {
        vec![0x81, len as u8]
    } else {
        vec![0x82, (len >> 8) as u8, (len & 0xff) as u8]
    }
}

#[cfg(target_os = "windows")]
fn der_int(bytes: &[u8]) -> Vec<u8> {
    let mut trimmed = bytes;
    while trimmed.len() > 1 && trimmed[0] == 0 {
        trimmed = &trimmed[1..];
    }
    let needs_pad = trimmed.first().is_some_and(|b| b & 0x80 != 0);
    let content_len = trimmed.len() + usize::from(needs_pad);
    let mut out = vec![0x02];
    out.extend_from_slice(&der_len(content_len));
    if needs_pad {
        out.push(0);
    }
    out.extend_from_slice(trimmed);
    out
}

#[cfg(target_os = "windows")]
#[allow(clippy::too_many_arguments)]
fn encode_rsa_pkcs1_der(
    n: &[u8],
    e: &[u8],
    d: &[u8],
    p: &[u8],
    q: &[u8],
    dp: &[u8],
    dq: &[u8],
    iq: &[u8],
) -> Vec<u8> {
    let mut seq = Vec::new();
    seq.extend_from_slice(&der_int(&[0])); // version = 0
    for comp in [n, e, d, p, q, dp, dq, iq] {
        seq.extend_from_slice(&der_int(comp));
    }
    let mut out = vec![0x30];
    out.extend_from_slice(&der_len(seq.len()));
    out.extend_from_slice(&seq);
    out
}
