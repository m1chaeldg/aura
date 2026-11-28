//! Pure-Rust `.proto` text parser and sample JSON payload generator.

use super::schema::{ProtoEnum, ProtoField, ProtoMessage, ProtoMethod, ProtoSchema, ProtoService};
use anyhow::Result;
use serde_json::{Map, Value};

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
                    let full_name = qualify_name(&schema.package, &name);
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
                    let full_name = qualify_name(&schema.package, &name);
                    scope_stack.push(("service", name.clone()));
                    current_service = Some(ProtoService {
                        name,
                        full_name,
                        methods: Vec::new(),
                    });
                }
                "oneof" => in_oneof = true,
                _ => scope_stack.push(("block", name)),
            }
            continue;
        }

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

fn qualify_name(pkg: &Option<String>, name: &str) -> String {
    match pkg {
        Some(p) if !p.is_empty() => format!("{p}.{name}"),
        _ => name.to_string(),
    }
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
            m.insert(
                "key".to_string(),
                sample_scalar_value(f.map_val_type.as_deref().unwrap_or("string")),
            );
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
