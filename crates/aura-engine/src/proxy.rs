//! Outbound proxy resolution (`http://`, `https://`, `socks5://`),
//! proxy authentication, bypass lists, and instrumented TCP/TLS timing tunnel.

use anyhow::{anyhow, Result};
use aura_core::model::{ProxyConfig, ProxyProtocol, RequestTimingBreakdown};
use parking_lot::Mutex;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio_socks::tcp::Socks5Stream;
use tokio_util::sync::CancellationToken;

/// Pick the active proxy configuration (per-request override takes precedence over global),
/// honoring bypass lists (`localhost`, `127.0.0.1`, wildcards, etc.).
pub fn resolve_proxy_for_url<'a>(
    target_url: &str,
    request_proxy: Option<&'a ProxyConfig>,
    global_proxy: Option<&'a ProxyConfig>,
) -> Option<&'a ProxyConfig> {
    let candidate = request_proxy
        .filter(|p| p.enabled && !p.host.trim().is_empty())
        .or_else(|| global_proxy.filter(|p| p.enabled && !p.host.trim().is_empty()))?;

    if candidate.should_bypass(target_url) {
        return None;
    }
    Some(candidate)
}

#[derive(Debug, Default)]
pub struct SharedPhaseTimings {
    pub tcp_connect_ms: u64,
    pub tls_handshake_ms: u64,
}

/// Active local loopback tunnel used either for SOCKS5 proxying or single-connection TCP/TLS phase timing.
pub struct ActiveTunnel {
    pub local_proxy_url: String,
    pub timings: Arc<Mutex<SharedPhaseTimings>>,
    cancel: CancellationToken,
}

impl Drop for ActiveTunnel {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// Apply proxy settings (or a single-connection instrumented loopback tunnel for direct TCP/TLS phase timing)
/// onto a `reqwest::ClientBuilder`.
pub async fn configure_proxy_and_timing(
    mut builder: reqwest::ClientBuilder,
    target_url: &str,
    proxy_cfg: Option<&ProxyConfig>,
) -> Result<(reqwest::ClientBuilder, Option<ActiveTunnel>, u64)> {
    let parsed = url::Url::parse(target_url)
        .map_err(|e| anyhow!("Invalid target URL '{target_url}': {e}"))?;
    let host = parsed.host_str().unwrap_or("localhost").to_string();
    let port = parsed
        .port_or_known_default()
        .unwrap_or(if parsed.scheme() == "https" { 443 } else { 80 });

    if let Some(proxy) = proxy_cfg {
        match proxy.protocol {
            ProxyProtocol::Http | ProxyProtocol::Https => {
                let base_url = format!("{}://{}:{}", proxy.protocol.scheme(), proxy.host.trim(), proxy.port);
                let mut rp = reqwest::Proxy::all(&base_url)
                    .map_err(|e| anyhow!("Invalid proxy URL '{base_url}': {e}"))?;
                if proxy.auth_enabled && !proxy.username.is_empty() {
                    rp = rp.basic_auth(&proxy.username, &proxy.password);
                }
                if !proxy.bypass_list.is_empty() {
                    rp = rp.no_proxy(reqwest::NoProxy::from_string(&proxy.bypass_list.join(",")));
                }
                builder = builder.proxy(rp);
                return Ok((builder, None, 0));
            }
            ProxyProtocol::Socks5 | ProxyProtocol::Socks5h => {
                let dns_start = Instant::now();
                let resolved_target = if matches!(proxy.protocol, ProxyProtocol::Socks5) {
                    resolve_host_addrs(&host, port).await.ok()
                } else {
                    None
                };
                let dns_ms = if matches!(proxy.protocol, ProxyProtocol::Socks5) {
                    dns_start.elapsed().as_millis() as u64
                } else {
                    0
                };
                let tunnel = spawn_local_tunnel(Some(proxy.clone()), resolved_target).await?;
                let rp = reqwest::Proxy::custom({
                    let u = tunnel.local_proxy_url.clone();
                    move |_| Some(u.clone())
                });
                builder = builder.proxy(rp);
                return Ok((builder, Some(tunnel), dns_ms));
            }
        }
    }

    // Direct connection: measure real DNS resolution first, then route through a single
    // non-duplicating local TCP/TLS timing tunnel so `tcp_connect_ms` and `tls_handshake_ms`
    // are measured accurately on the single actual socket without extra probes.
    let dns_start = Instant::now();
    let addrs = resolve_host_addrs(&host, port).await?;
    let dns_ms = if host.parse::<std::net::IpAddr>().is_ok() {
        0
    } else {
        dns_start.elapsed().as_millis() as u64
    };
    builder = builder.resolve_to_addrs(&host, &addrs);

    let tunnel = spawn_local_tunnel(None, Some(addrs)).await?;
    let rp = reqwest::Proxy::custom({
        let u = tunnel.local_proxy_url.clone();
        move |_| Some(u.clone())
    });
    builder = builder.proxy(rp);
    Ok((builder, Some(tunnel), dns_ms))
}

async fn resolve_host_addrs(host: &str, port: u16) -> Result<Vec<SocketAddr>> {
    let list: Vec<SocketAddr> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| anyhow!("DNS lookup failed for '{host}:{port}': {e}"))?
        .collect();
    if list.is_empty() {
        return Err(anyhow!("DNS lookup returned no addresses for '{host}:{port}'"));
    }
    Ok(list)
}

pub fn build_timing_breakdown(
    dns_lookup_ms: u64,
    tunnel: Option<&ActiveTunnel>,
    send_elapsed_ms: u64,
    content_download_ms: u64,
    total_ms: u64,
) -> RequestTimingBreakdown {
    let (tcp_ms, tls_ms) = tunnel
        .map(|t| {
            let g = t.timings.lock();
            (g.tcp_connect_ms, g.tls_handshake_ms)
        })
        .unwrap_or((0, 0));

    let ttfb_ms = send_elapsed_ms.saturating_sub(tcp_ms.saturating_add(tls_ms));
    RequestTimingBreakdown {
        dns_lookup_ms,
        tcp_connect_ms: tcp_ms,
        tls_handshake_ms: tls_ms,
        ttfb_ms,
        content_download_ms,
        total_ms,
    }
}

async fn spawn_local_tunnel(
    socks_proxy: Option<ProxyConfig>,
    resolved_addrs: Option<Vec<SocketAddr>>,
) -> Result<ActiveTunnel> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let local_addr = listener.local_addr()?;
    let cancel = CancellationToken::new();
    let cancel_task = cancel.clone();
    let timings = Arc::new(Mutex::new(SharedPhaseTimings::default()));
    let timings_task = Arc::clone(&timings);

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel_task.cancelled() => break,
                res = listener.accept() => {
                    let Ok((client_stream, _)) = res else { break; };
                    let socks = socks_proxy.clone();
                    let addrs = resolved_addrs.clone();
                    let timings_conn = Arc::clone(&timings_task);
                    tokio::spawn(async move {
                        let _ = handle_tunnel_connection(client_stream, socks, addrs, timings_conn).await;
                    });
                }
            }
        }
    });

    Ok(ActiveTunnel {
        local_proxy_url: format!("http://{local_addr}"),
        timings,
        cancel,
    })
}

async fn handle_tunnel_connection(
    client_stream: TcpStream,
    socks_proxy: Option<ProxyConfig>,
    resolved_addrs: Option<Vec<SocketAddr>>,
    timings: Arc<Mutex<SharedPhaseTimings>>,
) -> Result<()> {
    let mut reader = BufReader::new(client_stream);
    let mut first_line = String::new();
    if reader.read_line(&mut first_line).await? == 0 {
        return Ok(());
    }
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(anyhow!("Malformed proxy line: {first_line}"));
    }
    let method = parts[0];
    let target = parts[1];

    let mut header_lines = Vec::new();
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).await?;
        if n == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        header_lines.push(line);
    }

    if method.eq_ignore_ascii_case("CONNECT") {
        let (host, port) = split_host_port(target, 443)?;
        let tcp_start = Instant::now();
        let mut upstream = connect_upstream(&host, port, socks_proxy.as_ref(), resolved_addrs.as_deref()).await?;
        {
            let mut t = timings.lock();
            if t.tcp_connect_ms == 0 {
                t.tcp_connect_ms = tcp_start.elapsed().as_millis() as u64;
            }
        }

        let mut client = reader.into_inner();
        client
            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .await?;

        // Observe TLS ClientHello -> first TLS Server flight timing cleanly while relaying
        let tls_start = Instant::now();
        let mut buf_c = [0u8; 4096];
        if let Ok(Ok(n)) = tokio::time::timeout(Duration::from_millis(500), client.read(&mut buf_c)).await {
            if n > 0 {
                upstream.write_all(&buf_c[..n]).await?;
                let mut buf_s = [0u8; 4096];
                let sn = upstream.read(&mut buf_s).await?;
                if sn > 0 {
                    let tls_ms = tls_start.elapsed().as_millis() as u64;
                    {
                        let mut t = timings.lock();
                        if t.tls_handshake_ms == 0 {
                            t.tls_handshake_ms = tls_ms;
                        }
                    }
                    client.write_all(&buf_s[..sn]).await?;
                }
            }
        }
        let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
    } else {
        // Plain HTTP proxy forward: convert `METHOD http://host:port/path HTTP/1.1` to origin-form `METHOD /path HTTP/1.1`
        let parsed = url::Url::parse(target)?;
        let host = parsed.host_str().unwrap_or("localhost").to_string();
        let port = parsed.port_or_known_default().unwrap_or(80);
        let origin_path = match parsed.query() {
            Some(q) => format!("{}?{q}", parsed.path()),
            None => parsed.path().to_string(),
        };
        let version = parts.get(2).copied().unwrap_or("HTTP/1.1");

        let tcp_start = Instant::now();
        let mut upstream = connect_upstream(&host, port, socks_proxy.as_ref(), resolved_addrs.as_deref()).await?;
        {
            let mut t = timings.lock();
            if t.tcp_connect_ms == 0 {
                t.tcp_connect_ms = tcp_start.elapsed().as_millis() as u64;
            }
        }

        let mut head = format!("{method} {origin_path} {version}\r\n");
        let mut has_host = false;
        for h in &header_lines {
            let low = h.to_ascii_lowercase();
            if low.starts_with("proxy-connection:") || low.starts_with("proxy-authorization:") {
                continue;
            }
            if low.starts_with("host:") {
                has_host = true;
            }
            head.push_str(h);
        }
        if !has_host {
            head.push_str(&format!("Host: {host}:{port}\r\n"));
        }
        head.push_str("\r\n");
        upstream.write_all(head.as_bytes()).await?;

        // Flush any buffered body bytes already read into BufReader before bidirectional relay
        let leftover = reader.buffer().to_vec();
        if !leftover.is_empty() {
            upstream.write_all(&leftover).await?;
        }
        let mut client = reader.into_inner();
        let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
    }

    Ok(())
}

async fn connect_upstream(
    host: &str,
    port: u16,
    socks_proxy: Option<&ProxyConfig>,
    resolved_addrs: Option<&[SocketAddr]>,
) -> Result<TcpStream> {
    if let Some(proxy) = socks_proxy {
        let proxy_addr = format!("{}:{}", proxy.host.trim(), proxy.port);
        let target = if let Some(addr) = resolved_addrs.and_then(|a| a.first()) {
            addr.to_string()
        } else {
            format!("{host}:{port}")
        };
        let stream = if proxy.auth_enabled && !proxy.username.is_empty() {
            Socks5Stream::connect_with_password(
                proxy_addr.as_str(),
                target.as_str(),
                &proxy.username,
                &proxy.password,
            )
            .await
            .map_err(|e| anyhow!("SOCKS5 proxy auth connect failed: {e}"))?
        } else {
            Socks5Stream::connect(proxy_addr.as_str(), target.as_str())
                .await
                .map_err(|e| anyhow!("SOCKS5 proxy connect failed: {e}"))?
        };
        return Ok(stream.into_inner());
    }

    if let Some(addrs) = resolved_addrs {
        if let Some(first) = addrs.first() {
            return Ok(TcpStream::connect(*first).await?);
        }
    }
    Ok(TcpStream::connect((host, port)).await?)
}

fn split_host_port(authority: &str, default_port: u16) -> Result<(String, u16)> {
    if let Some((h, p)) = authority.rsplit_once(':') {
        if let Ok(port) = p.parse::<u16>() {
            let clean_h = h.trim_start_matches('[').trim_end_matches(']');
            return Ok((clean_h.to_string(), port));
        }
    }
    Ok((authority.to_string(), default_port))
}
