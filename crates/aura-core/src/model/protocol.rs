//! Transport protocol types and live streaming message models.

use serde::{Deserialize, Serialize};

/// Request transport protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RequestProtocol {
    #[default]
    Http,
    Graphql,
    Websocket,
    Sse,
    Grpc,
    Soap,
}

impl RequestProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Http => "HTTP",
            Self::Graphql => "GraphQL",
            Self::Websocket => "WS",
            Self::Sse => "SSE",
            Self::Grpc => "gRPC",
            Self::Soap => "SOAP",
        }
    }

    pub fn from_str_lenient(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "graphql" | "gql" => Self::Graphql,
            "ws" | "wss" | "websocket" => Self::Websocket,
            "sse" | "event-stream" => Self::Sse,
            "grpc" | "grpcs" => Self::Grpc,
            "soap" | "wsdl" => Self::Soap,
            _ => Self::Http,
        }
    }
}

/// Saved WebSocket message template attached to a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WsMessageTemplate {
    pub id: String,
    pub name: String,
    pub payload: String,
}

/// gRPC call configuration attached to a request.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GrpcConfig {
    #[serde(default)]
    pub proto_path: Option<String>,
    #[serde(default)]
    pub proto_content: Option<String>,
    #[serde(default)]
    pub service: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub use_reflection: bool,
    #[serde(default)]
    pub payload: String,
}

/// Parsed Server-Sent Event item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SseEvent {
    #[serde(default)]
    pub id: Option<String>,
    pub event: String,
    pub data: String,
    #[serde(default)]
    pub retry_ms: Option<u64>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Unified live streaming message (WebSocket or gRPC stream).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamMessage {
    pub direction: String, // "in", "out", "system"
    pub payload: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
