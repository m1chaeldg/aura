//! Protobuf schema AST types (`ProtoSchema`, `ProtoService`, `ProtoMessage`, `ProtoEnum`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RpcCardinality {
    Unary,
    ServerStreaming,
    ClientStreaming,
    BidiStreaming,
}

impl RpcCardinality {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unary => "Unary",
            Self::ServerStreaming => "Server Streaming",
            Self::ClientStreaming => "Client Streaming",
            Self::BidiStreaming => "Bidi Streaming",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtoField {
    pub name: String,
    pub number: u32,
    pub field_type: String,
    #[serde(default)]
    pub is_repeated: bool,
    #[serde(default)]
    pub is_map: bool,
    #[serde(default)]
    pub map_key_type: Option<String>,
    #[serde(default)]
    pub map_val_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProtoEnum {
    pub name: String,
    pub values: Vec<(String, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProtoMessage {
    pub name: String,
    pub full_name: String,
    pub fields: Vec<ProtoField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtoMethod {
    pub name: String,
    pub input_type: String,
    pub output_type: String,
    pub client_streaming: bool,
    pub server_streaming: bool,
    pub full_path: String,
}

impl ProtoMethod {
    pub fn cardinality(&self) -> RpcCardinality {
        match (self.client_streaming, self.server_streaming) {
            (false, false) => RpcCardinality::Unary,
            (false, true) => RpcCardinality::ServerStreaming,
            (true, false) => RpcCardinality::ClientStreaming,
            (true, true) => RpcCardinality::BidiStreaming,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProtoService {
    pub name: String,
    pub full_name: String,
    pub methods: Vec<ProtoMethod>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProtoSchema {
    pub package: Option<String>,
    pub services: Vec<ProtoService>,
    pub messages: HashMap<String, ProtoMessage>,
    pub enums: HashMap<String, ProtoEnum>,
}

impl ProtoSchema {
    pub fn resolve_message(&self, name: &str) -> Option<&ProtoMessage> {
        let trimmed = name.trim_start_matches('.');
        if let Some(m) = self.messages.get(trimmed) {
            return Some(m);
        }
        if let Some(pkg) = &self.package {
            let qualified = format!("{pkg}.{trimmed}");
            if let Some(m) = self.messages.get(&qualified) {
                return Some(m);
            }
        }
        self.messages
            .values()
            .find(|m| m.name == trimmed || m.full_name.ends_with(&format!(".{trimmed}")))
    }

    pub fn resolve_enum(&self, name: &str) -> Option<&ProtoEnum> {
        let trimmed = name.trim_start_matches('.');
        if let Some(e) = self.enums.get(trimmed) {
            return Some(e);
        }
        if let Some(pkg) = &self.package {
            let qualified = format!("{pkg}.{trimmed}");
            if let Some(e) = self.enums.get(&qualified) {
                return Some(e);
            }
        }
        self.enums.values().find(|e| e.name == trimmed)
    }

    pub fn merge(&mut self, other: ProtoSchema) {
        if self.package.is_none() {
            self.package = other.package;
        }
        for svc in other.services {
            if !self.services.iter().any(|s| s.full_name == svc.full_name) {
                self.services.push(svc);
            }
        }
        self.messages.extend(other.messages);
        self.enums.extend(other.enums);
    }
}
