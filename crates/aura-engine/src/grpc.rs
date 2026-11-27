//! Pure-Rust HTTP/2 gRPC transport supporting Unary, Server/Client/Bidi Streaming & Server Reflection.

use crate::tls::build_tls_connector;
use anyhow::{anyhow, Result};
use aura_core::grpc::{
    decode_file_descriptor_proto_bytes, decode_protobuf_to_json, decode_server_reflection_response,
    encode_json_to_protobuf, encode_server_reflection_request, frame_grpc_message,
    unframe_grpc_messages, ProtoSchema, ServerReflectionRequestKind, ServerReflectionResult,
};
use aura_core::model::{AuraRequest, AuraResponse, StreamMessage};
use aura_core::templating::TemplateContext;
use bytes::Bytes;
use chrono::Utc;
use http::Request;
use rustls::pki_types::ServerName;
use std::collections::HashMap;
use std::time::Instant;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

async fn open_h2_client(
    url_str: &str,
    verify_ssl: bool,
) -> Result<(h2::client::SendRequest<Bytes>, url::Url)> {
    let normalized = if url_str.starts_with("grpcs://") {
        url_str.replacen("grpcs://", "https://", 1)
    } else if url_str.starts_with("grpc://") {
        url_str.replacen("grpc://", "http://", 1)
    } else {
        url_str.to_string()
    };
    let parsed = url::Url::parse(&normalized)?;
    let is_tls = parsed.scheme().eq_ignore_ascii_case("https");
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("Missing host in gRPC URL"))?
        .to_string();
    let port = parsed.port().unwrap_or(if is_tls { 443 } else { 80 });
    let addr = format!("{host}:{port}");
    let tcp = TcpStream::connect(&addr).await?;

    if is_tls {
        let connector = build_tls_connector(verify_ssl, vec![b"h2".to_vec()])?;
        let server_name = ServerName::try_from(host.clone())
            .map_err(|_| anyhow!("Invalid TLS server name '{host}'"))?;
        let tls_stream = connector.connect(server_name, tcp).await?;
        let (h2_client, conn) = h2::client::handshake(tls_stream).await?;
        tokio::spawn(async move {
            let _ = conn.await;
        });
        Ok((h2_client, parsed))
    } else {
        let (h2_client, conn) = h2::client::handshake(tcp).await?;
        tokio::spawn(async move {
            let _ = conn.await;
        });
        Ok((h2_client, parsed))
    }
}

pub async fn reflect_grpc_services(
    url_str: &str,
    metadata: &[(String, String)],
    verify_ssl: bool,
) -> Result<ProtoSchema> {
    let (mut client, parsed) = open_h2_client(url_str, verify_ssl).await?;
    let authority = match parsed.port() {
        Some(p) => format!("{}:{p}", parsed.host_str().unwrap_or("")),
        None => parsed.host_str().unwrap_or("").to_string(),
    };

    let mut schema = ProtoSchema::default();
    let reflection_paths = [
        "/grpc.reflection.v1.ServerReflection/ServerReflectionInfo",
        "/grpc.reflection.v1alpha.ServerReflection/ServerReflectionInfo",
    ];

    let mut last_err = anyhow!("gRPC server reflection unavailable");
    for refl_path in reflection_paths {
        client = match client.ready().await {
            Ok(c) => c,
            Err(e) => {
                last_err = anyhow!("HTTP/2 readiness error: {e}");
                break;
            }
        };
        let uri = format!("{}://{}{}", parsed.scheme(), authority, refl_path);
        let mut req_builder = Request::post(uri)
            .header("content-type", "application/grpc")
            .header("te", "trailers");
        for (k, v) in metadata {
            req_builder = req_builder.header(k.as_str(), v.as_str());
        }
        let request = req_builder.body(())?;
        let (resp_fut, mut send_stream) = client.send_request(request, false)?;

        let list_req =
            frame_grpc_message(&encode_server_reflection_request(ServerReflectionRequestKind::ListServices));
        send_stream.send_data(Bytes::from(list_req), false)?;

        let resp = match resp_fut.await {
            Ok(r) => r,
            Err(e) => {
                last_err = anyhow!("Reflection RPC failed: {e}");
                continue;
            }
        };
        let mut body_stream = resp.into_body();
        let first_chunk = match body_stream.data().await {
            Some(Ok(chunk)) => chunk,
            Some(Err(e)) => {
                last_err = anyhow!("Reflection stream read error: {e}");
                continue;
            }
            None => {
                last_err = anyhow!("Empty reflection response on {refl_path}");
                continue;
            }
        };
        let _ = body_stream.flow_control().release_capacity(first_chunk.len());
        let msgs = unframe_grpc_messages(&first_chunk)?;
        let Some(first_msg) = msgs.first() else {
            continue;
        };

        let service_names = match decode_server_reflection_response(first_msg)? {
            ServerReflectionResult::ListServices(names) => names,
            ServerReflectionResult::Error(msg) => {
                last_err = anyhow!("Server reflection error: {msg}");
                continue;
            }
            _ => Vec::new(),
        };

        for svc_name in service_names {
            if svc_name.starts_with("grpc.reflection.") {
                continue;
            }
            let sym_req = frame_grpc_message(&encode_server_reflection_request(
                ServerReflectionRequestKind::FileContainingSymbol(svc_name),
            ));
            send_stream.send_data(Bytes::from(sym_req), false)?;
            if let Some(Ok(chunk)) = body_stream.data().await {
                let _ = body_stream.flow_control().release_capacity(chunk.len());
                if let Ok(frames) = unframe_grpc_messages(&chunk) {
                    for f in frames {
                        if let Ok(ServerReflectionResult::FileDescriptorBytes(desc_list)) =
                            decode_server_reflection_response(&f)
                        {
                            for fd in desc_list {
                                if let Ok(part) = decode_file_descriptor_proto_bytes(&fd) {
                                    schema.merge(part);
                                }
                            }
                        }
                    }
                }
            }
        }
        let _ = send_stream.send_data(Bytes::new(), true);
        if !schema.services.is_empty() {
            return Ok(schema);
        }
    }

    Err(last_err)
}

pub async fn execute_grpc_call(
    req: &AuraRequest,
    schema: Option<&ProtoSchema>,
    context: &TemplateContext,
    stream_tx: Option<mpsc::UnboundedSender<StreamMessage>>,
) -> Result<AuraResponse> {
    let rendered_url = context.render(&req.url);
    let grpc_cfg = req
        .grpc
        .as_ref()
        .ok_or_else(|| anyhow!("No gRPC service/method configured on this request"))?;

    let service = context.render(&grpc_cfg.service);
    let method = context.render(&grpc_cfg.method);
    if service.trim().is_empty() || method.trim().is_empty() {
        return Err(anyhow!("gRPC requires both a service name and method name"));
    }
    let path = format!("/{}/{}", service.trim(), method.trim());

    let method_meta = schema.and_then(|s| {
        s.services
            .iter()
            .find(|svc| svc.full_name == service || svc.name == service)
            .and_then(|svc| svc.methods.iter().find(|m| m.name == method))
    });

    let rendered_payload = context.render(&grpc_cfg.payload);
    let json_messages: Vec<serde_json::Value> = if rendered_payload.trim().is_empty() {
        vec![serde_json::json!({})]
    } else if let Ok(val) = serde_json::from_str::<serde_json::Value>(&rendered_payload) {
        if let Some(arr) = val.as_array() {
            arr.clone()
        } else {
            vec![val]
        }
    } else {
        // Support newline-delimited JSON objects for client/bidi streaming
        let mut list = Vec::new();
        for line in rendered_payload.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                list.push(serde_json::from_str::<serde_json::Value>(trimmed)?);
            }
        }
        if list.is_empty() {
            vec![serde_json::json!({})]
        } else {
            list
        }
    };

    let mut framed_payloads = Vec::with_capacity(json_messages.len());
    for jm in &json_messages {
        let raw_bytes = if let (Some(s), Some(mm)) = (schema, method_meta) {
            encode_json_to_protobuf(s, &mm.input_type, jm)?
        } else {
            serde_json::to_vec(jm)?
        };
        framed_payloads.push(frame_grpc_message(&raw_bytes));
    }

    let start = Instant::now();
    let (mut client, parsed) = open_h2_client(&rendered_url, req.settings.verify_ssl).await?;
    client = client.ready().await?;
    let authority = match parsed.port() {
        Some(p) => format!("{}:{p}", parsed.host_str().unwrap_or("")),
        None => parsed.host_str().unwrap_or("").to_string(),
    };
    let uri = format!("{}://{}{}", parsed.scheme(), authority, path);

    let mut builder = Request::post(uri)
        .header("content-type", "application/grpc")
        .header("te", "trailers");
    for h in &req.headers {
        if h.enabled {
            builder = builder.header(
                context.render(&h.key).as_str(),
                context.render(&h.value).as_str(),
            );
        }
    }
    let http_req = builder.body(())?;
    let (resp_fut, mut send_stream) = client.send_request(http_req, false)?;

    let total_out = framed_payloads.len();
    for (idx, frame) in framed_payloads.into_iter().enumerate() {
        let is_last = idx + 1 == total_out;
        if let Some(ref tx) = stream_tx {
            let _ = tx.send(StreamMessage {
                direction: "out".to_string(),
                payload: serde_json::to_string_pretty(&json_messages[idx]).unwrap_or_default(),
                timestamp: Utc::now(),
            });
        }
        send_stream.send_data(Bytes::from(frame), is_last)?;
    }

    let resp = resp_fut.await?;
    let http_status = resp.status().as_u16();
    let mut resp_headers = HashMap::new();
    for (k, v) in resp.headers() {
        if let Ok(vs) = v.to_str() {
            resp_headers.insert(k.to_string(), vs.to_string());
        }
    }

    let mut body_stream = resp.into_body();
    let mut raw_bytes = Vec::new();
    while let Some(chunk) = body_stream.data().await {
        let data = chunk?;
        let _ = body_stream.flow_control().release_capacity(data.len());
        raw_bytes.extend_from_slice(&data);
    }
    if let Ok(Some(trailers)) = body_stream.trailers().await {
        for (k, v) in &trailers {
            if let Ok(vs) = v.to_str() {
                resp_headers.insert(k.to_string(), vs.to_string());
            }
        }
    }

    let mut decoded_messages = Vec::new();
    let mut stream_messages = Vec::new();
    if !raw_bytes.is_empty() {
        if let Ok(frames) = unframe_grpc_messages(&raw_bytes) {
            for frame in frames {
                let decoded_str = if let (Some(s), Some(mm)) = (schema, method_meta) {
                    decode_protobuf_to_json(s, &mm.output_type, &frame)
                        .and_then(|v| Ok(serde_json::to_string_pretty(&v)?))
                        .unwrap_or_else(|_| String::from_utf8_lossy(&frame).to_string())
                } else {
                    String::from_utf8_lossy(&frame).to_string()
                };
                if let Some(ref tx) = stream_tx {
                    let _ = tx.send(StreamMessage {
                        direction: "in".to_string(),
                        payload: decoded_str.clone(),
                        timestamp: Utc::now(),
                    });
                }
                stream_messages.push(StreamMessage {
                    direction: "in".to_string(),
                    payload: decoded_str.clone(),
                    timestamp: Utc::now(),
                });
                decoded_messages.push(decoded_str);
            }
        }
    }

    let grpc_status = resp_headers
        .get("grpc-status")
        .map(String::as_str)
        .unwrap_or("0");
    let status_text = if grpc_status == "0" {
        "gRPC OK".to_string()
    } else {
        let msg = resp_headers
            .get("grpc-message")
            .map(String::as_str)
            .unwrap_or("Error");
        format!("gRPC {grpc_status}: {msg}")
    };

    let body = if decoded_messages.len() <= 1 {
        decoded_messages.into_iter().next().unwrap_or_default()
    } else {
        format!("[\n{}\n]", decoded_messages.join(",\n"))
    };

    Ok(AuraResponse {
        status: http_status,
        status_text,
        duration_ms: start.elapsed().as_millis() as u64,
        size_bytes: raw_bytes.len(),
        headers: resp_headers,
        body,
        timestamp: Utc::now(),
        assertions: Vec::new(),
        console_logs: Vec::new(),
        script_errors: Vec::new(),
        sse_events: Vec::new(),
        stream_messages,
        timing: None,
        captured_cookies: Vec::new(),
        next_request: None,
        stop_chain: false,
        exported_variables: HashMap::new(),
    })
}
