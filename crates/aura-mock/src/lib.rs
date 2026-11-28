//! Local & Production HTTP/1.1 Mock Server with rule matching, dynamic templates/fakers,
//! user datasets (CSV/JSON + stateful CRUD), `deno_core` JS scripting, and proxy passthrough.

use anyhow::{anyhow, Context, Result};
use aura_core::data_feeder::{DataFeeder, DataFeederConfig};
use aura_core::mock::{
    http_status_reason, match_route, parse_query_string, render_mock_template, IncomingMockRequest,
    MockDatasetConfig, MockDatasetSource, MockHitLog, MockResponseMode, MockResponseSpec,
    MockRouteRule, MockServerConfigFile, MockUnmatchedPolicy, DEFAULT_MAX_BODY_BYTES,
};
use aura_scripting::AuraScriptRuntime;
use chrono::Utc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;

const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_HIT_LOGS: usize = 500;

/// Legacy simple route struct kept for ergonomic Rust API compatibility.
#[derive(Debug, Clone)]
pub struct MockRoute {
    pub method: String,
    pub path: String,
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl From<MockRoute> for MockRouteRule {
    fn from(r: MockRoute) -> Self {
        let mut rule = MockRouteRule::simple(r.method, r.path, r.status, r.body);
        rule.response.headers = r.headers;
        rule
    }
}

#[derive(Debug)]
struct RuntimeDataset {
    rows: RwLock<Vec<serde_json::Value>>,
    cursor: AtomicUsize,
    stateful_crud: bool,
    id_field: String,
}

impl RuntimeDataset {
    fn from_config(cfg: &MockDatasetConfig) -> Result<Self> {
        let (rows, stateful_crud, id_field) = match &cfg.source {
            MockDatasetSource::InlineJson {
                rows,
                stateful_crud,
                id_field,
            } => (rows.clone(), *stateful_crud, id_field.clone()),
            MockDatasetSource::File {
                path,
                stateful_crud,
                id_field,
            } => {
                let loaded = load_dataset_file(path)?;
                (loaded, *stateful_crud, id_field.clone())
            }
        };
        Ok(Self {
            rows: RwLock::new(rows),
            cursor: AtomicUsize::new(0),
            stateful_crud,
            id_field,
        })
    }

    fn next_row_map(&self) -> Option<HashMap<String, String>> {
        let guard = self.rows.read();
        if guard.is_empty() {
            return None;
        }
        let idx = self.cursor.fetch_add(1, Ordering::Relaxed) % guard.len();
        Some(json_value_to_string_map(&guard[idx]))
    }

    fn all_rows_json(&self) -> String {
        let guard = self.rows.read();
        serde_json::to_string_pretty(&*guard).unwrap_or_else(|_| "[]".to_string())
    }

    fn find_by_id(&self, id: &str) -> Option<serde_json::Value> {
        let guard = self.rows.read();
        guard
            .iter()
            .find(|row| {
                row.get(&self.id_field)
                    .map(json_scalar_to_string)
                    .as_deref()
                    == Some(id)
            })
            .cloned()
    }

    fn insert_row(&self, mut item: serde_json::Value) -> serde_json::Value {
        if let Some(obj) = item.as_object_mut() {
            if !obj.contains_key(&self.id_field) {
                obj.insert(
                    self.id_field.clone(),
                    serde_json::Value::String(uuid::Uuid::new_v4().to_string()),
                );
            }
        }
        self.rows.write().push(item.clone());
        item
    }

    fn update_by_id(&self, id: &str, patch: serde_json::Value) -> Option<serde_json::Value> {
        let mut guard = self.rows.write();
        let target = guard.iter_mut().find(|row| {
            row.get(&self.id_field)
                .map(json_scalar_to_string)
                .as_deref()
                == Some(id)
        })?;
        if let (Some(dst), Some(src)) = (target.as_object_mut(), patch.as_object()) {
            for (k, v) in src {
                dst.insert(k.clone(), v.clone());
            }
        } else {
            *target = patch;
        }
        Some(target.clone())
    }

    fn delete_by_id(&self, id: &str) -> bool {
        let mut guard = self.rows.write();
        let before = guard.len();
        guard.retain(|row| {
            row.get(&self.id_field)
                .map(json_scalar_to_string)
                .as_deref()
                != Some(id)
        });
        guard.len() < before
    }
}

fn json_scalar_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

fn json_value_to_string_map(v: &serde_json::Value) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(obj) = v.as_object() {
        for (k, val) in obj {
            map.insert(k.clone(), json_scalar_to_string(val));
        }
    }
    map
}

fn load_dataset_file(path: &str) -> Result<Vec<serde_json::Value>> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("Failed to read dataset: {path}"))?;
    let trimmed = content.trim();
    if trimmed.starts_with('[') {
        let arr: Vec<serde_json::Value> =
            serde_json::from_str(trimmed).context("Failed to parse JSON array dataset")?;
        return Ok(arr);
    }

    // Fall back to DataFeeder (CSV or NDJSON)
    let cfg = DataFeederConfig {
        enabled: true,
        file_path: path.to_string(),
        ..Default::default()
    };
    let feeder = DataFeeder::from_config(&cfg)?;
    let count = feeder.row_count();
    let mut rows = Vec::with_capacity(count);
    let mut cursor = 0usize;
    for _ in 0..count {
        if let Some(row) = feeder.next_row(&mut cursor) {
            let obj: serde_json::Map<String, serde_json::Value> = row
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect();
            rows.push(serde_json::Value::Object(obj));
        }
    }
    Ok(rows)
}

struct MockRuntimeState {
    routes: RwLock<Vec<MockRouteRule>>,
    datasets: RwLock<HashMap<String, Arc<RuntimeDataset>>>,
    unmatched: RwLock<MockUnmatchedPolicy>,
    max_body_bytes: usize,
    hit_logs: RwLock<Vec<MockHitLog>>,
    http_client: reqwest::Client,
}

#[derive(Clone)]
pub struct MockServerHandle {
    pub listen_addr: SocketAddr,
    cancel: CancellationToken,
    state: Arc<MockRuntimeState>,
}

impl std::fmt::Debug for MockServerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockServerHandle")
            .field("listen_addr", &self.listen_addr)
            .field("running", &!self.cancel.is_cancelled())
            .finish()
    }
}

impl MockServerHandle {
    pub fn stop(&self) {
        self.cancel.cancel();
    }

    pub fn is_running(&self) -> bool {
        !self.cancel.is_cancelled()
    }

    pub fn add_route_rule(&self, rule: MockRouteRule) {
        self.state.routes.write().push(rule);
    }

    pub fn replace_route_rules(&self, rules: Vec<MockRouteRule>) {
        *self.state.routes.write() = rules;
    }

    pub fn list_route_rules(&self) -> Vec<MockRouteRule> {
        self.state.routes.read().clone()
    }

    pub fn hit_logs(&self) -> Vec<MockHitLog> {
        self.state.hit_logs.read().clone()
    }

    pub fn clear_hit_logs(&self) {
        self.state.hit_logs.write().clear();
    }
}

pub struct MockServer {
    config: MockServerConfigFile,
}

impl MockServer {
    pub fn new(port: u16) -> Self {
        let mut config = MockServerConfigFile::default();
        config.port = port;
        Self { config }
    }

    pub fn from_config(config: MockServerConfigFile) -> Self {
        Self { config }
    }

    pub fn add_route(&mut self, route: MockRoute) {
        self.config.routes.push(route.into());
    }

    pub fn add_rule(&mut self, rule: MockRouteRule) {
        self.config.routes.push(rule);
    }

    /// Start the mock server in the background and return a handle with its bound address
    /// and cancellation token.
    pub async fn start(self) -> Result<MockServerHandle> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr)
            .await
            .with_context(|| format!("Failed to bind mock server on {addr}"))?;
        let listen_addr = listener.local_addr()?;

        let mut datasets_map = HashMap::new();
        for ds in &self.config.datasets {
            let rt = RuntimeDataset::from_config(ds)?;
            datasets_map.insert(ds.name.clone(), Arc::new(rt));
        }

        let max_body_bytes = if self.config.max_body_bytes == 0 {
            DEFAULT_MAX_BODY_BYTES
        } else {
            self.config.max_body_bytes
        };

        let state = Arc::new(MockRuntimeState {
            routes: RwLock::new(self.config.routes),
            datasets: RwLock::new(datasets_map),
            unmatched: RwLock::new(self.config.unmatched),
            max_body_bytes,
            hit_logs: RwLock::new(Vec::new()),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()?,
        });

        let cancel = CancellationToken::new();
        let cancel_loop = cancel.clone();
        let state_loop = Arc::clone(&state);

        tracing::info!(
            target: "aura::mock",
            "MockServer listening on http://{}",
            listen_addr
        );

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel_loop.cancelled() => {
                        tracing::info!(target: "aura::mock", "MockServer shutting down on {}", listen_addr);
                        break;
                    }
                    accepted = listener.accept() => {
                        match accepted {
                            Ok((stream, _peer)) => {
                                let conn_state = Arc::clone(&state_loop);
                                tokio::spawn(async move {
                                    if let Err(e) = handle_connection(stream, conn_state).await {
                                        tracing::debug!(target: "aura::mock", "Connection error: {e}");
                                    }
                                });
                            }
                            Err(e) => {
                                tracing::warn!(target: "aura::mock", "Accept error: {e}");
                            }
                        }
                    }
                }
            }
        });

        Ok(MockServerHandle {
            listen_addr,
            cancel,
            state,
        })
    }

    /// Legacy convenience wrapper that starts the server on the configured port.
    pub async fn run(&self) -> Result<()> {
        let clone = MockServer::from_config(self.config.clone());
        let _handle = clone.start().await?;
        Ok(())
    }
}

struct RenderedHttpResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: String,
}

async fn handle_connection(mut stream: TcpStream, state: Arc<MockRuntimeState>) -> Result<()> {
    let started = Instant::now();
    let req = match read_http_request(&mut stream, state.max_body_bytes).await {
        Ok(Some(r)) => r,
        Ok(None) => return Ok(()),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("Payload Too Large") {
                413
            } else {
                400
            };
            let body = serde_json::json!({ "error": msg }).to_string();
            write_http_response(
                &mut stream,
                status,
                &HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
                body.as_bytes(),
            )
            .await?;
            return Ok(());
        }
    };

    let matched_opt = {
        let routes = state.routes.read();
        match_route(&routes, &req).map(|m| (m.rule.clone(), m.path_params))
    };

    let (matched_name, resp) = if let Some((rule, path_params)) = matched_opt {
        if let Some(delay_ms) = rule.response.delay_ms {
            if delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
        }
        let rendered = execute_matched_response(&rule.response, &req, &path_params, &state).await;
        (Some(rule.name), rendered)
    } else {
        let policy = state.unmatched.read().clone();
        let rendered = handle_unmatched(&policy, &req, &state.http_client).await;
        (None, rendered)
    };

    let duration_ms = started.elapsed().as_millis() as u64;
    tracing::info!(
        target: "aura::mock",
        method = %req.method,
        path = %req.raw_path,
        status = resp.status,
        route = ?matched_name,
        duration_ms = duration_ms,
        "Mock request handled"
    );

    {
        let mut logs = state.hit_logs.write();
        logs.push(MockHitLog {
            timestamp: Utc::now().to_rfc3339(),
            method: req.method.clone(),
            path: req.raw_path.clone(),
            matched_route: matched_name,
            status: resp.status,
            duration_ms,
        });
        if logs.len() > MAX_HIT_LOGS {
            let overflow = logs.len() - MAX_HIT_LOGS;
            logs.drain(0..overflow);
        }
    }

    write_http_response(&mut stream, resp.status, &resp.headers, resp.body.as_bytes()).await
}

async fn execute_matched_response(
    spec: &MockResponseSpec,
    req: &IncomingMockRequest,
    path_params: &HashMap<String, String>,
    state: &Arc<MockRuntimeState>,
) -> RenderedHttpResponse {
    let dataset_opt = spec
        .dataset_name
        .as_ref()
        .and_then(|name| state.datasets.read().get(name).cloned());

    match spec.mode {
        MockResponseMode::Static => RenderedHttpResponse {
            status: spec.status,
            headers: ensure_content_type(spec.headers.clone(), &spec.body),
            body: spec.body.clone(),
        },
        MockResponseMode::Template => {
            let row = dataset_opt.as_ref().and_then(|ds| ds.next_row_map());
            let body =
                render_mock_template(&spec.body, req, path_params, row.as_ref(), &HashMap::new());
            let mut headers = HashMap::new();
            for (k, v) in &spec.headers {
                headers.insert(
                    k.clone(),
                    render_mock_template(v, req, path_params, row.as_ref(), &HashMap::new()),
                );
            }
            RenderedHttpResponse {
                status: spec.status,
                headers: ensure_content_type(headers, &body),
                body,
            }
        }
        MockResponseMode::Dataset => {
            execute_dataset_response(spec, req, path_params, dataset_opt.as_deref())
        }
        MockResponseMode::Script => {
            execute_script_response(spec, req, path_params, dataset_opt.as_deref()).await
        }
    }
}

fn execute_dataset_response(
    spec: &MockResponseSpec,
    req: &IncomingMockRequest,
    path_params: &HashMap<String, String>,
    dataset_opt: Option<&RuntimeDataset>,
) -> RenderedHttpResponse {
    let Some(ds) = dataset_opt else {
        let body = serde_json::json!({
            "error": "Configured dataset not found",
            "dataset": spec.dataset_name
        })
        .to_string();
        return RenderedHttpResponse {
            status: 500,
            headers: json_headers(),
            body,
        };
    };

    if ds.stateful_crud {
        let id_opt = path_params
            .get(&ds.id_field)
            .or_else(|| path_params.get("id"))
            .map(String::as_str);

        match req.method.as_str() {
            "GET" => {
                if let Some(id) = id_opt {
                    if let Some(item) = ds.find_by_id(id) {
                        return RenderedHttpResponse {
                            status: 200,
                            headers: json_headers(),
                            body: serde_json::to_string_pretty(&item).unwrap_or_default(),
                        };
                    }
                    return RenderedHttpResponse {
                        status: 404,
                        headers: json_headers(),
                        body: serde_json::json!({"error": "Not found", "id": id}).to_string(),
                    };
                }
                return RenderedHttpResponse {
                    status: 200,
                    headers: json_headers(),
                    body: ds.all_rows_json(),
                };
            }
            "POST" => {
                let parsed: serde_json::Value = serde_json::from_str(&req.body)
                    .unwrap_or_else(|_| serde_json::json!({ "raw": req.body }));
                let created = ds.insert_row(parsed);
                return RenderedHttpResponse {
                    status: 201,
                    headers: json_headers(),
                    body: serde_json::to_string_pretty(&created).unwrap_or_default(),
                };
            }
            "PUT" | "PATCH" => {
                if let Some(id) = id_opt {
                    let parsed: serde_json::Value = serde_json::from_str(&req.body)
                        .unwrap_or_else(|_| serde_json::json!({}));
                    if let Some(updated) = ds.update_by_id(id, parsed) {
                        return RenderedHttpResponse {
                            status: 200,
                            headers: json_headers(),
                            body: serde_json::to_string_pretty(&updated).unwrap_or_default(),
                        };
                    }
                }
                return RenderedHttpResponse {
                    status: 404,
                    headers: json_headers(),
                    body: serde_json::json!({"error": "Not found"}).to_string(),
                };
            }
            "DELETE" => {
                if let Some(id) = id_opt {
                    if ds.delete_by_id(id) {
                        return RenderedHttpResponse {
                            status: 200,
                            headers: json_headers(),
                            body: serde_json::json!({"deleted": true, "id": id}).to_string(),
                        };
                    }
                }
                return RenderedHttpResponse {
                    status: 404,
                    headers: json_headers(),
                    body: serde_json::json!({"error": "Not found"}).to_string(),
                };
            }
            _ => {}
        }
    }

    // Non-CRUD or fallback: pick next row and render body template (or return row JSON)
    let row = ds.next_row_map();
    let body = if spec.body.trim().is_empty() {
        serde_json::to_string_pretty(&row).unwrap_or_else(|_| "{}".to_string())
    } else {
        render_mock_template(&spec.body, req, path_params, row.as_ref(), &HashMap::new())
    };
    RenderedHttpResponse {
        status: spec.status,
        headers: ensure_content_type(spec.headers.clone(), &body),
        body,
    }
}

async fn execute_script_response(
    spec: &MockResponseSpec,
    req: &IncomingMockRequest,
    path_params: &HashMap<String, String>,
    dataset_opt: Option<&RuntimeDataset>,
) -> RenderedHttpResponse {
    let script = spec.script.clone();
    let fallback_status = spec.status;
    let fallback_headers = spec.headers.clone();
    let req_json = serde_json::json!({
        "method": req.method,
        "path": req.path,
        "rawPath": req.raw_path,
        "params": path_params,
        "query": req.query,
        "headers": req.headers,
        "body": req.body,
    });
    let dataset_json = dataset_opt
        .map(|ds| ds.rows.read().clone())
        .unwrap_or_default();

    let res = tokio::task::spawn_blocking(move || -> Result<RenderedHttpResponse> {
        let mut rt = AuraScriptRuntime::new(HashMap::new())?;
        let req_str = serde_json::to_string(&req_json)?;
        let ds_str = serde_json::to_string(&dataset_json)?;
        let wrapped = format!(
            r#"
            const req = {req_str};
            req.json = function() {{ return JSON.parse(req.body || "{{}}"); }};
            const dataset = {ds_str};
            function respond(res) {{
                aura.variables.set("__mock_res", JSON.stringify(res || {{}}));
            }}
            const __fn_result = (function() {{
                {script}
            }})();
            if (__fn_result !== undefined && !aura.variables.get("__mock_res")) {{
                respond(__fn_result);
            }}
            "#
        );
        let exec_res = rt.run(&wrapped)?;
        let raw_res = exec_res
            .variables
            .get("__mock_res")
            .cloned()
            .unwrap_or_else(|| "{}".to_string());
        let val: serde_json::Value =
            serde_json::from_str(&raw_res).unwrap_or_else(|_| serde_json::json!({}));
        let status = val
            .get("status")
            .and_then(|v| v.as_u64())
            .map(|s| s as u16)
            .unwrap_or(fallback_status);
        let mut headers = fallback_headers;
        if let Some(hobj) = val.get("headers").and_then(|v| v.as_object()) {
            for (k, v) in hobj {
                headers.insert(k.clone(), json_scalar_to_string(v));
            }
        }
        let body = match val.get("body") {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(other) => serde_json::to_string(other).unwrap_or_default(),
            None => String::new(),
        };
        Ok(RenderedHttpResponse {
            status,
            headers: ensure_content_type(headers, &body),
            body,
        })
    })
    .await;

    match res {
        Ok(Ok(rendered)) => rendered,
        Ok(Err(e)) => RenderedHttpResponse {
            status: 500,
            headers: json_headers(),
            body: serde_json::json!({ "error": format!("Mock script error: {e}") }).to_string(),
        },
        Err(e) => RenderedHttpResponse {
            status: 500,
            headers: json_headers(),
            body: serde_json::json!({ "error": format!("Mock script join error: {e}") }).to_string(),
        },
    }
}

async fn handle_unmatched(
    policy: &MockUnmatchedPolicy,
    req: &IncomingMockRequest,
    client: &reqwest::Client,
) -> RenderedHttpResponse {
    match policy {
        MockUnmatchedPolicy::NotFound => RenderedHttpResponse {
            status: 404,
            headers: json_headers(),
            body: serde_json::json!({
                "error": "No mock route matched",
                "method": req.method,
                "path": req.raw_path
            })
            .to_string(),
        },
        MockUnmatchedPolicy::StaticOk => RenderedHttpResponse {
            status: 200,
            headers: json_headers(),
            body: "{\"mock\": true, \"status\": \"ok\"}".to_string(),
        },
        MockUnmatchedPolicy::ProxyPassthrough { target_base_url } => {
            let base = target_base_url.trim_end_matches('/');
            let url = format!("{base}{}", req.raw_path);
            let method = reqwest::Method::from_bytes(req.method.as_bytes())
                .unwrap_or(reqwest::Method::GET);
            let mut builder = client.request(method, &url);
            for (k, v) in &req.headers {
                if !k.eq_ignore_ascii_case("host") && !k.eq_ignore_ascii_case("content-length") {
                    builder = builder.header(k, v);
                }
            }
            if !req.body.is_empty() {
                builder = builder.body(req.body.clone());
            }
            match builder.send().await {
                Ok(up) => {
                    let status = up.status().as_u16();
                    let mut headers = HashMap::new();
                    for (k, v) in up.headers() {
                        if let Ok(vs) = v.to_str() {
                            if !k.as_str().eq_ignore_ascii_case("transfer-encoding") {
                                headers.insert(k.to_string(), vs.to_string());
                            }
                        }
                    }
                    let body = up.text().await.unwrap_or_default();
                    RenderedHttpResponse {
                        status,
                        headers: ensure_content_type(headers, &body),
                        body,
                    }
                }
                Err(e) => RenderedHttpResponse {
                    status: 502,
                    headers: json_headers(),
                    body: serde_json::json!({
                        "error": format!("Proxy passthrough failed: {e}"),
                        "target": url
                    })
                    .to_string(),
                },
            }
        }
    }
}

fn json_headers() -> HashMap<String, String> {
    HashMap::from([("Content-Type".to_string(), "application/json".to_string())])
}

fn ensure_content_type(mut headers: HashMap<String, String>, body: &str) -> HashMap<String, String> {
    let has_ct = headers.keys().any(|k| k.eq_ignore_ascii_case("content-type"));
    if !has_ct {
        let trimmed = body.trim_start();
        let ct = if trimmed.starts_with('{') || trimmed.starts_with('[') {
            "application/json"
        } else {
            "text/plain; charset=utf-8"
        };
        headers.insert("Content-Type".to_string(), ct.to_string());
    }
    headers
}

async fn read_http_request(
    stream: &mut TcpStream,
    max_body_bytes: usize,
) -> Result<Option<IncomingMockRequest>> {
    let mut buf = Vec::with_capacity(2048);
    let mut temp = [0u8; 2048];
    let header_end;

    loop {
        let n = stream.read(&mut temp).await?;
        if n == 0 {
            if buf.is_empty() {
                return Ok(None);
            }
            return Err(anyhow!("Unexpected EOF before HTTP headers completed"));
        }
        buf.extend_from_slice(&temp[..n]);
        if let Some(pos) = find_double_crlf(&buf) {
            header_end = pos;
            break;
        }
        if buf.len() > MAX_HEADER_BYTES {
            return Err(anyhow!("HTTP headers exceed maximum size"));
        }
    }

    let header_text = String::from_utf8_lossy(&buf[..header_end]);
    let mut lines = header_text.split("\r\n");
    let first_line = lines.next().ok_or_else(|| anyhow!("Empty HTTP request"))?;
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_uppercase();
    let raw_path = parts.next().unwrap_or("/").to_string();

    let mut headers = HashMap::new();
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_string(), v.trim().to_string());
        }
    }

    let mut body_bytes = buf[(header_end + 4)..].to_vec();
    let is_chunked = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("transfer-encoding"))
        .map(|(_, v)| v.to_ascii_lowercase().contains("chunked"))
        .unwrap_or(false);

    if is_chunked {
        while !chunked_complete(&body_bytes) {
            if body_bytes.len() > max_body_bytes {
                return Err(anyhow!("Payload Too Large"));
            }
            let n = stream.read(&mut temp).await?;
            if n == 0 {
                break;
            }
            body_bytes.extend_from_slice(&temp[..n]);
        }
        body_bytes = decode_chunked(&body_bytes, max_body_bytes)?;
    } else if let Some(len_str) = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .map(|(_, v)| v.as_str())
    {
        let content_len: usize = len_str.parse().unwrap_or(0);
        if content_len > max_body_bytes {
            return Err(anyhow!("Payload Too Large"));
        }
        while body_bytes.len() < content_len {
            let n = stream.read(&mut temp).await?;
            if n == 0 {
                break;
            }
            body_bytes.extend_from_slice(&temp[..n]);
        }
        body_bytes.truncate(content_len);
    }

    let (path, query) = parse_query_string(&raw_path);
    Ok(Some(IncomingMockRequest {
        method,
        raw_path,
        path,
        query,
        headers,
        body: String::from_utf8_lossy(&body_bytes).to_string(),
    }))
}

fn find_double_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn chunked_complete(buf: &[u8]) -> bool {
    buf.windows(5).any(|w| w == b"0\r\n\r\n") || buf.starts_with(b"0\r\n\r\n")
}

fn decode_chunked(raw: &[u8], max_body_bytes: usize) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos < raw.len() {
        let Some(rel_crlf) = raw[pos..].windows(2).position(|w| w == b"\r\n") else {
            break;
        };
        let size_line = String::from_utf8_lossy(&raw[pos..pos + rel_crlf]);
        let hex_part = size_line.split(';').next().unwrap_or("").trim();
        let chunk_size = usize::from_str_radix(hex_part, 16)
            .map_err(|_| anyhow!("Invalid chunk size in Transfer-Encoding: chunked"))?;
        pos += rel_crlf + 2;
        if chunk_size == 0 {
            break;
        }
        if out.len() + chunk_size > max_body_bytes {
            return Err(anyhow!("Payload Too Large"));
        }
        if pos + chunk_size > raw.len() {
            out.extend_from_slice(&raw[pos..]);
            break;
        }
        out.extend_from_slice(&raw[pos..pos + chunk_size]);
        pos += chunk_size + 2; // skip trailing \r\n
    }
    Ok(out)
}

async fn write_http_response(
    stream: &mut TcpStream,
    status: u16,
    headers: &HashMap<String, String>,
    body: &[u8],
) -> Result<()> {
    let reason = http_status_reason(status);
    let mut head = format!("HTTP/1.1 {status} {reason}\r\n");
    for (k, v) in headers {
        if !k.eq_ignore_ascii_case("content-length") && !k.eq_ignore_ascii_case("connection") {
            head.push_str(&format!("{k}: {v}\r\n"));
        }
    }
    head.push_str(&format!("Content-Length: {}\r\n", body.len()));
    head.push_str("Connection: close\r\n\r\n");

    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await?;
    let _ = stream.shutdown().await;
    Ok(())
}
