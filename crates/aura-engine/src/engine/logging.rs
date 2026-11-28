//! Payload truncation, header redaction, and reqwest error diagnostics.

pub(super) const MAX_LOG_PAYLOAD_LEN: usize = 2048;

pub(super) struct RequestErrorDetails {
    pub(super) kind: &'static str,
    pub(super) cause_chain: String,
    pub(super) possible_cause: String,
}

pub(super) fn truncate_for_log(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut end = max_len;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}... [truncated, {} total bytes]", &s[..end], s.len())
    }
}

pub(super) fn is_sensitive_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "authorization"
            | "proxy-authorization"
            | "cookie"
            | "set-cookie"
            | "x-api-key"
            | "api-key"
            | "x-auth-token"
    )
}

pub(super) fn format_kv_for_log(
    pairs: &[(String, String)],
    redact_sensitive_keys: bool,
) -> String {
    if pairs.is_empty() {
        return "[]".to_string();
    }
    let items: Vec<String> = pairs
        .iter()
        .map(|(k, v)| {
            if redact_sensitive_keys && is_sensitive_header(k) {
                format!("{k}=***")
            } else {
                format!("{k}={}", truncate_for_log(v, 256))
            }
        })
        .collect();
    format!("[{}]", items.join(", "))
}

pub(super) fn describe_reqwest_error(
    err: &reqwest::Error,
    timeout_ms: u64,
    max_redirects: usize,
) -> RequestErrorDetails {
    let mut causes = vec![err.to_string()];
    let mut current: Option<&dyn std::error::Error> = std::error::Error::source(err);
    while let Some(src) = current {
        causes.push(src.to_string());
        current = src.source();
    }
    let cause_chain = causes.join(" -> ");
    let lower = cause_chain.to_ascii_lowercase();

    let kind = if err.is_timeout() {
        "Timeout"
    } else if err.is_connect() {
        "Connect"
    } else if err.is_redirect() {
        "Redirect"
    } else if err.is_builder() {
        "Builder"
    } else if err.is_request() {
        "Request"
    } else if err.is_body() {
        "Body"
    } else if err.is_decode() {
        "Decode"
    } else if err.is_status() {
        "Status"
    } else {
        "Network"
    };

    let possible_cause = match kind {
        "Timeout" => {
            if timeout_ms == 0 {
                "Request timed out at the OS/network layer.".to_string()
            } else {
                format!(
                    "Request timed out after {timeout_ms}ms. The server may be slow or unreachable, or timeout_ms may need to be increased."
                )
            }
        }
        "Connect" => {
            if lower.contains("dns")
                || lower.contains("failed to lookup")
                || lower.contains("no such host")
                || lower.contains("resolve")
            {
                "DNS resolution failed. Verify the hostname, internet connectivity, or DNS/VPN settings.".to_string()
            } else if lower.contains("connection refused") || lower.contains("10061") {
                "Connection refused by target host. Verify the host/port and ensure the server is running.".to_string()
            } else if lower.contains("tls")
                || lower.contains("ssl")
                || lower.contains("certificate")
                || lower.contains("handshake")
                || lower.contains("webpki")
                || lower.contains("rustls")
            {
                "TLS/SSL handshake failed. The server certificate may be invalid, self-signed, or expired (check verify_ssl).".to_string()
            } else if lower.contains("timed out") || lower.contains("timeout") {
                format!(
                    "TCP connection timed out (timeout_ms={timeout_ms}). Check firewall, proxy, or host reachability."
                )
            } else {
                "Failed to establish connection to the server. Check network connectivity, proxy/VPN, firewall, or target URL.".to_string()
            }
        }
        "Redirect" => format!(
            "Too many redirects or redirect loop (max_redirects={max_redirects})."
        ),
        "Builder" => "Invalid request URL, method, or header format.".to_string(),
        "Request" => "Failed while sending request headers or payload to the server (connection may have been reset or closed).".to_string(),
        "Body" => "Failed while reading or streaming the request/response body.".to_string(),
        "Decode" => "Failed to decode the response body (e.g., invalid encoding or compression).".to_string(),
        _ => "Unexpected network/transport error; inspect cause_chain for underlying OS or TLS details.".to_string(),
    };

    RequestErrorDetails {
        kind,
        cause_chain,
        possible_cause,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_for_log_respects_utf8_boundaries() {
        let s = "hello 🚀 world";
        let truncated = truncate_for_log(s, 8);
        assert!(truncated.contains(&format!("... [truncated, {} total bytes]", s.len())));
        assert!(truncated.starts_with("hello "));
    }

    #[test]
    fn test_format_kv_for_log_redacts_sensitive_headers() {
        let pairs = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Authorization".to_string(), "Bearer secret-token".to_string()),
            ("X-API-Key".to_string(), "my-secret-key".to_string()),
        ];
        let formatted = format_kv_for_log(&pairs, true);
        assert!(formatted.contains("Content-Type=application/json"));
        assert!(formatted.contains("Authorization=***"));
        assert!(formatted.contains("X-API-Key=***"));
        assert!(!formatted.contains("secret-token"));
        assert!(!formatted.contains("my-secret-key"));
    }
}
