//! Local Mock Server with dynamic response scripting.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[derive(Debug, Clone)]
pub struct MockRoute {
    pub method: String,
    pub path: String,
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

#[derive(Debug, Clone, Default)]
pub struct MockServerConfig {
    pub port: u16,
    pub routes: Arc<parking_lot::RwLock<Vec<MockRoute>>>,
}

pub struct MockServer {
    config: MockServerConfig,
}

impl MockServer {
    pub fn new(port: u16) -> Self {
        Self {
            config: MockServerConfig {
                port,
                routes: Arc::new(parking_lot::RwLock::new(Vec::new())),
            },
        }
    }

    pub fn add_route(&self, route: MockRoute) {
        self.config.routes.write().push(route);
    }

    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.config.port)).await?;
        let routes = Arc::clone(&self.config.routes);

        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let routes_clone = Arc::clone(&routes);
                tokio::spawn(async move {
                    let mut buffer = [0u8; 4096];
                    if let Ok(n) = socket.read(&mut buffer).await {
                        if n > 0 {
                            let req_text = String::from_utf8_lossy(&buffer[..n]);
                            let mut lines = req_text.lines();
                            let first_line = lines.next().unwrap_or("");
                            let parts: Vec<&str> = first_line.split_whitespace().collect();

                            let method = parts.first().copied().unwrap_or("GET");
                            let path = parts.get(1).copied().unwrap_or("/");

                            let (status, body) = {
                                let guard = routes_clone.read();
                                let matched = guard.iter().find(|r| {
                                    r.method.eq_ignore_ascii_case(method) && r.path == path
                                });

                                if let Some(m) = matched {
                                    (m.status, m.body.clone())
                                } else {
                                    (200, "{\"mock\": true, \"status\": \"ok\"}".to_string())
                                }
                            };

                            let resp_str = format!(
                                "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = socket.write_all(resp_str.as_bytes()).await;
                        }
                    }
                });
            }
        });

        Ok(())
    }
}
