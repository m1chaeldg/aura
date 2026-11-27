//! Network Interceptor, Request Rewriter, and Live MITM Capture Proxy Manager.

use crate::model::{AuraRequest, BodyType, HeaderItem, HttpMethod};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterceptorRule {
    pub match_url_pattern: String,
    pub rewrite_url_target: Option<String>,
    pub override_headers: HashMap<String, String>,
}

/// Captured HTTP/HTTPS transaction recorded by the live MITM proxy server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedTransaction {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub method: HttpMethod,
    pub url: String,
    pub request_headers: Vec<(String, String)>,
    #[serde(default)]
    pub request_body: Option<String>,
    pub status: u16,
    pub response_headers: Vec<(String, String)>,
    #[serde(default)]
    pub response_body: Option<String>,
    pub duration_ms: u64,
    pub is_https: bool,
}

impl CapturedTransaction {
    /// Convert captured traffic into an editable `AuraRequest` for collections or tabs.
    pub fn to_aura_request(&self) -> AuraRequest {
        let mut req = AuraRequest::new(
            format!("Captured {} {}", self.method.as_str(), self.url),
            self.method,
            self.url.clone(),
        );
        req.headers = self
            .request_headers
            .iter()
            .filter(|(k, _)| {
                !k.eq_ignore_ascii_case("host")
                    && !k.eq_ignore_ascii_case("content-length")
                    && !k.eq_ignore_ascii_case("proxy-connection")
            })
            .map(|(k, v)| HeaderItem::new(k.clone(), v.clone()))
            .collect();

        if let Some(ref body_str) = self.request_body {
            if !body_str.is_empty() {
                let ct = self
                    .request_headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                    .map(|(_, v)| v.to_ascii_lowercase())
                    .unwrap_or_default();
                req.body = if ct.contains("json") {
                    BodyType::Json(body_str.clone())
                } else if ct.contains("xml") {
                    BodyType::Xml(body_str.clone())
                } else {
                    BodyType::Text(body_str.clone())
                };
            }
        }
        req
    }
}

#[derive(Debug, Clone, Default)]
struct ProxyServerRuntimeState {
    running: bool,
    listen_addr: String,
    ca_cert_pem: String,
}

#[derive(Debug, Clone, Default)]
pub struct InterceptorManager {
    rules: Arc<parking_lot::RwLock<Vec<InterceptorRule>>>,
    captures: Arc<parking_lot::RwLock<Vec<CapturedTransaction>>>,
    server_state: Arc<parking_lot::RwLock<ProxyServerRuntimeState>>,
}

impl InterceptorManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_rule(&self, rule: InterceptorRule) {
        self.rules.write().push(rule);
    }

    pub fn list_rules(&self) -> Vec<InterceptorRule> {
        self.rules.read().clone()
    }

    pub fn remove_rule(&self, idx: usize) {
        let mut guard = self.rules.write();
        if idx < guard.len() {
            guard.remove(idx);
        }
    }

    pub fn clear_rules(&self) {
        self.rules.write().clear();
    }

    pub fn apply_rewrites(&self, url: &str, headers: &mut HashMap<String, String>) -> String {
        let mut final_url = url.to_string();
        let rules = self.rules.read();

        for rule in rules.iter() {
            if !rule.match_url_pattern.is_empty() && final_url.contains(&rule.match_url_pattern) {
                if let Some(ref target) = rule.rewrite_url_target {
                    final_url = final_url.replace(&rule.match_url_pattern, target);
                }
                for (k, v) in &rule.override_headers {
                    headers.insert(k.clone(), v.clone());
                }
            }
        }

        final_url
    }

    pub fn record_capture(&self, tx: CapturedTransaction) {
        let mut guard = self.captures.write();
        guard.push(tx);
        if guard.len() > 500 {
            let overflow = guard.len() - 500;
            guard.drain(0..overflow);
        }
    }

    pub fn list_captures(&self) -> Vec<CapturedTransaction> {
        self.captures.read().clone()
    }

    pub fn clear_captures(&self) {
        self.captures.write().clear();
    }

    pub fn set_server_status(&self, running: bool, listen_addr: String, ca_cert_pem: String) {
        let mut guard = self.server_state.write();
        guard.running = running;
        guard.listen_addr = listen_addr;
        guard.ca_cert_pem = ca_cert_pem;
    }

    pub fn is_running(&self) -> bool {
        self.server_state.read().running
    }

    pub fn listen_addr(&self) -> String {
        self.server_state.read().listen_addr.clone()
    }

    pub fn ca_cert_pem(&self) -> String {
        self.server_state.read().ca_cert_pem.clone()
    }
}
