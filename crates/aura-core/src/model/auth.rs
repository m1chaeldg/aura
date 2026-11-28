//! Authentication, mTLS client certificate, and proxy configuration models.

use serde::{Deserialize, Serialize};

/// Authentication type with cascading inheritance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthConfig {
    #[default]
    Inherit,
    None,
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    ApiKey {
        key: String,
        value: String,
        in_header: bool, // true: header, false: query
    },
    #[serde(alias = "aws_v4")]
    AwsSigV4 {
        #[serde(default)]
        use_profile: bool,
        #[serde(default)]
        profile: String,
        #[serde(default)]
        credentials_path: Option<String>,
        #[serde(default)]
        access_key: String,
        #[serde(default)]
        secret_key: String,
        region: String,
        service: String,
        #[serde(default)]
        session_token: Option<String>,
    },
    OAuth2 {
        grant_type: String,
        access_token: String,
        #[serde(default)]
        token_url: Option<String>,
        #[serde(default)]
        client_id: Option<String>,
        #[serde(default)]
        client_secret: Option<String>,
        #[serde(default)]
        auth_url: Option<String>,
        #[serde(default)]
        redirect_uri: Option<String>,
        #[serde(default)]
        scope: Option<String>,
        #[serde(default)]
        username: Option<String>,
        #[serde(default)]
        password: Option<String>,
        #[serde(default)]
        refresh_token: Option<String>,
        #[serde(default)]
        expires_at: Option<i64>,
        #[serde(default)]
        use_pkce: bool,
        #[serde(default)]
        auto_refresh: bool,
    },
    Digest {
        username: String,
        password: String,
        #[serde(default)]
        realm: Option<String>,
        #[serde(default)]
        nonce: Option<String>,
        #[serde(default)]
        qop: Option<String>,
        #[serde(default)]
        algorithm: Option<String>,
    },
    Ntlm {
        username: String,
        password: String,
        #[serde(default)]
        domain: String,
        #[serde(default)]
        workstation: String,
    },
    #[serde(alias = "oauth1")]
    OAuth1 {
        consumer_key: String,
        consumer_secret: String,
        #[serde(default)]
        token: String,
        #[serde(default)]
        token_secret: String,
        #[serde(default)]
        signature_method: String,
        #[serde(default)]
        callback_url: Option<String>,
        #[serde(default)]
        realm: Option<String>,
        #[serde(default)]
        add_to_query: bool,
    },
    Hawk {
        id: String,
        key: String,
        #[serde(default)]
        algorithm: String,
        #[serde(default)]
        ext: Option<String>,
        #[serde(default)]
        app: Option<String>,
        #[serde(default)]
        dlg: Option<String>,
    },
    #[serde(alias = "edgegrid")]
    AkamaiEdgeGrid {
        client_token: String,
        client_secret: String,
        access_token: String,
        #[serde(default)]
        max_body: usize,
    },
    Wsse {
        username: String,
        password: String,
        #[serde(default)]
        digest_password: bool,
    },
}

impl AuthConfig {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Inherit => "Inherit",
            Self::None => "None",
            Self::Bearer { .. } => "Bearer Token",
            Self::Basic { .. } => "Basic Auth",
            Self::ApiKey { .. } => "API Key",
            Self::AwsSigV4 { .. } => "AWS SigV4",
            Self::OAuth2 { .. } => "OAuth 2.0",
            Self::Digest { .. } => "Digest Auth",
            Self::Ntlm { .. } => "NTLM",
            Self::OAuth1 { .. } => "OAuth 1.0",
            Self::Hawk { .. } => "Hawk",
            Self::AkamaiEdgeGrid { .. } => "Akamai EdgeGrid",
            Self::Wsse { .. } => "WSSE",
        }
    }

    pub fn default_for_type(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "inherit" => Self::Inherit,
            "none" => Self::None,
            "bearer" | "bearer token" => Self::Bearer {
                token: "".to_string(),
            },
            "basic" | "basic auth" => Self::Basic {
                username: "".to_string(),
                password: "".to_string(),
            },
            "api_key" | "apikey" | "api key" => Self::ApiKey {
                key: "X-API-Key".to_string(),
                value: "".to_string(),
                in_header: true,
            },
            "aws" | "aws_sigv4" | "awssigv4" | "aws sigv4" => Self::AwsSigV4 {
                use_profile: true,
                profile: "default".to_string(),
                credentials_path: None,
                access_key: "".to_string(),
                secret_key: "".to_string(),
                region: "us-east-1".to_string(),
                service: "execute-api".to_string(),
                session_token: None,
            },
            "oauth2" | "oauth 2.0" => Self::OAuth2 {
                grant_type: "authorization_code".to_string(),
                access_token: "".to_string(),
                token_url: None,
                client_id: None,
                client_secret: None,
                auth_url: None,
                redirect_uri: Some("http://127.0.0.1:8765/callback".to_string()),
                scope: None,
                username: None,
                password: None,
                refresh_token: None,
                expires_at: None,
                use_pkce: true,
                auto_refresh: true,
            },
            "digest" | "digest auth" => Self::Digest {
                username: "".to_string(),
                password: "".to_string(),
                realm: None,
                nonce: None,
                qop: Some("auth".to_string()),
                algorithm: Some("MD5".to_string()),
            },
            "ntlm" => Self::Ntlm {
                username: "".to_string(),
                password: "".to_string(),
                domain: "".to_string(),
                workstation: "AURA-WORKSTATION".to_string(),
            },
            "oauth1" | "oauth 1.0" => Self::OAuth1 {
                consumer_key: "".to_string(),
                consumer_secret: "".to_string(),
                token: "".to_string(),
                token_secret: "".to_string(),
                signature_method: "HMAC-SHA1".to_string(),
                callback_url: None,
                realm: None,
                add_to_query: false,
            },
            "hawk" => Self::Hawk {
                id: "".to_string(),
                key: "".to_string(),
                algorithm: "sha256".to_string(),
                ext: None,
                app: None,
                dlg: None,
            },
            "akamai_edgegrid" | "akamai edgegrid" | "edgegrid" => Self::AkamaiEdgeGrid {
                client_token: "".to_string(),
                client_secret: "".to_string(),
                access_token: "".to_string(),
                max_body: 131072,
            },
            "wsse" => Self::Wsse {
                username: "".to_string(),
                password: "".to_string(),
                digest_password: true,
            },
            _ => Self::None,
        }
    }
}

/// Client certificate configuration for mTLS (.pem or .pfx/.p12).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ClientCertificateConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub cert_path: String,
    #[serde(default)]
    pub key_path: Option<String>,
    #[serde(default)]
    pub passphrase: Option<String>,
}

/// Supported outbound proxy protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ProxyProtocol {
    #[default]
    Http,
    Https,
    Socks5,
    Socks5h,
}

impl ProxyProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Http => "HTTP",
            Self::Https => "HTTPS",
            Self::Socks5 => "SOCKS5",
            Self::Socks5h => "SOCKS5H",
        }
    }

    pub fn scheme(&self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
            Self::Socks5 => "socks5",
            Self::Socks5h => "socks5h",
        }
    }
}

/// Global or per-request proxy configuration (`http://`, `https://`, `socks5://`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub protocol: ProxyProtocol,
    #[serde(default)]
    pub host: String,
    #[serde(default = "default_proxy_port")]
    pub port: u16,
    #[serde(default)]
    pub auth_enabled: bool,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub bypass_list: Vec<String>,
}

fn default_proxy_port() -> u16 {
    8080
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            protocol: ProxyProtocol::Http,
            host: String::new(),
            port: 8080,
            auth_enabled: false,
            username: String::new(),
            password: String::new(),
            bypass_list: vec!["localhost".to_string(), "127.0.0.1".to_string()],
        }
    }
}

impl ProxyConfig {
    pub fn effective_url(&self) -> Option<String> {
        let host = self.host.trim();
        if !self.enabled || host.is_empty() {
            return None;
        }
        let scheme = self.protocol.scheme();
        if self.auth_enabled && !self.username.is_empty() {
            Some(format!(
                "{scheme}://{}:{}@{host}:{}",
                self.username, self.password, self.port
            ))
        } else {
            Some(format!("{scheme}://{host}:{}", self.port))
        }
    }

    pub fn should_bypass(&self, target_url: &str) -> bool {
        let Ok(parsed) = url::Url::parse(target_url) else {
            return false;
        };
        let Some(host) = parsed.host_str().map(|h| h.to_ascii_lowercase()) else {
            return false;
        };
        let host_port = match parsed.port() {
            Some(p) => format!("{host}:{p}"),
            None => host.clone(),
        };
        for raw in &self.bypass_list {
            let pat = raw.trim().to_ascii_lowercase();
            if pat.is_empty() {
                continue;
            }
            if pat == "*" || pat == host || pat == host_port {
                return true;
            }
            if let Some(suffix) = pat.strip_prefix("*.") {
                if host == suffix || host.ends_with(&format!(".{suffix}")) {
                    return true;
                }
            } else if let Some(suffix) = pat.strip_prefix('.') {
                if host == suffix || host.ends_with(&format!(".{suffix}")) {
                    return true;
                }
            }
        }
        false
    }
}
