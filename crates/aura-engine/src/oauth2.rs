//! Interactive OAuth 2.0 flows (Authorization Code + PKCE, Client Credentials,
//! Password, Implicit) with local loopback callback listener and automatic token refresh.

use anyhow::{anyhow, Result};
use aura_core::model::AuthConfig;
use aura_core::security_schemes::{build_oauth2_authorize_url, generate_pkce_pair};
use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuth2TokenResult {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: Option<String>,
    pub expires_at: Option<i64>,
}

/// Exchange credentials, authorization code, or refresh token at `token_url`.
#[allow(clippy::too_many_arguments)]
pub async fn exchange_oauth2_token(
    grant_type: &str,
    token_url: &str,
    client_id: Option<&str>,
    client_secret: Option<&str>,
    code: Option<&str>,
    redirect_uri: Option<&str>,
    code_verifier: Option<&str>,
    scope: Option<&str>,
    username: Option<&str>,
    password: Option<&str>,
    refresh_token: Option<&str>,
    verify_ssl: bool,
) -> Result<OAuth2TokenResult> {
    let url = token_url.trim();
    if url.is_empty() {
        return Err(anyhow!("OAuth 2.0 token URL is required"));
    }

    let mut form: Vec<(&str, String)> = vec![("grant_type", grant_type.to_string())];
    if let Some(cid) = client_id.map(str::trim).filter(|s| !s.is_empty()) {
        form.push(("client_id", cid.to_string()));
    }
    if let Some(csec) = client_secret.map(str::trim).filter(|s| !s.is_empty()) {
        form.push(("client_secret", csec.to_string()));
    }
    if let Some(c) = code.map(str::trim).filter(|s| !s.is_empty()) {
        form.push(("code", c.to_string()));
    }
    if let Some(ru) = redirect_uri.map(str::trim).filter(|s| !s.is_empty()) {
        form.push(("redirect_uri", ru.to_string()));
    }
    if let Some(cv) = code_verifier.map(str::trim).filter(|s| !s.is_empty()) {
        form.push(("code_verifier", cv.to_string()));
    }
    if let Some(sc) = scope.map(str::trim).filter(|s| !s.is_empty()) {
        form.push(("scope", sc.to_string()));
    }
    if let Some(u) = username.filter(|s| !s.is_empty()) {
        form.push(("username", u.to_string()));
    }
    if let Some(p) = password.filter(|s| !s.is_empty()) {
        form.push(("password", p.to_string()));
    }
    if let Some(rt) = refresh_token.map(str::trim).filter(|s| !s.is_empty()) {
        form.push(("refresh_token", rt.to_string()));
    }

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(!verify_ssl)
        .timeout(Duration::from_secs(30))
        .build()?;

    let resp = client
        .post(url)
        .header("Accept", "application/json")
        .form(&form)
        .send()
        .await?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(anyhow!(
            "OAuth 2.0 token endpoint returned HTTP {}: {}",
            status.as_u16(),
            body
        ));
    }

    parse_token_response_body(&body)
}

fn parse_token_response_body(body: &str) -> Result<OAuth2TokenResult> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(err) = json.get("error").and_then(|v| v.as_str()) {
            let desc = json
                .get("error_description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            return Err(anyhow!("OAuth 2.0 error '{err}': {desc}"));
        }
        let access_token = json
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("OAuth 2.0 token response missing 'access_token': {body}"))?
            .to_string();
        let refresh_token = json
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let token_type = json
            .get("token_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let expires_in = json.get("expires_in").and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
        });
        let expires_at = expires_in.map(|secs| Utc::now().timestamp() + secs);
        return Ok(OAuth2TokenResult {
            access_token,
            refresh_token,
            token_type,
            expires_at,
        });
    }

    // Fallback: application/x-www-form-urlencoded (e.g. GitHub style)
    let pairs: HashMap<String, String> = url::form_urlencoded::parse(body.as_bytes())
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    if let Some(access_token) = pairs.get("access_token") {
        let expires_at = pairs
            .get("expires_in")
            .and_then(|s| s.parse::<i64>().ok())
            .map(|secs| Utc::now().timestamp() + secs);
        return Ok(OAuth2TokenResult {
            access_token: access_token.clone(),
            refresh_token: pairs.get("refresh_token").cloned(),
            token_type: pairs.get("token_type").cloned(),
            expires_at,
        });
    }

    Err(anyhow!("Unrecognized OAuth 2.0 token response: {body}"))
}

fn open_system_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map_err(|e| anyhow!("Failed to open browser: {e}"))?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| anyhow!("Failed to open browser: {e}"))?;
        return Ok(());
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| anyhow!("Failed to open browser: {e}"))?;
        return Ok(());
    }
}

/// Bind a localhost callback listener and return (listener, effective_redirect_uri, authorize_url, verifier, state).
pub async fn prepare_interactive_oauth2_session(
    grant_type: &str,
    auth_url: &str,
    client_id: &str,
    redirect_uri: &str,
    scope: Option<&str>,
    use_pkce: bool,
) -> Result<(TcpListener, String, String, Option<String>, String)> {
    let ru = if redirect_uri.trim().is_empty() {
        "http://127.0.0.1:8765/callback"
    } else {
        redirect_uri.trim()
    };
    let parsed = url::Url::parse(ru)
        .map_err(|e| anyhow!("Invalid redirect_uri '{ru}': {e}"))?;
    let host = parsed.host_str().unwrap_or("127.0.0.1");
    let port = parsed.port_or_known_default().unwrap_or(8765);
    let bind_addr = format!("{host}:{port}");
    let listener = TcpListener::bind(&bind_addr)
        .await
        .map_err(|e| anyhow!("Failed to bind OAuth 2.0 callback listener on '{bind_addr}': {e}"))?;
    let local_port = listener.local_addr()?.port();

    let mut effective_redirect = parsed.clone();
    let _ = effective_redirect.set_port(Some(local_port));
    let effective_ru = effective_redirect.to_string();

    let state = uuid::Uuid::new_v4().simple().to_string();
    let is_implicit = grant_type.eq_ignore_ascii_case("implicit");
    let (verifier, challenge) = if !is_implicit && use_pkce {
        let (v, c) = generate_pkce_pair();
        (Some(v), Some(c))
    } else {
        (None, None)
    };
    let response_type = if is_implicit { "token" } else { "code" };
    let authorize_url = build_oauth2_authorize_url(
        auth_url,
        client_id,
        &effective_ru,
        scope,
        &state,
        response_type,
        challenge.as_deref(),
    )?;

    Ok((listener, effective_ru, authorize_url, verifier, state))
}

/// Run the full interactive browser/loopback OAuth 2.0 flow on an already-bound listener.
#[allow(clippy::too_many_arguments)]
pub async fn complete_interactive_oauth2_session(
    listener: TcpListener,
    grant_type: &str,
    token_url: Option<&str>,
    client_id: &str,
    client_secret: Option<&str>,
    effective_redirect_uri: &str,
    verifier: Option<&str>,
    expected_state: &str,
    scope: Option<&str>,
    verify_ssl: bool,
) -> Result<OAuth2TokenResult> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);

    loop {
        let accept_res = tokio::time::timeout_at(deadline, listener.accept()).await;
        let (mut stream, _) = match accept_res {
            Ok(Ok(conn)) => conn,
            Ok(Err(e)) => return Err(anyhow!("OAuth 2.0 callback accept error: {e}")),
            Err(_) => return Err(anyhow!("OAuth 2.0 authorization timed out after 180s")),
        };

        let mut buf = [0u8; 8192];
        let n = stream.read(&mut buf).await.unwrap_or(0);
        if n == 0 {
            continue;
        }
        let req_str = String::from_utf8_lossy(&buf[..n]);
        let first_line = req_str.lines().next().unwrap_or("");
        let target = first_line.split_whitespace().nth(1).unwrap_or("/");

        if target.starts_with("/favicon.ico") {
            let _ = stream
                .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await;
            continue;
        }

        let callback_url = url::Url::parse(&format!("http://localhost{target}"))
            .map_err(|e| anyhow!("Malformed callback target '{target}': {e}"))?;
        let params: HashMap<String, String> = callback_url
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        if let Some(err) = params.get("error") {
            let desc = params.get("error_description").map(|s| s.as_str()).unwrap_or("");
            let html = format!(
                "<html><body><h2>Authorization Failed</h2><p>{err}: {desc}</p></body></html>"
            );
            let resp = format!(
                "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                html.len(),
                html
            );
            let _ = stream.write_all(resp.as_bytes()).await;
            return Err(anyhow!("OAuth 2.0 authorization rejected: {err} {desc}"));
        }

        if let Some(access_token) = params.get("access_token") {
            if let Some(st) = params.get("state") {
                if st != expected_state {
                    return Err(anyhow!("OAuth 2.0 state mismatch"));
                }
            }
            let html = "<html><body><h2>Aura OAuth 2.0 Authorization Succeeded</h2><p>You may return to Aura and close this tab.</p></body></html>";
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                html.len(),
                html
            );
            let _ = stream.write_all(resp.as_bytes()).await;
            let expires_at = params
                .get("expires_in")
                .and_then(|s| s.parse::<i64>().ok())
                .map(|secs| Utc::now().timestamp() + secs);
            return Ok(OAuth2TokenResult {
                access_token: access_token.clone(),
                refresh_token: params.get("refresh_token").cloned(),
                token_type: params.get("token_type").cloned(),
                expires_at,
            });
        }

        if let Some(code) = params.get("code") {
            if let Some(st) = params.get("state") {
                if st != expected_state {
                    return Err(anyhow!("OAuth 2.0 callback state mismatch"));
                }
            }
            let html = "<html><body><h2>Aura OAuth 2.0 Authorization Succeeded</h2><p>Exchanging authorization code... You may close this window.</p></body></html>";
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                html.len(),
                html
            );
            let _ = stream.write_all(resp.as_bytes()).await;

            let tu = token_url
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow!("OAuth 2.0 token_url is required to exchange code"))?;
            return exchange_oauth2_token(
                "authorization_code",
                tu,
                Some(client_id),
                client_secret,
                Some(code),
                Some(effective_redirect_uri),
                verifier,
                scope,
                None,
                None,
                None,
                verify_ssl,
            )
            .await;
        }

        // For implicit grant (#access_token=...), browsers send fragment only to JS:
        // bounce fragment query parameters back to this callback listener.
        if grant_type.eq_ignore_ascii_case("implicit") {
            let html = "<html><body><script>if(location.hash.length>1){location.replace(location.pathname+'?'+location.hash.slice(1));}</script><p>Completing OAuth 2.0 implicit flow...</p></body></html>";
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                html.len(),
                html
            );
            let _ = stream.write_all(resp.as_bytes()).await;
            continue;
        }

        let _ = stream
            .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await;
    }
}

/// Execute any configured OAuth 2.0 token acquisition flow interactively or headlessly.
#[allow(clippy::too_many_arguments)]
pub async fn run_oauth2_token_acquisition(
    grant_type: &str,
    auth_url: Option<&str>,
    token_url: Option<&str>,
    client_id: Option<&str>,
    client_secret: Option<&str>,
    redirect_uri: Option<&str>,
    scope: Option<&str>,
    username: Option<&str>,
    password: Option<&str>,
    use_pkce: bool,
    verify_ssl: bool,
) -> Result<OAuth2TokenResult> {
    let gt = grant_type.trim().to_ascii_lowercase();
    match gt.as_str() {
        "client_credentials" => {
            let tu = token_url
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow!("token_url is required for Client Credentials grant"))?;
            exchange_oauth2_token(
                "client_credentials",
                tu,
                client_id,
                client_secret,
                None,
                None,
                None,
                scope,
                None,
                None,
                None,
                verify_ssl,
            )
            .await
        }
        "password" => {
            let tu = token_url
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow!("token_url is required for Password grant"))?;
            exchange_oauth2_token(
                "password",
                tu,
                client_id,
                client_secret,
                None,
                None,
                None,
                scope,
                username,
                password,
                None,
                verify_ssl,
            )
            .await
        }
        _ => {
            let au = auth_url
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow!("auth_url is required for browser OAuth 2.0 grant"))?;
            let cid = client_id
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow!("client_id is required for browser OAuth 2.0 grant"))?;
            let ru = redirect_uri.unwrap_or("http://127.0.0.1:8765/callback");
            let (listener, effective_ru, authorize_url, verifier, state) =
                prepare_interactive_oauth2_session(&gt, au, cid, ru, scope, use_pkce).await?;
            open_system_browser(&authorize_url)?;
            complete_interactive_oauth2_session(
                listener,
                &gt,
                token_url,
                cid,
                client_secret,
                &effective_ru,
                verifier.as_deref(),
                &state,
                scope,
                verify_ssl,
            )
            .await
        }
    }
}

/// Automatically refreshes an OAuth 2.0 token when `auto_refresh` is enabled and the token
/// is missing or within 60 seconds of `expires_at`. Returns `Ok(true)` if updated.
pub async fn maybe_refresh_oauth2_auth(auth: &mut AuthConfig, verify_ssl: bool) -> Result<bool> {
    let AuthConfig::OAuth2 {
        grant_type,
        access_token,
        token_url,
        client_id,
        client_secret,
        scope,
        refresh_token,
        expires_at,
        auto_refresh,
        ..
    } = auth
    else {
        return Ok(false);
    };

    if !*auto_refresh {
        return Ok(false);
    }

    let now = Utc::now().timestamp();
    let is_expiring = expires_at.is_some_and(|exp| now + 60 >= exp);
    let empty_token = access_token.trim().is_empty();
    if !is_expiring && !empty_token {
        return Ok(false);
    }

    let Some(tu) = token_url.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(false);
    };

    if let Some(rt) = refresh_token.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let refreshed = exchange_oauth2_token(
            "refresh_token",
            tu,
            client_id.as_deref(),
            client_secret.as_deref(),
            None,
            None,
            None,
            scope.as_deref(),
            None,
            None,
            Some(rt),
            verify_ssl,
        )
        .await?;
        *access_token = refreshed.access_token;
        if let Some(new_rt) = refreshed.refresh_token {
            *refresh_token = Some(new_rt);
        }
        *expires_at = refreshed.expires_at;
        return Ok(true);
    }

    if grant_type.eq_ignore_ascii_case("client_credentials") {
        let refreshed = exchange_oauth2_token(
            "client_credentials",
            tu,
            client_id.as_deref(),
            client_secret.as_deref(),
            None,
            None,
            None,
            scope.as_deref(),
            None,
            None,
            None,
            verify_ssl,
        )
        .await?;
        *access_token = refreshed.access_token;
        if let Some(new_rt) = refreshed.refresh_token {
            *refresh_token = Some(new_rt);
        }
        *expires_at = refreshed.expires_at;
        return Ok(true);
    }

    Ok(false)
}
