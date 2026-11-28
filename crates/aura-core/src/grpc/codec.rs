//! gRPC length-prefixed framing and dynamic Protobuf wire encoder/decoder.

use super::schema::{ProtoField, ProtoSchema};
use anyhow::{anyhow, Result};
use serde_json::{Map, Value};

pub fn frame_grpc_message(msg: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(5 + msg.len());
    out.push(0u8);
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
        if framed[idx] != 0 {
            return Err(anyhow!("Compressed gRPC frames are not supported"));
        }
        let len = u32::from_be_bytes([
            framed[idx + 1],
            framed[idx + 2],
            framed[idx + 3],
            framed[idx + 4],
        ]) as usize;
        idx += 5;
        if idx + len > framed.len() {
            return Err(anyhow!("Truncated gRPC frame payload"));
        }
        messages.push(framed[idx..idx + len].to_vec());
        idx += len;
    }
    Ok(messages)
}

pub(super) fn write_varint(buf: &mut Vec<u8>, mut val: u64) {
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

pub(super) fn read_varint(buf: &[u8], pos: &mut usize) -> Result<u64> {
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
            let bytes = val.as_str().unwrap_or("").as_bytes();
            write_varint(out, ((number as u64) << 3) | 2);
            write_varint(out, bytes.len() as u64);
            out.extend_from_slice(bytes);
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
                    Some("sint64") => Value::from(((v >> 1) as i64) ^ (-((v & 1) as i64))),
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
                    Some(f) if f.field_type == "string" || f.field_type == "bytes" => {
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
