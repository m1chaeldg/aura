//! Response, assertion, timing, and execution report models.

use super::{SseEvent, StreamMessage};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Test assertion result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestAssertion {
    pub name: String,
    pub passed: bool,
    #[serde(default)]
    pub message: Option<String>,
    pub duration_ms: u64,
}

/// Console log captured during script execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsoleMessage {
    pub level: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Script execution error record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptError {
    pub stage: String,
    pub message: String,
}

/// Phase-level HTTP timing breakdown (all durations in milliseconds).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestTimingBreakdown {
    #[serde(default)]
    pub dns_lookup_ms: u64,
    #[serde(default)]
    pub tcp_connect_ms: u64,
    #[serde(default)]
    pub tls_handshake_ms: u64,
    #[serde(default)]
    pub ttfb_ms: u64,
    #[serde(default)]
    pub content_download_ms: u64,
    #[serde(default)]
    pub total_ms: u64,
}

impl RequestTimingBreakdown {
    pub fn phases(&self) -> [(&'static str, u64); 5] {
        [
            ("DNS Lookup", self.dns_lookup_ms),
            ("TCP Connect", self.tcp_connect_ms),
            ("TLS Handshake", self.tls_handshake_ms),
            ("TTFB", self.ttfb_ms),
            ("Download", self.content_download_ms),
        ]
    }
}

/// HTTP / protocol response from the Aura execution engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuraResponse {
    pub status: u16,
    pub status_text: String,
    pub duration_ms: u64,
    pub size_bytes: usize,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub assertions: Vec<TestAssertion>,
    #[serde(default)]
    pub console_logs: Vec<ConsoleMessage>,
    #[serde(default)]
    pub script_errors: Vec<ScriptError>,
    #[serde(default)]
    pub sse_events: Vec<SseEvent>,
    #[serde(default)]
    pub stream_messages: Vec<StreamMessage>,
    #[serde(default)]
    pub timing: Option<RequestTimingBreakdown>,
    #[serde(default)]
    pub captured_cookies: Vec<crate::cookies::CookieEntry>,
    #[serde(default)]
    pub next_request: Option<String>,
    #[serde(default)]
    pub stop_chain: bool,
    #[serde(default)]
    pub exported_variables: HashMap<String, String>,
}

impl AuraResponse {
    pub fn status_pill_color(&self) -> &'static str {
        match self.status {
            200..=299 => crate::design::colors::STATUS_2XX,
            300..=399 => crate::design::colors::STATUS_3XX,
            400..=499 => crate::design::colors::STATUS_4XX,
            _ => crate::design::colors::STATUS_5XX,
        }
    }

    pub fn formatted_size(&self) -> String {
        if self.size_bytes < 1024 {
            format!("{} B", self.size_bytes)
        } else if self.size_bytes < 1024 * 1024 {
            format!("{:.2} KB", self.size_bytes as f64 / 1024.0)
        } else {
            format!("{:.2} MB", self.size_bytes as f64 / (1024.0 * 1024.0))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult {
    pub name: String,
    pub passed: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReport {
    pub request_name: String,
    pub method: String,
    pub url: String,
    pub status_code: u16,
    pub duration_ms: u64,
    pub response_headers: IndexMap<String, String>,
    pub response_body: String,
    pub assertions: Vec<AssertionResult>,
    pub logs: Vec<String>,
    pub error: Option<String>,
}

impl ExecutionReport {
    pub fn all_passed(&self) -> bool {
        self.error.is_none() && self.assertions.iter().all(|a| a.passed)
    }

    pub fn exit_code(&self) -> i32 {
        if self.error.is_some() {
            return 3;
        }
        if self.assertions.iter().any(|a| !a.passed) {
            1
        } else {
            0
        }
    }
}
