//! Pure-Rust RFC 6455 WebSocket (`ws://` / `wss://`) persistent connection client.

use crate::tls::build_tls_connector;
use anyhow::{anyhow, Result};
use base64::Engine as _;
use chrono::{DateTime, Utc};
use rustls::pki_types::ServerName;
use sha1_smol::Sha1;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum WsCommand {
    SendText(String),
    SendPing,
    Close,
}

#[derive(Debug, Clone)]
pub enum WsEvent {
    Connected { url: String, timestamp: DateTime<Utc> },
    Message { payload: String, is_binary: bool, timestamp: DateTime<Utc> },
    Closed { reason: String, timestamp: DateTime<Utc> },
    Error(String),
}

pub trait AsyncRw: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> AsyncRw for T {}

fn encode_ws_frame(opcode: u8, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(payload.len() + 14);
    frame.push(0x80 | (opcode & 0x0f)); // FIN + opcode
    let mask_key: [u8; 4] = rand_mask_key();
    let len = payload.len();
    if len <= 125 {
        frame.push(0x80 | (len as u8));
    } else if len <= 65535 {
        frame.push(0x80 | 126);
        frame.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        frame.push(0x80 | 127);
        frame.extend_from_slice(&(len as u64).to_be_bytes());
    }
    frame.extend_from_slice(&mask_key);
    for (i, b) in payload.iter().enumerate() {
        frame.push(b ^ mask_key[i % 4]);
    }
    frame
}

fn rand_mask_key() -> [u8; 4] {
    let u = uuid::Uuid::new_v4();
    let bytes = u.as_bytes();
    [bytes[0], bytes[1], bytes[2], bytes[3]]
}

async fn read_ws_frame<R: AsyncRead + Unpin>(reader: &mut R) -> Result<(u8, Vec<u8>)> {
    let mut hdr = [0u8; 2];
    reader.read_exact(&mut hdr).await?;
    let opcode = hdr[0] & 0x0f;
    let masked = (hdr[1] & 0x80) != 0;
    let mut len = (hdr[1] & 0x7f) as u64;
    if len == 126 {
        let mut ext = [0u8; 2];
        reader.read_exact(&mut ext).await?;
        len = u16::from_be_bytes(ext) as u64;
    } else if len == 127 {
        let mut ext = [0u8; 8];
        reader.read_exact(&mut ext).await?;
        len = u64::from_be_bytes(ext);
    }
    let mask = if masked {
        let mut m = [0u8; 4];
        reader.read_exact(&mut m).await?;
        Some(m)
    } else {
        None
    };
    let mut payload = vec![0u8; len as usize];
    reader.read_exact(&mut payload).await?;
    if let Some(m) = mask {
        for (i, b) in payload.iter_mut().enumerate() {
            *b ^= m[i % 4];
        }
    }
    Ok((opcode, payload))
}

pub fn compute_ws_accept_key(sec_key: &str) -> String {
    const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut sha1 = Sha1::new();
    sha1.update(sec_key.as_bytes());
    sha1.update(GUID.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(sha1.digest().bytes())
}

pub async fn connect_websocket(
    url_str: &str,
    headers: Vec<(String, String)>,
    verify_ssl: bool,
) -> Result<(mpsc::UnboundedSender<WsCommand>, mpsc::UnboundedReceiver<WsEvent>)> {
    let parsed = url::Url::parse(url_str)?;
    let scheme = parsed.scheme().to_ascii_lowercase();
    let is_tls = match scheme.as_str() {
        "wss" | "https" => true,
        "ws" | "http" => false,
        other => return Err(anyhow!("Unsupported WebSocket scheme '{other}' (expected ws:// or wss://)")),
    };
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("Missing host in WebSocket URL"))?
        .to_string();
    let port = parsed.port().unwrap_or(if is_tls { 443 } else { 80 });
    let addr = format!("{host}:{port}");
    let path_and_query = match parsed.query() {
        Some(q) => format!("{}?{q}", parsed.path()),
        None => parsed.path().to_string(),
    };
    let path_and_query = if path_and_query.is_empty() {
        "/".to_string()
    } else {
        path_and_query
    };

    let tcp = TcpStream::connect(&addr).await?;
    let mut stream: Box<dyn AsyncRw> = if is_tls {
        let connector = build_tls_connector(verify_ssl, vec![b"http/1.1".to_vec()])?;
        let server_name = ServerName::try_from(host.clone())
            .map_err(|_| anyhow!("Invalid TLS server name '{host}'"))?;
        Box::new(connector.connect(server_name, tcp).await?)
    } else {
        Box::new(tcp)
    };

    let sec_key =
        base64::engine::general_purpose::STANDARD.encode(uuid::Uuid::new_v4().as_bytes());
    let expected_accept = compute_ws_accept_key(&sec_key);

    let mut req = format!(
        "GET {path_and_query} HTTP/1.1\r\nHost: {host}:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {sec_key}\r\nSec-WebSocket-Version: 13\r\n"
    );
    for (k, v) in headers {
        if !k.eq_ignore_ascii_case("host")
            && !k.eq_ignore_ascii_case("upgrade")
            && !k.eq_ignore_ascii_case("connection")
        {
            req.push_str(&format!("{k}: {v}\r\n"));
        }
    }
    req.push_str("\r\n");
    stream.write_all(req.as_bytes()).await?;

    // Read HTTP 101 response headers
    let mut resp_buf = Vec::with_capacity(1024);
    let mut byte = [0u8; 1];
    loop {
        let n = stream.read(&mut byte).await?;
        if n == 0 {
            return Err(anyhow!("Connection closed during WebSocket handshake"));
        }
        resp_buf.push(byte[0]);
        if resp_buf.ends_with(b"\r\n\r\n") || resp_buf.len() > 16384 {
            break;
        }
    }
    let resp_text = String::from_utf8_lossy(&resp_buf);
    if !resp_text.starts_with("HTTP/1.1 101") && !resp_text.starts_with("HTTP/1.0 101") {
        let first_line = resp_text.lines().next().unwrap_or("Handshake failed");
        return Err(anyhow!("WebSocket upgrade rejected: {first_line}"));
    }
    if let Some(accept_line) = resp_text
        .lines()
        .find(|l| l.to_ascii_lowercase().starts_with("sec-websocket-accept:"))
    {
        let got = accept_line.split_once(':').map(|x| x.1.trim()).unwrap_or("");
        if got != expected_accept {
            return Err(anyhow!("Invalid Sec-WebSocket-Accept from server"));
        }
    }

    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<WsCommand>();
    let (ev_tx, ev_rx) = mpsc::unbounded_channel::<WsEvent>();
    let _ = ev_tx.send(WsEvent::Connected {
        url: url_str.to_string(),
        timestamp: Utc::now(),
    });

    let (mut reader, mut writer) = tokio::io::split(stream);
    let ev_tx_read = ev_tx.clone();
    let cmd_tx_pong = cmd_tx.clone();
    tokio::spawn(async move {
        loop {
            match read_ws_frame(&mut reader).await {
                Ok((opcode, payload)) => match opcode {
                    0x1 | 0x0 => {
                        let _ = ev_tx_read.send(WsEvent::Message {
                            payload: String::from_utf8_lossy(&payload).to_string(),
                            is_binary: false,
                            timestamp: Utc::now(),
                        });
                    }
                    0x2 => {
                        let _ = ev_tx_read.send(WsEvent::Message {
                            payload: format!("[binary {} bytes] {}", payload.len(), base64::engine::general_purpose::STANDARD.encode(&payload)),
                            is_binary: true,
                            timestamp: Utc::now(),
                        });
                    }
                    0x8 => {
                        let reason = if payload.len() >= 2 {
                            String::from_utf8_lossy(&payload[2..]).to_string()
                        } else {
                            "Server closed connection".to_string()
                        };
                        let _ = cmd_tx_pong.send(WsCommand::Close);
                        let _ = ev_tx_read.send(WsEvent::Closed {
                            reason,
                            timestamp: Utc::now(),
                        });
                        break;
                    }
                    0x9 => {
                        let _ = cmd_tx_pong.send(WsCommand::SendPing);
                    }
                    _ => {}
                },
                Err(e) => {
                    let _ = ev_tx_read.send(WsEvent::Closed {
                        reason: e.to_string(),
                        timestamp: Utc::now(),
                    });
                    break;
                }
            }
        }
    });

    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                WsCommand::SendText(msg) => {
                    let frame = encode_ws_frame(0x1, msg.as_bytes());
                    if let Err(e) = writer.write_all(&frame).await {
                        let _ = ev_tx.send(WsEvent::Error(e.to_string()));
                        break;
                    }
                }
                WsCommand::SendPing => {
                    let frame = encode_ws_frame(0xA, &[]);
                    let _ = writer.write_all(&frame).await;
                }
                WsCommand::Close => {
                    let frame = encode_ws_frame(0x8, &1000u16.to_be_bytes());
                    let _ = writer.write_all(&frame).await;
                    let _ = writer.shutdown().await;
                    break;
                }
            }
        }
    });

    Ok((cmd_tx, ev_rx))
}
