//! gRPC Server Reflection & `FileDescriptorProto` wire-format codecs.

use super::codec::{read_varint, write_varint};
use super::schema::{ProtoEnum, ProtoField, ProtoMessage, ProtoMethod, ProtoSchema, ProtoService};
use anyhow::Result;

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
                let len = read_varint(bytes, &mut pos)? as usize;
                let sub = &bytes[pos..(pos + len).min(bytes.len())];
                pos += len.min(bytes.len() - pos);
                let mut sp = 0usize;
                while sp < sub.len() {
                    let stag = read_varint(sub, &mut sp)?;
                    if (stag >> 3) == 1 && (stag & 0x7) == 2 {
                        let flen = read_varint(sub, &mut sp)? as usize;
                        files.push(sub[sp..(sp + flen).min(sub.len())].to_vec());
                        sp += flen.min(sub.len() - sp);
                    } else {
                        skip_wire_field(sub, &mut sp, (stag & 0x7) as u8)?;
                    }
                }
            }
            (6, 2) => {
                let len = read_varint(bytes, &mut pos)? as usize;
                let sub = &bytes[pos..(pos + len).min(bytes.len())];
                pos += len.min(bytes.len() - pos);
                let mut sp = 0usize;
                while sp < sub.len() {
                    let stag = read_varint(sub, &mut sp)?;
                    if (stag >> 3) == 1 && (stag & 0x7) == 2 {
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
                        skip_wire_field(sub, &mut sp, (stag & 0x7) as u8)?;
                    }
                }
            }
            (7, 2) => {
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
        match ((tag >> 3) as u32, (tag & 0x7) as u8) {
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
            (_, w) => skip_wire_field(bytes, &mut pos, w)?,
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
        match ((tag >> 3) as u32, (tag & 0x7) as u8) {
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
            (_, w) => skip_wire_field(bytes, &mut pos, w)?,
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
        match ((tag >> 3) as u32, (tag & 0x7) as u8) {
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
            (_, w) => skip_wire_field(bytes, &mut pos, w)?,
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
