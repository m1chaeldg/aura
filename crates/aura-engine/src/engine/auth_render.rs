//! Template rendering for `AuthConfig`, request auth injection, and 401 challenge-response retries.

use anyhow::{anyhow, Result};
use aura_core::model::AuthConfig;
use aura_core::security::{
    apply_request_auth, resolve_aws_profile_credentials, sign_aws_sigv4, AwsSigV4Credentials,
};
use aura_core::security_schemes::{
    build_digest_authorization, build_ntlm_type1_negotiate, build_ntlm_type3_authenticate,
    parse_ntlm_type2_challenge, DigestChallenge,
};
use aura_core::templating::TemplateContext;
use reqwest::header::{HeaderMap, HeaderValue};

pub(super) fn render_auth_config(auth: &AuthConfig, ctx: &TemplateContext) -> AuthConfig {
    let opt = |v: &Option<String>| v.as_ref().map(|s| ctx.render(s));
    match auth {
        AuthConfig::Inherit => AuthConfig::Inherit,
        AuthConfig::None => AuthConfig::None,
        AuthConfig::Bearer { token } => AuthConfig::Bearer {
            token: ctx.render(token),
        },
        AuthConfig::Basic { username, password } => AuthConfig::Basic {
            username: ctx.render(username),
            password: ctx.render(password),
        },
        AuthConfig::ApiKey {
            key,
            value,
            in_header,
        } => AuthConfig::ApiKey {
            key: ctx.render(key),
            value: ctx.render(value),
            in_header: *in_header,
        },
        AuthConfig::AwsSigV4 {
            use_profile,
            profile,
            credentials_path,
            access_key,
            secret_key,
            region,
            service,
            session_token,
        } => AuthConfig::AwsSigV4 {
            use_profile: *use_profile,
            profile: ctx.render(profile),
            credentials_path: opt(credentials_path),
            access_key: ctx.render(access_key),
            secret_key: ctx.render(secret_key),
            region: ctx.render(region),
            service: ctx.render(service),
            session_token: opt(session_token),
        },
        AuthConfig::OAuth2 {
            grant_type,
            access_token,
            token_url,
            client_id,
            client_secret,
            auth_url,
            redirect_uri,
            scope,
            username,
            password,
            refresh_token,
            expires_at,
            use_pkce,
            auto_refresh,
        } => AuthConfig::OAuth2 {
            grant_type: ctx.render(grant_type),
            access_token: ctx.render(access_token),
            token_url: opt(token_url),
            client_id: opt(client_id),
            client_secret: opt(client_secret),
            auth_url: opt(auth_url),
            redirect_uri: opt(redirect_uri),
            scope: opt(scope),
            username: opt(username),
            password: opt(password),
            refresh_token: opt(refresh_token),
            expires_at: *expires_at,
            use_pkce: *use_pkce,
            auto_refresh: *auto_refresh,
        },
        AuthConfig::Digest {
            username,
            password,
            realm,
            nonce,
            qop,
            algorithm,
        } => AuthConfig::Digest {
            username: ctx.render(username),
            password: ctx.render(password),
            realm: opt(realm),
            nonce: opt(nonce),
            qop: opt(qop),
            algorithm: opt(algorithm),
        },
        AuthConfig::Ntlm {
            username,
            password,
            domain,
            workstation,
        } => AuthConfig::Ntlm {
            username: ctx.render(username),
            password: ctx.render(password),
            domain: ctx.render(domain),
            workstation: ctx.render(workstation),
        },
        AuthConfig::OAuth1 {
            consumer_key,
            consumer_secret,
            token,
            token_secret,
            signature_method,
            callback_url,
            realm,
            add_to_query,
        } => AuthConfig::OAuth1 {
            consumer_key: ctx.render(consumer_key),
            consumer_secret: ctx.render(consumer_secret),
            token: ctx.render(token),
            token_secret: ctx.render(token_secret),
            signature_method: ctx.render(signature_method),
            callback_url: opt(callback_url),
            realm: opt(realm),
            add_to_query: *add_to_query,
        },
        AuthConfig::Hawk {
            id,
            key,
            algorithm,
            ext,
            app,
            dlg,
        } => AuthConfig::Hawk {
            id: ctx.render(id),
            key: ctx.render(key),
            algorithm: ctx.render(algorithm),
            ext: opt(ext),
            app: opt(app),
            dlg: opt(dlg),
        },
        AuthConfig::AkamaiEdgeGrid {
            client_token,
            client_secret,
            access_token,
            max_body,
        } => AuthConfig::AkamaiEdgeGrid {
            client_token: ctx.render(client_token),
            client_secret: ctx.render(client_secret),
            access_token: ctx.render(access_token),
            max_body: *max_body,
        },
        AuthConfig::Wsse {
            username,
            password,
            digest_password,
        } => AuthConfig::Wsse {
            username: ctx.render(username),
            password: ctx.render(password),
            digest_password: *digest_password,
        },
    }
}

pub(super) fn apply_rendered_auth(
    req_method: &reqwest::Method,
    rendered_url: &str,
    rendered_auth: &AuthConfig,
    body_bytes: &[u8],
    header_pairs: &mut Vec<(String, String)>,
    url_params: &mut Vec<(String, String)>,
) -> Result<()> {
    let mut full_url = url::Url::parse(rendered_url)
        .map_err(|e| anyhow!("Invalid URL '{rendered_url}': {e}"))?;
    if !url_params.is_empty() {
        let mut qp = full_url.query_pairs_mut();
        for (k, v) in url_params.iter() {
            qp.append_pair(k, v);
        }
    }

    if let AuthConfig::AwsSigV4 {
        use_profile,
        profile,
        credentials_path,
        access_key,
        secret_key,
        region,
        service,
        session_token,
    } = rendered_auth
    {
        let creds = if *use_profile {
            resolve_aws_profile_credentials(
                profile,
                credentials_path.as_deref(),
                region,
                service,
            )?
        } else {
            AwsSigV4Credentials {
                access_key: access_key.clone(),
                secret_key: secret_key.clone(),
                session_token: session_token.clone(),
                region: if region.trim().is_empty() {
                    "us-east-1".to_string()
                } else {
                    region.clone()
                },
                service: if service.trim().is_empty() {
                    "execute-api".to_string()
                } else {
                    service.clone()
                },
            }
        };
        sign_aws_sigv4(
            req_method.as_str(),
            full_url.as_str(),
            header_pairs,
            body_bytes,
            &creds,
            chrono::Utc::now(),
        )?;
        return Ok(());
    }

    apply_request_auth(
        req_method.as_str(),
        full_url.as_str(),
        body_bytes,
        rendered_auth,
        header_pairs,
        url_params,
    )?;
    if matches!(rendered_auth, AuthConfig::Ntlm { .. }) {
        header_pairs.push(("Authorization".to_string(), build_ntlm_type1_negotiate()));
    }
    Ok(())
}

pub(super) async fn maybe_handle_auth_challenge(
    client: &reqwest::Client,
    req_method: &reqwest::Method,
    rendered_url: &str,
    rendered_auth: &AuthConfig,
    header_map: &HeaderMap,
    url_params: &[(String, String)],
    body_bytes: &[u8],
    is_multipart: bool,
    mut resp: reqwest::Response,
) -> Result<reqwest::Response> {
    if resp.status().as_u16() != 401 || is_multipart {
        return Ok(resp);
    }

    let www_auths: Vec<String> = resp
        .headers()
        .get_all(reqwest::header::WWW_AUTHENTICATE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(ToString::to_string)
        .collect();

    let retry_auth_hdr: Option<String> = match rendered_auth {
        AuthConfig::Digest {
            username, password, ..
        } => www_auths
            .iter()
            .find_map(|hdr| DigestChallenge::parse(hdr))
            .map(|challenge| {
                let uri = url::Url::parse(rendered_url)
                    .map(|u| match u.query() {
                        Some(q) => format!("{}?{q}", u.path()),
                        None => u.path().to_string(),
                    })
                    .unwrap_or_else(|_| "/".to_string());
                build_digest_authorization(req_method.as_str(), &uri, username, password, &challenge)
            }),
        AuthConfig::Ntlm {
            username,
            password,
            domain,
            workstation,
        } => www_auths
            .iter()
            .find_map(|hdr| parse_ntlm_type2_challenge(hdr).ok())
            .and_then(|type2| {
                build_ntlm_type3_authenticate(username, password, domain, workstation, &type2).ok()
            }),
        _ => None,
    };

    if let Some(auth_val) = retry_auth_hdr {
        if let Ok(val) = HeaderValue::from_str(&auth_val) {
            let _ = resp.bytes().await;
            let mut retry_headers = header_map.clone();
            retry_headers.insert(reqwest::header::AUTHORIZATION, val);
            let mut retry_builder = client
                .request(req_method.clone(), rendered_url)
                .headers(retry_headers);
            if !url_params.is_empty() {
                retry_builder = retry_builder.query(url_params);
            }
            if !body_bytes.is_empty() {
                retry_builder = retry_builder.body(body_bytes.to_vec());
            }
            resp = retry_builder
                .send()
                .await
                .map_err(|e| anyhow!("Auth challenge-response retry failed: {e}"))?;
        }
    }

    Ok(resp)
}
