//! Pure-Rust .proto parser, dynamic Protobuf wire encoder/decoder,
//! gRPC length-prefixed framing, and FileDescriptorProto / ServerReflection codecs.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
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

fn strip_proto_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut in_str = false;
    while i < chars.len() {
        if in_str {
            out.push(chars[i]);
            if chars[i] == '\\' && i + 1 < chars.len() {
                i += 1;
                out.push(chars[i]);
            } else if chars[i] == '"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if chars[i] == '"' {
            in_str = true;
            out.push('"');
            i += 1;
            continue;
        }
        if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                if chars[i] == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            i += 2;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

pub fn parse_proto_file(source: &str) -> Result<ProtoSchema> {
    let cleaned = strip_proto_comments(source);
    let mut schema = ProtoSchema::default();
    let mut scope_stack: Vec<(&'static str, String)> = Vec::new();
    let mut current_msg: Option<ProtoMessage> = None;
    let mut current_enum: Option<ProtoEnum> = None;
    let mut current_service: Option<ProtoService> = None;
    let mut in_oneof = false;

    let mut pos = 0usize;
    let bytes = cleaned.as_bytes();
    while pos < bytes.len() {
        // Read statement up to ;, {, or }
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }

        if bytes[pos] == b'}' {
            pos += 1;
            if in_oneof {
                in_oneof = false;
                continue;
            }
            if let Some((kind, _)) = scope_stack.pop() {
                match kind {
                    "message" => {
                        if let Some(m) = current_msg.take() {
                            schema.messages.insert(m.full_name.clone(), m);
                        }
                    }
                    "enum" => {
                        if let Some(e) = current_enum.take() {
                            schema.enums.insert(e.name.clone(), e);
                        }
                    }
                    "service" => {
                        if let Some(s) = current_service.take() {
                            schema.services.push(s);
                        }
                    }
                    _ => {}
                }
            }
            continue;
        }

        let start = pos;
        let mut delim = 0u8;
        while pos < bytes.len() {
            let b = bytes[pos];
            if b == b';' || b == b'{' || b == b'}' {
                delim = b;
                break;
            }
            pos += 1;
        }
        let stmt = cleaned[start..pos].trim();
        if delim != 0 && delim != b'}' {
            pos += 1;
        }
        if stmt.is_empty() {
            continue;
        }

        if delim == b'{' {
            let mut parts = stmt.split_whitespace();
            let kw = parts.next().unwrap_or("");
            let name = parts.next().unwrap_or("").trim().to_string();
            match kw {
                "message" => {
                    if let Some(prev) = current_msg.take() {
                        schema.messages.insert(prev.full_name.clone(), prev);
                    }
                    let full_name = match &schema.package {
                        Some(pkg) if !pkg.is_empty() => format!("{pkg}.{name}"),
                        _ => name.clone(),
                    };
                    scope_stack.push(("message", name.clone()));
                    current_msg = Some(ProtoMessage {
                        name,
                        full_name,
                        fields: Vec::new(),
                    });
                }
                "enum" => {
                    scope_stack.push(("enum", name.clone()));
                    current_enum = Some(ProtoEnum {
                        name,
                        values: Vec::new(),
                    });
                }
                "service" => {
                    let full_name = match &schema.package {
                        Some(pkg) if !pkg.is_empty() => format!("{pkg}.{name}"),
                        _ => name.clone(),
                    };
                    scope_stack.push(("service", name.clone()));
                    current_service = Some(ProtoService {
                        name,
                        full_name,
                        methods: Vec::new(),
                    });
                }
                "oneof" => {
                    in_oneof = true;
                }
                _ => {
                    scope_stack.push(("block", name));
                }
            }
            continue;
        }

        // Semicolon statements
        if let Some(rest) = stmt.strip_prefix("package ") {
            schema.package = Some(rest.trim().trim_matches('"').to_string());
        } else if stmt.starts_with("syntax ") || stmt.starts_with("import ") || stmt.starts_with("option ") {
            continue;
        } else if let Some(ref mut svc) = current_service {
            if let Some(method) = parse_rpc_line(stmt, &svc.full_name) {
                svc.methods.push(method);
            }
        } else if let Some(ref mut en) = current_enum {
            if let Some((k, v_str)) = stmt.split_once('=') {
                let val_clean = v_str.split('[').next().unwrap_or(v_str).trim();
                if let Ok(num) = val_clean.parse::<i32>() {
                    en.values.push((k.trim().to_string(), num));
                }
            }
        } else if let Some(ref mut msg) = current_msg {
            if let Some(field) = parse_proto_field_line(stmt) {
                msg.fields.push(field);
            }
        }
    }

    if let Some(m) = current_msg.take() {
        schema.messages.insert(m.full_name.clone(), m);
    }
    if let Some(e) = current_enum.take() {
        schema.enums.insert(e.name.clone(), e);
    }
    if let Some(s) = current_service.take() {
        schema.services.push(s);
    }

    Ok(schema)
}

fn parse_rpc_line(stmt: &str, svc_full_name: &str) -> Option<ProtoMethod> {
    let rest = stmt.strip_prefix("rpc ")?.trim();
    let open_req = rest.find('(')?;
    let name = rest[..open_req].trim().to_string();
    let after_name = &rest[open_req + 1..];
    let close_req = after_name.find(')')?;
    let req_spec = after_name[..close_req].trim();
    let (client_streaming, input_type) = if let Some(stripped) = req_spec.strip_prefix("stream ") {
        (true, stripped.trim().to_string())
    } else {
        (false, req_spec.to_string())
    };

    let after_req = after_name[close_req + 1..].trim();
    let open_resp = after_req.find('(')?;
    let after_open_resp = &after_req[open_resp + 1..];
    let close_resp = after_open_resp.find(')')?;
    let resp_spec = after_open_resp[..close_resp].trim();
    let (server_streaming, output_type) =
        if let Some(stripped) = resp_spec.strip_prefix("stream ") {
            (true, stripped.trim().to_string())
        } else {
            (false, resp_spec.to_string())
        };

    let full_path = format!("/{svc_full_name}/{name}");
    Some(ProtoMethod {
        name,
        input_type,
        output_type,
        client_streaming,
        server_streaming,
        full_path,
    })
}

fn parse_proto_field_line(stmt: &str) -> Option<ProtoField> {
    let without_opts = stmt.split('[').next()?.trim();
    let (left, num_str) = without_opts.split_once('=')?;
    let number: u32 = num_str.trim().parse().ok()?;
    let left_trimmed = left.trim();

    if let Some(map_rest) = left_trimmed.strip_prefix("map<") {
        let close = map_rest.find('>')?;
        let inner = &map_rest[..close];
        let (kt, vt) = inner.split_once(',')?;
        let name = map_rest[close + 1..].trim().to_string();
        return Some(ProtoField {
            name,
            number,
            field_type: "map".to_string(),
            is_repeated: true,
            is_map: true,
            map_key_type: Some(kt.trim().to_string()),
            map_val_type: Some(vt.trim().to_string()),
        });
    }

    let tokens: Vec<&str> = left_trimmed.split_whitespace().collect();
    match tokens.as_slice() {
        [qual, ftype, fname] if *qual == "repeated" || *qual == "optional" || *qual == "required" => {
            Some(ProtoField {
                name: (*fname).to_string(),
                number,
                field_type: (*ftype).to_string(),
                is_repeated: *qual == "repeated",
                is_map: false,
                map_key_type: None,
                map_val_type: None,
            })
        }
        [ftype, fname] => Some(ProtoField {
            name: (*fname).to_string(),
            number,
            field_type: (*ftype).to_string(),
            is_repeated: false,
            is_map: false,
            map_key_type: None,
            map_val_type: None,
        }),
        _ => None,
    }
}

pub fn sample_json_for_message(schema: &ProtoSchema, message_name: &str) -> Value {
    let mut visited = std::collections::HashSet::new();
    sample_json_inner(schema, message_name, &mut visited)
}

fn sample_json_inner(
    schema: &ProtoSchema,
    message_name: &str,
    visited: &mut std::collections::HashSet<String>,
) -> Value {
    let Some(msg) = schema.resolve_message(message_name) else {
        return Value::Object(Map::new());
    };
    if !visited.insert(msg.full_name.clone()) {
        return Value::Object(Map::new());
    }
    let mut obj = Map::new();
    for f in &msg.fields {
        let val = if f.is_map {
            let mut m = Map::new();
            m.insert("key".to_string(), sample_scalar_value(f.map_val_type.as_deref().unwrap_or("string")));
            Value::Object(m)
        } else {
            let item = if let Some(en) = schema.resolve_enum(&f.field_type) {
                Value::String(
                    en.values
                        .first()
                        .map(|(n, _)| n.clone())
                        .unwrap_or_else(|| "UNKNOWN".to_string()),
                )
            } else if schema.resolve_message(&f.field_type).is_some() {
                sample_json_inner(schema, &f.field_type, visited)
            } else {
                sample_scalar_value(&f.field_type)
            };
            if f.is_repeated {
                Value::Array(vec![item])
            } else {
                item
            }
        };
        obj.insert(f.name.clone(), val);
    }
    visited.remove(&msg.full_name);
    Value::Object(obj)
}

fn sample_scalar_value(ftype: &str) -> Value {
    match ftype {
        "string" => Value::String("example".to_string()),
        "bool" => Value::Bool(false),
        "bytes" => Value::String("".to_string()),
        "float" | "double" => Value::from(0.0),
        "int32" | "int64" | "uint32" | "uint64" | "sint32" | "sint64" | "fixed32"
        | "fixed64" | "sfixed32" | "sfixed64" => Value::from(0),
        _ => Value::String("value".to_string()),
    }
}

pub fn frame_grpc_message(msg: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(5 + msg.len());
    out.push(0u8); // compression flag: uncompressed
    out.extend_from_slice(&(msg.len() as u32).to_be_bytes());
    out.extend_from_slice(msg);
    out
}

pub fn unframe_grpc_messages(framed: &[u8]) -> Result<Vec<Vec<u8>>> {
    let mut messages = Vec::new();
    let mut idx = 0usize;
    while idx < framed.len() {
        if idx + 5 > framed.len() {
            return Err(anyhow!("Incomplete gRPC frame header"));
        }
        let compressed = framed[idx];
        if compressed != 0 {
            return Err(anyhow!("Compressed gRPC frames are not supported"));
        }
        let len = u32::from_be_bytes([framed[idx + 1], framed[idx + 2], framed[idx + 3], framed[idx + 4]])
            as usize;
        idx += 5;
        if idx + len > framed.len() {
            return Err(anyhow!("Truncated gRPC frame payload"));
        }
        messages.push(framed[idx..idx + len].to_vec());
        idx += len;
    }
    Ok(messages)
}

fn write_varint(buf: &mut Vec<u8>, mut val: u64) {
    loop {
        let mut b = (val & 0x7f) as u8;
        val >>= 7;
        if val != 0 {
            b |= 0x80;
        }
        buf.push(b);
        if val == 0 {
            break;
        }
    }
}

fn read_varint(buf: &[u8], pos: &mut usize) -> Result<u64> {
    let mut result = 0u64;
    let mut shift = 0u32;
    while *pos < buf.len() {
        let b = buf[*pos];
        *pos += 1;
        result |= ((b & 0x7f) as u64) << shift;
        if b & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
        if shift >= 64 {
            return Err(anyhow!("Malformed protobuf varint"));
        }
    }
    Err(anyhow!("Unexpected EOF reading varint"))
}

fn zigzag32(n: i32) -> u64 {
    ((n << 1) ^ (n >> 31)) as u32 as u64
}

fn zigzag64(n: i64) -> u64 {
    ((n << 1) ^ (n >> 63)) as u64
}

pub fn encode_json_to_protobuf(
    schema: &ProtoSchema,
    message_name: &str,
    val: &Value,
) -> Result<Vec<u8>> {
    let msg = schema
        .resolve_message(message_name)
        .ok_or_else(|| anyhow!("Message '{message_name}' not found in proto schema"))?;
    let obj = val
        .as_object()
        .ok_or_else(|| anyhow!("Protobuf payload must be a JSON object"))?;

    let mut out = Vec::new();
    for field in &msg.fields {
        let Some(fval) = obj.get(&field.name) else {
            continue;
        };
        if fval.is_null() {
            continue;
        }
        if field.is_map {
            if let Some(map_obj) = fval.as_object() {
                for (mk, mv) in map_obj {
                    let mut entry = Vec::new();
                    encode_typed_field(
                        schema,
                        1,
                        field.map_key_type.as_deref().unwrap_or("string"),
                        &Value::String(mk.clone()),
                        &mut entry,
                    )?;
                    encode_typed_field(
                        schema,
                        2,
                        field.map_val_type.as_deref().unwrap_or("string"),
                        mv,
                        &mut entry,
                    )?;
                    write_varint(&mut out, ((field.number as u64) << 3) | 2);
                    write_varint(&mut out, entry.len() as u64);
                    out.extend_from_slice(&entry);
                }
            }
        } else if field.is_repeated {
            if let Some(arr) = fval.as_array() {
                for item in arr {
                    encode_typed_field(schema, field.number, &field.field_type, item, &mut out)?;
                }
            }
        } else {
            encode_typed_field(schema, field.number, &field.field_type, fval, &mut out)?;
        }
    }
    Ok(out)
}

fn encode_typed_field(
    schema: &ProtoSchema,
    number: u32,
    ftype: &str,
    val: &Value,
    out: &mut Vec<u8>,
) -> Result<()> {
    match ftype {
        "string" => {
            let s = val.as_str().map(ToString::to_string).unwrap_or_else(|| val.to_string());
            write_varint(out, ((number as u64) << 3) | 2);
            write_varint(out, s.len() as u64);
            out.extend_from_slice(s.as_bytes());
        }
        "bytes" => {
            let raw = val.as_str().unwrap_or("");
            let bytes = base64_decode_lenient(raw);
            write_varint(out, ((number as u64) << 3) | 2);
            write_varint(out, bytes.len() as u64);
            out.extend_from_slice(&bytes);
        }
        "bool" => {
            let b = val.as_bool().unwrap_or(false);
            write_varint(out, (number as u64) << 3);
            write_varint(out, if b { 1 } else { 0 });
        }
        "int32" | "int64" => {
            let n = val.as_i64().unwrap_or(0);
            write_varint(out, (number as u64) << 3);
            write_varint(out, n as u64);
        }
        "uint32" | "uint64" => {
            let n = val.as_u64().unwrap_or(0);
            write_varint(out, (number as u64) << 3);
            write_varint(out, n);
        }
        "sint32" => {
            let n = val.as_i64().unwrap_or(0) as i32;
            write_varint(out, (number as u64) << 3);
            write_varint(out, zigzag32(n));
        }
        "sint64" => {
            let n = val.as_i64().unwrap_or(0);
            write_varint(out, (number as u64) << 3);
            write_varint(out, zigzag64(n));
        }
        "fixed32" | "sfixed32" => {
            let n = val.as_i64().unwrap_or(0) as i32;
            write_varint(out, ((number as u64) << 3) | 5);
            out.extend_from_slice(&n.to_le_bytes());
        }
        "float" => {
            let n = val.as_f64().unwrap_or(0.0) as f32;
            write_varint(out, ((number as u64) << 3) | 5);
            out.extend_from_slice(&n.to_le_bytes());
        }
        "fixed64" | "sfixed64" => {
            let n = val.as_i64().unwrap_or(0);
            write_varint(out, ((number as u64) << 3) | 1);
            out.extend_from_slice(&n.to_le_bytes());
        }
        "double" => {
            let n = val.as_f64().unwrap_or(0.0);
            write_varint(out, ((number as u64) << 3) | 1);
            out.extend_from_slice(&n.to_le_bytes());
        }
        _ => {
            if let Some(en) = schema.resolve_enum(ftype) {
                let enum_num = if let Some(n) = val.as_i64() {
                    n as i32
                } else if let Some(s) = val.as_str() {
                    en.values
                        .iter()
                        .find(|(name, _)| name == s)
                        .map(|(_, num)| *num)
                        .unwrap_or(0)
                } else {
                    0
                };
                write_varint(out, (number as u64) << 3);
                write_varint(out, enum_num as u64);
            } else if schema.resolve_message(ftype).is_some() {
                let nested = encode_json_to_protobuf(schema, ftype, val)?;
                write_varint(out, ((number as u64) << 3) | 2);
                write_varint(out, nested.len() as u64);
                out.extend_from_slice(&nested);
            }
        }
    }
    Ok(())
}

fn base64_decode_lenient(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

pub fn decode_protobuf_to_json(
    schema: &ProtoSchema,
    message_name: &str,
    bytes: &[u8],
) -> Result<Value> {
    let msg = schema
        .resolve_message(message_name)
        .ok_or_else(|| anyhow!("Message '{message_name}' not found in proto schema"))?;

    let mut obj = Map::new();
    let mut pos = 0usize;
    while pos < bytes.len() {
        let tag = read_varint(bytes, &mut pos)?;
        let field_num = (tag >> 3) as u32;
        let wire_type = (tag & 0x7) as u8;
        let field_opt = msg.fields.iter().find(|f| f.number == field_num);

        let raw_val = match wire_type {
            0 => {
                let v = read_varint(bytes, &mut pos)?;
                match field_opt.map(|f| f.field_type.as_str()) {
                    Some("bool") => Value::Bool(v != 0),
                    Some("sint32") => {
                        let u = v as u32;
                        Value::from(((u >> 1) as i32) ^ (-((u & 1) as i32)))
                    }
                    Some("sint64") => {
                        Value::from(((v >> 1) as i64) ^ (-((v & 1) as i64)))
                    }
                    Some("int32") => Value::from(v as i32),
                    Some("int64") => Value::from(v as i64),
                    Some(t) => {
                        if let Some(en) = schema.resolve_enum(t) {
                            en.values
                                .iter()
                                .find(|(_, num)| *num == v as i32)
                                .map(|(name, _)| Value::String(name.clone()))
                                .unwrap_or_else(|| Value::from(v as i32))
                        } else {
                            Value::from(v)
                        }
                    }
                    None => Value::from(v),
                }
            }
            1 => {
                if pos + 8 > bytes.len() {
                    return Err(anyhow!("Truncated 64-bit protobuf field"));
                }
                let arr: [u8; 8] = bytes[pos..pos + 8].try_into().unwrap();
                pos += 8;
                match field_opt.map(|f| f.field_type.as_str()) {
                    Some("double") => Value::from(f64::from_le_bytes(arr)),
                    _ => Value::from(i64::from_le_bytes(arr)),
                }
            }
            2 => {
                let len = read_varint(bytes, &mut pos)? as usize;
                if pos + len > bytes.len() {
                    return Err(anyhow!("Truncated length-delimited protobuf field"));
                }
                let slice = &bytes[pos..pos + len];
                pos += len;
                match field_opt {
                    Some(f) if f.is_map => {
                        if let Some((mk, mv)) = decode_map_entry(schema, f, slice)? {
                            let entry_map = obj
                                .entry(f.name.clone())
                                .or_insert_with(|| Value::Object(Map::new()));
                            if let Value::Object(m) = entry_map {
                                m.insert(mk, mv);
                            }
                            continue;
                        }
                        Value::Null
                    }
                    Some(f) if f.field_type == "string" => {
                        Value::String(String::from_utf8_lossy(slice).to_string())
                    }
                    Some(f) if f.field_type == "bytes" => {
                        Value::String(String::from_utf8_lossy(slice).to_string())
                    }
                    Some(f) if schema.resolve_message(&f.field_type).is_some() => {
                        decode_protobuf_to_json(schema, &f.field_type, slice)?
                    }
                    _ => Value::String(String::from_utf8_lossy(slice).to_string()),
                }
            }
            5 => {
                if pos + 4 > bytes.len() {
                    return Err(anyhow!("Truncated 32-bit protobuf field"));
                }
                let arr: [u8; 4] = bytes[pos..pos + 4].try_into().unwrap();
                pos += 4;
                match field_opt.map(|f| f.field_type.as_str()) {
                    Some("float") => Value::from(f32::from_le_bytes(arr)),
                    _ => Value::from(i32::from_le_bytes(arr)),
                }
            }
            other => return Err(anyhow!("Unsupported protobuf wire type {other}")),
        };

        if let Some(f) = field_opt {
            if f.is_repeated {
                let arr = obj
                    .entry(f.name.clone())
                    .or_insert_with(|| Value::Array(Vec::new()));
                if let Value::Array(vec) = arr {
                    vec.push(raw_val);
                }
            } else {
                obj.insert(f.name.clone(), raw_val);
            }
        } else {
            obj.insert(format!("field_{field_num}"), raw_val);
        }
    }

    Ok(Value::Object(obj))
}

fn decode_map_entry(
    schema: &ProtoSchema,
    field: &ProtoField,
    bytes: &[u8],
) -> Result<Option<(String, Value)>> {
    let mut k = None;
    let mut v = None;
    let mut pos = 0usize;
    while pos < bytes.len() {
        let tag = read_varint(bytes, &mut pos)?;
        let fnum = (tag >> 3) as u32;
        let wire = (tag & 0x7) as u8;
        match (fnum, wire) {
            (1, 2) => {
                let len = read_varint(bytes, &mut pos)? as usize;
                let s = String::from_utf8_lossy(&bytes[pos..pos + len]).to_string();
                pos += len;
                k = Some(s);
            }
            (2, 2) => {
                let len = read_varint(bytes, &mut pos)? as usize;
                let slice = &bytes[pos..pos + len];
                pos += len;
                let vt = field.map_val_type.as_deref().unwrap_or("string");
                if schema.resolve_message(vt).is_some() {
                    v = Some(decode_protobuf_to_json(schema, vt, slice)?);
                } else {
                    v = Some(Value::String(String::from_utf8_lossy(slice).to_string()));
                }
            }
            (2, 0) => {
                let n = read_varint(bytes, &mut pos)?;
                v = Some(Value::from(n as i64));
            }
            (_, 0) => {
                let _ = read_varint(bytes, &mut pos)?;
            }
            (_, 2) => {
                let len = read_varint(bytes, &mut pos)? as usize;
                pos += len.min(bytes.len() - pos);
            }
            _ => break,
        }
    }
    Ok(k.map(|key| (key, v.unwrap_or(Value::Null))))
}

// --- Server Reflection & FileDescriptorProto Wire Support ---

#[derive(Debug, Clone)]
pub enum ServerReflectionRequestKind {
    ListServices,
    FileContainingSymbol(String),
}

#[derive(Debug, Clone)]
pub enum ServerReflectionResult {
    ListServices(Vec<String>),
    FileDescriptorBytes(Vec<Vec<u8>>),
    Error(String),
}

pub fn encode_server_reflection_request(kind: ServerReflectionRequestKind) -> Vec<u8> {
    // grpc.reflection.v1(alpha).ServerReflectionRequest:
    // field 1 (host: string) = ""
    // field 4 (file_containing_symbol: string)
    // field 7 (list_services: string)
    let mut buf = Vec::new();
    match kind {
        ServerReflectionRequestKind::ListServices => {
            write_varint(&mut buf, (7 << 3) | 2);
            write_varint(&mut buf, 0);
        }
        ServerReflectionRequestKind::FileContainingSymbol(sym) => {
            write_varint(&mut buf, (4 << 3) | 2);
            write_varint(&mut buf, sym.len() as u64);
            buf.extend_from_slice(sym.as_bytes());
        }
    }
    buf
}

fn skip_wire_field(bytes: &[u8], pos: &mut usize, wire_type: u8) -> Result<()> {
    match wire_type {
        0 => {
            let _ = read_varint(bytes, pos)?;
        }
        1 => *pos = (*pos + 8).min(bytes.len()),
        2 => {
            let len = read_varint(bytes, pos)? as usize;
            *pos = (*pos + len).min(bytes.len());
        }
        5 => *pos = (*pos + 4).min(bytes.len()),
        _ => *pos = bytes.len(),
    }
    Ok(())
}

pub fn decode_server_reflection_response(bytes: &[u8]) -> Result<ServerReflectionResult> {
    let mut pos = 0usize;
    let mut services = Vec::new();
    let mut files = Vec::new();
    let mut err_msg = None;

    while pos < bytes.len() {
        let tag = read_varint(bytes, &mut pos)?;
        let fnum = (tag >> 3) as u32;
        let wire = (tag & 0x7) as u8;
        match (fnum, wire) {
            (4, 2) => {
                // file_descriptor_response (repeated bytes file_descriptor_proto = 1)
                let len = read_varint(bytes, &mut pos)? as usize;
                let sub = &bytes[pos..(pos + len).min(bytes.len())];
                pos += len.min(bytes.len() - pos);
                let mut sp = 0usize;
                while sp < sub.len() {
                    let stag = read_varint(sub, &mut sp)?;
                    let sf = (stag >> 3) as u32;
                    let sw = (stag & 0x7) as u8;
                    if sf == 1 && sw == 2 {
                        let flen = read_varint(sub, &mut sp)? as usize;
                        files.push(sub[sp..(sp + flen).min(sub.len())].to_vec());
                        sp += flen.min(sub.len() - sp);
                    } else {
                        skip_wire_field(sub, &mut sp, sw)?;
                    }
                }
            }
            (6, 2) => {
                // list_services_response (repeated ServiceResponse service = 1 -> string name = 1)
                let len = read_varint(bytes, &mut pos)? as usize;
                let sub = &bytes[pos..(pos + len).min(bytes.len())];
                pos += len.min(bytes.len() - pos);
                let mut sp = 0usize;
                while sp < sub.len() {
                    let stag = read_varint(sub, &mut sp)?;
                    let sf = (stag >> 3) as u32;
                    let sw = (stag & 0x7) as u8;
                    if sf == 1 && sw == 2 {
                        let slen = read_varint(sub, &mut sp)? as usize;
                        let svc_msg = &sub[sp..(sp + slen).min(sub.len())];
                        sp += slen.min(sub.len() - sp);
                        let mut mp = 0usize;
                        while mp < svc_msg.len() {
                            let mtag = read_varint(svc_msg, &mut mp)?;
                            if (mtag >> 3) == 1 && (mtag & 0x7) == 2 {
                                let nlen = read_varint(svc_msg, &mut mp)? as usize;
                                services.push(
                                    String::from_utf8_lossy(
                                        &svc_msg[mp..(mp + nlen).min(svc_msg.len())],
                                    )
                                    .to_string(),
                                );
                                mp += nlen.min(svc_msg.len() - mp);
                            } else {
                                skip_wire_field(svc_msg, &mut mp, (mtag & 0x7) as u8)?;
                            }
                        }
                    } else {
                        skip_wire_field(sub, &mut sp, sw)?;
                    }
                }
            }
            (7, 2) => {
                // error_response (int32 error_code = 1, string error_message = 2)
                let len = read_varint(bytes, &mut pos)? as usize;
                let sub = &bytes[pos..(pos + len).min(bytes.len())];
                pos += len.min(bytes.len() - pos);
                let mut sp = 0usize;
                while sp < sub.len() {
                    let stag = read_varint(sub, &mut sp)?;
                    if (stag >> 3) == 2 && (stag & 0x7) == 2 {
                        let mlen = read_varint(sub, &mut sp)? as usize;
                        err_msg = Some(
                            String::from_utf8_lossy(&sub[sp..(sp + mlen).min(sub.len())])
                                .to_string(),
                        );
                        sp += mlen.min(sub.len() - sp);
                    } else {
                        skip_wire_field(sub, &mut sp, (stag & 0x7) as u8)?;
                    }
                }
            }
            _ => skip_wire_field(bytes, &mut pos, wire)?,
        }
    }

    if let Some(msg) = err_msg {
        return Ok(ServerReflectionResult::Error(msg));
    }
    if !files.is_empty() {
        return Ok(ServerReflectionResult::FileDescriptorBytes(files));
    }
    Ok(ServerReflectionResult::ListServices(services))
}

pub fn decode_file_descriptor_proto_bytes(bytes: &[u8]) -> Result<ProtoSchema> {
    let mut schema = ProtoSchema::default();
    let mut raw_messages = Vec::new();
    let mut raw_services = Vec::new();
    let mut raw_enums = Vec::new();

    let mut pos = 0usize;
    while pos < bytes.len() {
        let tag = read_varint(bytes, &mut pos)?;
        let fnum = (tag >> 3) as u32;
        let wire = (tag & 0x7) as u8;
        match (fnum, wire) {
            (2, 2) => {
                let len = read_varint(bytes, &mut pos)? as usize;
                schema.package = Some(
                    String::from_utf8_lossy(&bytes[pos..(pos + len).min(bytes.len())]).to_string(),
                );
                pos += len.min(bytes.len() - pos);
            }
            (4, 2) => {
                let len = read_varint(bytes, &mut pos)? as usize;
                raw_messages.push(bytes[pos..(pos + len).min(bytes.len())].to_vec());
                pos += len.min(bytes.len() - pos);
            }
            (5, 2) => {
                let len = read_varint(bytes, &mut pos)? as usize;
                raw_enums.push(bytes[pos..(pos + len).min(bytes.len())].to_vec());
                pos += len.min(bytes.len() - pos);
            }
            (6, 2) => {
                let len = read_varint(bytes, &mut pos)? as usize;
                raw_services.push(bytes[pos..(pos + len).min(bytes.len())].to_vec());
                pos += len.min(bytes.len() - pos);
            }
            _ => skip_wire_field(bytes, &mut pos, wire)?,
        }
    }

    let pkg = schema.package.clone().unwrap_or_default();
    for raw_e in raw_enums {
        if let Ok(en) = decode_descriptor_enum(&raw_e) {
            schema.enums.insert(en.name.clone(), en);
        }
    }
    for raw_m in raw_messages {
        if let Ok(msg) = decode_descriptor_message(&raw_m, &pkg) {
            schema.messages.insert(msg.full_name.clone(), msg);
        }
    }
    for raw_s in raw_services {
        if let Ok(svc) = decode_descriptor_service(&raw_s, &pkg) {
            schema.services.push(svc);
        }
    }
    Ok(schema)
}

fn descriptor_type_to_str(t: i32, type_name: Option<String>) -> String {
    match t {
        1 => "double".to_string(),
        2 => "float".to_string(),
        3 => "int64".to_string(),
        4 => "uint64".to_string(),
        5 => "int32".to_string(),
        6 => "fixed64".to_string(),
        7 => "fixed32".to_string(),
        8 => "bool".to_string(),
        9 => "string".to_string(),
        11 | 14 => type_name
            .unwrap_or_else(|| "Message".to_string())
            .trim_start_matches('.')
            .to_string(),
        12 => "bytes".to_string(),
        13 => "uint32".to_string(),
        15 => "sfixed32".to_string(),
        16 => "sfixed64".to_string(),
        17 => "sint32".to_string(),
        18 => "sint64".to_string(),
        _ => "string".to_string(),
    }
}

fn decode_descriptor_enum(bytes: &[u8]) -> Result<ProtoEnum> {
    let mut name = String::new();
    let mut values = Vec::new();
    let mut pos = 0usize;
    while pos < bytes.len() {
        let tag = read_varint(bytes, &mut pos)?;
        let fnum = (tag >> 3) as u32;
        let wire = (tag & 0x7) as u8;
        match (fnum, wire) {
            (1, 2) => {
                let len = read_varint(bytes, &mut pos)? as usize;
                name = String::from_utf8_lossy(&bytes[pos..(pos + len).min(bytes.len())])
                    .to_string();
                pos += len.min(bytes.len() - pos);
            }
            (2, 2) => {
                let len = read_varint(bytes, &mut pos)? as usize;
                let sub = &bytes[pos..(pos + len).min(bytes.len())];
                pos += len.min(bytes.len() - pos);
                let mut vname = String::new();
                let mut vnum = 0i32;
                let mut sp = 0usize;
                while sp < sub.len() {
                    let stag = read_varint(sub, &mut sp)?;
                    match (stag >> 3, stag & 0x7) {
                        (1, 2) => {
                            let l = read_varint(sub, &mut sp)? as usize;
                            vname = String::from_utf8_lossy(&sub[sp..(sp + l).min(sub.len())])
                                .to_string();
                            sp += l.min(sub.len() - sp);
                        }
                        (2, 0) => vnum = read_varint(sub, &mut sp)? as i32,
                        (_, w) => skip_wire_field(sub, &mut sp, w as u8)?,
                    }
                }
                values.push((vname, vnum));
            }
            _ => skip_wire_field(bytes, &mut pos, wire)?,
        }
    }
    Ok(ProtoEnum { name, values })
}

fn decode_descriptor_message(bytes: &[u8], pkg: &str) -> Result<ProtoMessage> {
    let mut name = String::new();
    let mut fields = Vec::new();
    let mut pos = 0usize;
    while pos < bytes.len() {
        let tag = read_varint(bytes, &mut pos)?;
        let fnum = (tag >> 3) as u32;
        let wire = (tag & 0x7) as u8;
        match (fnum, wire) {
            (1, 2) => {
                let len = read_varint(bytes, &mut pos)? as usize;
                name = String::from_utf8_lossy(&bytes[pos..(pos + len).min(bytes.len())])
                    .to_string();
                pos += len.min(bytes.len() - pos);
            }
            (2, 2) => {
                let len = read_varint(bytes, &mut pos)? as usize;
                let sub = &bytes[pos..(pos + len).min(bytes.len())];
                pos += len.min(bytes.len() - pos);
                let mut fname = String::new();
                let mut number = 0u32;
                let mut label = 1i32;
                let mut ftype_num = 9i32;
                let mut ftype_name = None;
                let mut sp = 0usize;
                while sp < sub.len() {
                    let stag = read_varint(sub, &mut sp)?;
                    match (stag >> 3, stag & 0x7) {
                        (1, 2) => {
                            let l = read_varint(sub, &mut sp)? as usize;
                            fname = String::from_utf8_lossy(&sub[sp..(sp + l).min(sub.len())])
                                .to_string();
                            sp += l.min(sub.len() - sp);
                        }
                        (3, 0) => number = read_varint(sub, &mut sp)? as u32,
                        (4, 0) => label = read_varint(sub, &mut sp)? as i32,
                        (5, 0) => ftype_num = read_varint(sub, &mut sp)? as i32,
                        (6, 2) => {
                            let l = read_varint(sub, &mut sp)? as usize;
                            ftype_name = Some(
                                String::from_utf8_lossy(&sub[sp..(sp + l).min(sub.len())])
                                    .to_string(),
                            );
                            sp += l.min(sub.len() - sp);
                        }
                        (_, w) => skip_wire_field(sub, &mut sp, w as u8)?,
                    }
                }
                fields.push(ProtoField {
                    name: fname,
                    number,
                    field_type: descriptor_type_to_str(ftype_num, ftype_name),
                    is_repeated: label == 3,
                    is_map: false,
                    map_key_type: None,
                    map_val_type: None,
                });
            }
            _ => skip_wire_field(bytes, &mut pos, wire)?,
        }
    }
    let full_name = if pkg.is_empty() {
        name.clone()
    } else {
        format!("{pkg}.{name}")
    };
    Ok(ProtoMessage {
        name,
        full_name,
        fields,
    })
}

fn decode_descriptor_service(bytes: &[u8], pkg: &str) -> Result<ProtoService> {
    let mut name = String::new();
    let mut methods = Vec::new();
    let mut pos = 0usize;
    let mut temp_methods = Vec::new();
    while pos < bytes.len() {
        let tag = read_varint(bytes, &mut pos)?;
        let fnum = (tag >> 3) as u32;
        let wire = (tag & 0x7) as u8;
        match (fnum, wire) {
            (1, 2) => {
                let len = read_varint(bytes, &mut pos)? as usize;
                name = String::from_utf8_lossy(&bytes[pos..(pos + len).min(bytes.len())])
                    .to_string();
                pos += len.min(bytes.len() - pos);
            }
            (2, 2) => {
                let len = read_varint(bytes, &mut pos)? as usize;
                let sub = &bytes[pos..(pos + len).min(bytes.len())];
                pos += len.min(bytes.len() - pos);
                let mut mname = String::new();
                let mut in_t = String::new();
                let mut out_t = String::new();
                let mut cs = false;
                let mut ss = false;
                let mut sp = 0usize;
                while sp < sub.len() {
                    let stag = read_varint(sub, &mut sp)?;
                    match (stag >> 3, stag & 0x7) {
                        (1, 2) => {
                            let l = read_varint(sub, &mut sp)? as usize;
                            mname = String::from_utf8_lossy(&sub[sp..(sp + l).min(sub.len())])
                                .to_string();
                            sp += l.min(sub.len() - sp);
                        }
                        (2, 2) => {
                            let l = read_varint(sub, &mut sp)? as usize;
                            in_t = String::from_utf8_lossy(&sub[sp..(sp + l).min(sub.len())])
                                .trim_start_matches('.')
                                .to_string();
                            sp += l.min(sub.len() - sp);
                        }
                        (3, 2) => {
                            let l = read_varint(sub, &mut sp)? as usize;
                            out_t = String::from_utf8_lossy(&sub[sp..(sp + l).min(sub.len())])
                                .trim_start_matches('.')
                                .to_string();
                            sp += l.min(sub.len() - sp);
                        }
                        (5, 0) => cs = read_varint(sub, &mut sp)? != 0,
                        (6, 0) => ss = read_varint(sub, &mut sp)? != 0,
                        (_, w) => skip_wire_field(sub, &mut sp, w as u8)?,
                    }
                }
                temp_methods.push((mname, in_t, out_t, cs, ss));
            }
            _ => skip_wire_field(bytes, &mut pos, wire)?,
        }
    }
    let full_name = if pkg.is_empty() {
        name.clone()
    } else {
        format!("{pkg}.{name}")
    };
    for (mname, in_t, out_t, cs, ss) in temp_methods {
        let full_path = format!("/{full_name}/{mname}");
        methods.push(ProtoMethod {
            name: mname,
            input_type: in_t,
            output_type: out_t,
            client_streaming: cs,
            server_streaming: ss,
            full_path,
        });
    }
    Ok(ProtoService {
        name,
        full_name,
        methods,
    })
}
