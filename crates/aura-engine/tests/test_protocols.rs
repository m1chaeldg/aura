//! Loopback integration tests for SSE streaming, WebSockets, and gRPC HTTP/2 transport.

use aura_core::grpc::{
    decode_protobuf_to_json, frame_grpc_message, parse_proto_file, unframe_grpc_messages,
};
use aura_core::model::{AuraRequest, GrpcConfig, HttpMethod, RequestProtocol};
use aura_core::templating::TemplateContext;
use aura_engine::grpc::execute_grpc_call;
use aura_engine::sse::{execute_sse_request, SseStreamParser};
use aura_engine::websocket::{connect_websocket, WsCommand, WsEvent};
use bytes::Bytes;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[test]
fn test_sse_stream_parser_chunks_and_multiline_data() {
    let mut parser = SseStreamParser::new();
    let e1 = parser.push_chunk(": heartbeat\nid: 10\nevent: update\ndata: hello\n");
    assert!(e1.is_empty());
    let e2 = parser.push_chunk("data: world\n\n");
    assert_eq!(e2.len(), 1);
    assert_eq!(e2[0].id.as_deref(), Some("10"));
    assert_eq!(e2[0].event, "update");
    assert_eq!(e2[0].data, "hello\nworld");
}

#[tokio::test]
async fn test_loopback_sse_endpoint_streaming() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((mut sock, _)) = listener.accept().await {
            let mut buf = [0u8; 1024];
            let _ = sock.read(&mut buf).await;
            let body = "id: 1\nevent: tick\ndata: first\n\nid: 2\ndata: second\n\n";
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = sock.write_all(resp.as_bytes()).await;
        }
    });

    let mut req = AuraRequest::new("SSE Test", HttpMethod::GET, format!("http://{addr}/events"));
    req.protocol = RequestProtocol::Sse;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let resp = execute_sse_request(&req, &TemplateContext::new(), Some(tx))
        .await
        .expect("SSE execution");
    assert_eq!(resp.status, 200);
    assert_eq!(resp.sse_events.len(), 2);
    assert_eq!(resp.sse_events[0].event, "tick");
    assert_eq!(resp.sse_events[0].data, "first");
    assert_eq!(resp.sse_events[1].event, "message");
    assert!(rx.recv().await.is_some());
}

#[tokio::test]
async fn test_loopback_websocket_bidirectional_messages() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((mut sock, _)) = listener.accept().await {
            let mut req_buf = Vec::new();
            let mut b = [0u8; 1];
            while sock.read(&mut b).await.unwrap_or(0) > 0 {
                req_buf.push(b[0]);
                if req_buf.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            let req_text = String::from_utf8_lossy(&req_buf);
            let sec_key = req_text
                .lines()
                .find(|l| l.to_ascii_lowercase().starts_with("sec-websocket-key:"))
                .and_then(|l| l.split_once(':'))
                .map(|x| x.1.trim())
                .unwrap_or("");
            let accept = aura_engine::websocket::compute_ws_accept_key(sec_key);
            let handshake = format!(
                "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
            );
            let _ = sock.write_all(handshake.as_bytes()).await;

            // Read one masked text frame from client and echo unmasked text frame back
            let mut hdr = [0u8; 2];
            if sock.read_exact(&mut hdr).await.is_ok() {
                let len = (hdr[1] & 0x7f) as usize;
                let mut mask = [0u8; 4];
                let _ = sock.read_exact(&mut mask).await;
                let mut payload = vec![0u8; len];
                let _ = sock.read_exact(&mut payload).await;
                for (i, byte) in payload.iter_mut().enumerate() {
                    *byte ^= mask[i % 4];
                }
                let reply = format!("echo:{}", String::from_utf8_lossy(&payload));
                let mut out = vec![0x81, reply.len() as u8];
                out.extend_from_slice(reply.as_bytes());
                let _ = sock.write_all(&out).await;
            }
        }
    });

    let (cmd_tx, mut ev_rx) = connect_websocket(&format!("ws://{addr}/ws"), vec![], false)
        .await
        .expect("WS connect");

    match ev_rx.recv().await {
        Some(WsEvent::Connected { .. }) => {}
        other => panic!("Expected Connected, got {other:?}"),
    }

    cmd_tx.send(WsCommand::SendText("ping".to_string())).unwrap();
    match ev_rx.recv().await {
        Some(WsEvent::Message { payload, .. }) => assert_eq!(payload, "echo:ping"),
        other => panic!("Expected echoed WS message, got {other:?}"),
    }
    let _ = cmd_tx.send(WsCommand::Close);
}

#[tokio::test]
async fn test_loopback_grpc_unary_http2_execution() {
    let proto_src = r#"
        syntax = "proto3";
        package echo;
        message EchoReq { string text = 1; }
        message EchoResp { string reply = 1; }
        service EchoService {
            rpc Echo (EchoReq) returns (EchoResp);
        }
    "#;
    let schema = parse_proto_file(proto_src).unwrap();
    let schema_server = schema.clone();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((sock, _)) = listener.accept().await {
            let mut conn = h2::server::handshake(sock).await.unwrap();
            while let Some(Ok((req, mut respond))) = conn.accept().await {
                let mut body = req.into_body();
                let chunk = body.data().await.unwrap().unwrap();
                let frames = unframe_grpc_messages(&chunk).unwrap();
                let decoded =
                    decode_protobuf_to_json(&schema_server, "echo.EchoReq", &frames[0]).unwrap();
                let reply_json = serde_json::json!({
                    "reply": format!("hello {}", decoded["text"].as_str().unwrap_or(""))
                });
                let reply_bytes = aura_core::grpc::encode_json_to_protobuf(
                    &schema_server,
                    "echo.EchoResp",
                    &reply_json,
                )
                .unwrap();
                let resp = http::Response::builder()
                    .status(200)
                    .header("content-type", "application/grpc")
                    .body(())
                    .unwrap();
                let mut send_stream = respond.send_response(resp, false).unwrap();
                send_stream
                    .send_data(Bytes::from(frame_grpc_message(&reply_bytes)), false)
                    .unwrap();
                let mut trailers = http::HeaderMap::new();
                trailers.insert("grpc-status", "0".parse().unwrap());
                send_stream.send_trailers(trailers).unwrap();
            }
        }
    });

    let mut req = AuraRequest::new("gRPC Echo", HttpMethod::POST, format!("http://{addr}"));
    req.protocol = RequestProtocol::Grpc;
    req.grpc = Some(GrpcConfig {
        proto_path: None,
        proto_content: Some(proto_src.to_string()),
        service: "echo.EchoService".to_string(),
        method: "Echo".to_string(),
        use_reflection: false,
        payload: "{\"text\": \"world\"}".to_string(),
    });

    let resp = execute_grpc_call(&req, Some(&schema), &TemplateContext::new(), None)
        .await
        .expect("gRPC unary call");
    assert_eq!(resp.status, 200);
    assert_eq!(resp.status_text, "gRPC OK");
    assert!(resp.body.contains("hello world"));
}
