//! RFC 6265 Cookie Jar, Set-Cookie parser, and domain-scoped persistence.

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CookieEntry {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    #[serde(default)]
    pub expires: Option<String>,
    #[serde(default)]
    pub max_age: Option<i64>,
    #[serde(default)]
    pub secure: bool,
    #[serde(default)]
    pub http_only: bool,
    #[serde(default)]
    pub same_site: Option<String>,
    #[serde(default)]
    pub host_only: bool,
    #[serde(default)]
    pub expires_at: Option<i64>,
}

impl CookieEntry {
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        if let Some(ma) = self.max_age {
            if ma <= 0 {
                return true;
            }
        }
        if let Some(exp_ts) = self.expires_at {
            return exp_ts <= now.timestamp();
        }
        false
    }

    pub fn matches_url(&self, parsed: &url::Url, now: DateTime<Utc>) -> bool {
        if self.is_expired(now) {
            return false;
        }
        if self.secure && !matches!(parsed.scheme(), "https" | "wss") {
            return false;
        }
        let Some(req_host) = parsed.host_str().map(|h| h.to_ascii_lowercase()) else {
            return false;
        };
        let cookie_domain = self.domain.trim_start_matches('.').to_ascii_lowercase();
        if cookie_domain.is_empty() {
            return false;
        }
        let domain_matches = if self.host_only {
            req_host == cookie_domain
        } else {
            req_host == cookie_domain || req_host.ends_with(&format!(".{cookie_domain}"))
        };
        if !domain_matches {
            return false;
        }

        let req_path = if parsed.path().is_empty() {
            "/"
        } else {
            parsed.path()
        };
        let c_path = if self.path.is_empty() {
            "/"
        } else {
            self.path.as_str()
        };
        if c_path == "/" {
            return true;
        }
        if req_path == c_path {
            return true;
        }
        if req_path.starts_with(c_path) {
            return c_path.ends_with('/') || req_path[c_path.len()..].starts_with('/');
        }
        false
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CookieJar {
    #[serde(default)]
    pub cookies: Vec<CookieEntry>,
}

impl CookieJar {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a single `Set-Cookie` header value scoped to `request_url`.
    pub fn parse_set_cookie(header_val: &str, request_url: &str) -> Option<CookieEntry> {
        let parsed_url = url::Url::parse(request_url).ok()?;
        let req_host = parsed_url.host_str()?.to_ascii_lowercase();
        let req_path = default_cookie_path(parsed_url.path());

        let mut parts = header_val.split(';');
        let nv = parts.next()?.trim();
        let (name_raw, val_raw) = nv.split_once('=')?;
        let name = name_raw.trim().to_string();
        if name.is_empty() {
            return None;
        }
        let value = val_raw.trim().to_string();

        let mut domain = req_host.clone();
        let mut host_only = true;
        let mut path = req_path;
        let mut expires: Option<String> = None;
        let mut expires_at: Option<i64> = None;
        let mut max_age: Option<i64> = None;
        let mut secure = false;
        let mut http_only = false;
        let mut same_site: Option<String> = None;

        for attr in parts {
            let trimmed = attr.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                let k_low = k.trim().to_ascii_lowercase();
                let v_trim = v.trim();
                match k_low.as_str() {
                    "domain" => {
                        let clean = v_trim.trim_start_matches('.').to_ascii_lowercase();
                        if !clean.is_empty()
                            && (req_host == clean || req_host.ends_with(&format!(".{clean}")))
                        {
                            domain = clean;
                            host_only = false;
                        }
                    }
                    "path" => {
                        if v_trim.starts_with('/') {
                            path = v_trim.to_string();
                        }
                    }
                    "max-age" => {
                        if let Ok(secs) = v_trim.parse::<i64>() {
                            max_age = Some(secs);
                            expires_at = Some(Utc::now().timestamp() + secs);
                        }
                    }
                    "expires" => {
                        expires = Some(v_trim.to_string());
                        if max_age.is_none() {
                            if let Some(ts) = parse_http_date_timestamp(v_trim) {
                                expires_at = Some(ts);
                            }
                        }
                    }
                    "samesite" => {
                        same_site = Some(v_trim.to_string());
                    }
                    _ => {}
                }
            } else {
                match trimmed.to_ascii_lowercase().as_str() {
                    "secure" => secure = true,
                    "httponly" => http_only = true,
                    _ => {}
                }
            }
        }

        Some(CookieEntry {
            name,
            value,
            domain,
            path,
            expires,
            max_age,
            secure,
            http_only,
            same_site,
            host_only,
            expires_at,
        })
    }

    /// Persist all `Set-Cookie` headers from a response into the jar.
    /// Returns the parsed cookies stored or updated.
    pub fn apply_set_cookie_headers(
        &mut self,
        request_url: &str,
        set_cookie_headers: &[String],
    ) -> Vec<CookieEntry> {
        let now = Utc::now();
        let mut applied = Vec::new();
        for raw in set_cookie_headers {
            if let Some(entry) = Self::parse_set_cookie(raw, request_url) {
                applied.push(entry.clone());
                self.set_cookie(entry, now);
            }
        }
        applied
    }

    pub fn set_cookie(&mut self, entry: CookieEntry, now: DateTime<Utc>) {
        let key_dom = entry.domain.trim_start_matches('.').to_ascii_lowercase();
        self.cookies.retain(|c| {
            !(c.name == entry.name
                && c.domain.trim_start_matches('.').to_ascii_lowercase() == key_dom
                && c.path == entry.path)
        });
        if !entry.is_expired(now) {
            self.cookies.push(entry);
        }
    }

    pub fn remove_cookie(&mut self, domain: &str, path: &str, name: &str) {
        let key_dom = domain.trim_start_matches('.').to_ascii_lowercase();
        self.cookies.retain(|c| {
            !(c.name == name
                && c.domain.trim_start_matches('.').to_ascii_lowercase() == key_dom
                && c.path == path)
        });
    }

    pub fn clear_domain(&mut self, domain: &str) {
        let target = domain.trim_start_matches('.').to_ascii_lowercase();
        self.cookies.retain(|c| {
            let cd = c.domain.trim_start_matches('.').to_ascii_lowercase();
            cd != target && !cd.ends_with(&format!(".{target}"))
        });
    }

    pub fn clear_all(&mut self) {
        self.cookies.clear();
    }

    /// Build the outgoing `Cookie` header value matching `request_url`.
    pub fn cookie_header_for_url(&self, request_url: &str) -> Option<String> {
        let parsed = url::Url::parse(request_url).ok()?;
        let now = Utc::now();
        let mut matched: Vec<&CookieEntry> = self
            .cookies
            .iter()
            .filter(|c| c.matches_url(&parsed, now))
            .collect();
        if matched.is_empty() {
            return None;
        }
        // Longest path first per RFC 6265 §5.4
        matched.sort_by(|a, b| b.path.len().cmp(&a.path.len()));
        let joined = matched
            .into_iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ");
        Some(joined)
    }

    pub fn cookies_for_domain(&self, domain_filter: &str) -> Vec<&CookieEntry> {
        let f = domain_filter.trim().to_ascii_lowercase();
        if f.is_empty() {
            return self.cookies.iter().collect();
        }
        self.cookies
            .iter()
            .filter(|c| c.domain.to_ascii_lowercase().contains(&f))
            .collect()
    }
}

fn default_cookie_path(uri_path: &str) -> String {
    if uri_path.is_empty() || !uri_path.starts_with('/') {
        return "/".to_string();
    }
    if uri_path == "/" {
        return "/".to_string();
    }
    match uri_path.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(idx) => uri_path[..idx].to_string(),
    }
}

fn parse_http_date_timestamp(raw: &str) -> Option<i64> {
    if let Ok(dt) = DateTime::parse_from_rfc2822(raw) {
        return Some(dt.timestamp());
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return Some(dt.timestamp());
    }
    for fmt in [
        "%a, %d-%b-%Y %H:%M:%S GMT",
        "%a, %d %b %Y %H:%M:%S GMT",
        "%d-%b-%Y %H:%M:%S GMT",
        "%a, %d %b %Y %H:%M:%S %Z",
    ] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(raw, fmt) {
            return Some(naive.and_utc().timestamp());
        }
    }
    None
}
