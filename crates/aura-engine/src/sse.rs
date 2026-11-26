//! Incremental Server-Sent Events (SSE / `text/event-stream`) parser and async streaming client.

use anyhow::Result;
use aura_core::model::{AuraRequest, AuraResponse, SseEvent};
use aura_core::templating::TemplateContext;
use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;
use tokio::sync::mpsc;

#[derive(Debug, Default)]
pub struct SseStreamParser {
    buffer: String,
    current_id: Option<String>,
    current_event: String,
    data_lines: Vec<String>,
    retry_ms: Option<u64>,
}

impl SseStreamParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_chunk(&mut self, chunk: &str) -> Vec<SseEvent> {
        self.buffer.push_str(chunk);
        let mut events = Vec::new();
        while let Some(nl_pos) = self.buffer.find('\n') {
            let line = self.buffer[..nl_pos].trim_end_matches('\r').to_string();
            self.buffer.drain(..=nl_pos);
            if let Some(ev) = self.process_line(&line) {
                events.push(ev);
            }
        }
        events
    }

    pub fn finish(&mut self) -> Vec<SseEvent> {
        let mut events = Vec::new();
        if !self.buffer.is_empty() {
            let rem = std::mem::take(&mut self.buffer);
            let line = rem.trim_end_matches('\r').to_string();
            if let Some(ev) = self.process_line(&line) {
                events.push(ev);
            }
        }
        if !self.data_lines.is_empty() {
            events.push(self.flush_event());
        }
        events
    }

    fn process_line(&mut self, line: &str) -> Option<SseEvent> {
        if line.is_empty() {
            if !self.data_lines.is_empty() || !self.current_event.is_empty() {
                return Some(self.flush_event());
            }
            return None;
        }
        if line.starts_with(':') {
            return None;
        }
        let (field, val) = if let Some((f, v)) = line.split_once(':') {
            (f, v.strip_prefix(' ').unwrap_or(v))
        } else {
            (line, "")
        };
        match field {
            "data" => self.data_lines.push(val.to_string()),
            "event" => self.current_event = val.to_string(),
            "id" => {
                if !val.contains('\0') {
                    self.current_id = Some(val.to_string());
                }
            }
            "retry" => {
                if let Ok(ms) = val.parse::<u64>() {
                    self.retry_ms = Some(ms);
                }
            }
            _ => {}
        }
        None
    }

    fn flush_event(&mut self) -> SseEvent {
        let event_name = if self.current_event.is_empty() {
            "message".to_string()
        } else {
            std::mem::take(&mut self.current_event)
        };
        let data = std::mem::take(&mut self.data_lines).join("\n");
        SseEvent {
            id: self.current_id.clone(),
            event: event_name,
            data,
            retry_ms: self.retry_ms,
            timestamp: Utc::now(),
        }
    }
}

pub async fn execute_sse_request(
    req: &AuraRequest,
    context: &TemplateContext,
    event_tx: Option<mpsc::UnboundedSender<SseEvent>>,
) -> Result<AuraResponse> {
    let url = context.render(&req.url);
    let mut header_map = HeaderMap::new();
    for h in &req.headers {
        if h.enabled {
            let k = context.render(&h.key);
            let v = context.render(&h.value);
            if let (Ok(hn), Ok(hv)) = (HeaderName::from_str(&k), HeaderValue::from_str(&v)) {
                header_map.insert(hn, hv);
            }
        }
    }
    if !header_map.contains_key("accept") {
        header_map.insert("accept", HeaderValue::from_static("text/event-stream"));
    }

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(!req.settings.verify_ssl)
        .build()?;

    let mut params = Vec::new();
    for p in &req.params {
        if p.enabled {
            params.push((context.render(&p.key), context.render(&p.value)));
        }
    }

    let start = Instant::now();
    let mut builder = client.get(&url).headers(header_map);
    if !params.is_empty() {
        builder = builder.query(&params);
    }
    let mut resp = builder.send().await?;
    let status = resp.status().as_u16();
    let status_text = resp.status().canonical_reason().unwrap_or("").to_string();
    let mut resp_headers = HashMap::new();
    for (k, v) in resp.headers() {
        if let Ok(vs) = v.to_str() {
            resp_headers.insert(k.to_string(), vs.to_string());
        }
    }

    let mut parser = SseStreamParser::new();
    let mut all_events = Vec::new();
    let mut raw_body = String::new();
    let mut total_bytes = 0usize;

    while let Some(chunk) = resp.chunk().await? {
        total_bytes += chunk.len();
        let text = String::from_utf8_lossy(&chunk);
        raw_body.push_str(&text);
        for ev in parser.push_chunk(&text) {
            if let Some(ref tx) = event_tx {
                let _ = tx.send(ev.clone());
            }
            all_events.push(ev);
        }
    }
    for ev in parser.finish() {
        if let Some(ref tx) = event_tx {
            let _ = tx.send(ev.clone());
        }
        all_events.push(ev);
    }

    Ok(AuraResponse {
        status,
        status_text,
        duration_ms: start.elapsed().as_millis() as u64,
        size_bytes: total_bytes,
        headers: resp_headers,
        body: raw_body,
        timestamp: Utc::now(),
        assertions: Vec::new(),
        console_logs: Vec::new(),
        script_errors: Vec::new(),
        sse_events: all_events,
        stream_messages: Vec::new(),
    })
}
