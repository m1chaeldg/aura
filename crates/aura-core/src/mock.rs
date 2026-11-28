//! Pure domain models, route/request rule matching, template rendering, and collection
//! conversion for Aura's local & production Mock Server.

use crate::model::AuraRequest;
use crate::templating::TemplateContext;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

pub const DEFAULT_MAX_BODY_BYTES: usize = 10 * 1024 * 1024; // 10 MiB

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum MockPathMatcher {
    Exact(String),
    /// Supports `:param` / `{param}` segments and `*` / `**` wildcards.
    Pattern(String),
    Regex(String),
}

impl Default for MockPathMatcher {
    fn default() -> Self {
        Self::Exact("/".to_string())
    }
}

impl MockPathMatcher {
    fn specificity(&self) -> u8 {
        match self {
            Self::Exact(_) => 3,
            Self::Pattern(_) => 2,
            Self::Regex(_) => 1,
        }
    }

    pub fn display_value(&self) -> &str {
        match self {
            Self::Exact(s) | Self::Pattern(s) | Self::Regex(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchOperator {
    #[default]
    Equals,
    NotEquals,
    Contains,
    Regex,
    Exists,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MockHeaderMatcher {
    pub name: String,
    #[serde(default)]
    pub operator: MatchOperator,
    #[serde(default)]
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MockQueryMatcher {
    pub name: String,
    #[serde(default)]
    pub operator: MatchOperator,
    #[serde(default)]
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MockBodyMatcher {
    Exact {
        value: String,
    },
    Contains {
        value: String,
    },
    Regex {
        pattern: String,
    },
    JsonPath {
        path: String,
        #[serde(default)]
        operator: MatchOperator,
        #[serde(default)]
        value: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MockRequestRule {
    /// Empty or `["*"]` matches any HTTP method.
    #[serde(default)]
    pub methods: Vec<String>,
    #[serde(default)]
    pub path: MockPathMatcher,
    #[serde(default)]
    pub query: Vec<MockQueryMatcher>,
    #[serde(default)]
    pub headers: Vec<MockHeaderMatcher>,
    #[serde(default)]
    pub body: Option<MockBodyMatcher>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MockResponseMode {
    #[default]
    Static,
    Template,
    Dataset,
    Script,
}

impl MockResponseMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Static => "Static",
            Self::Template => "Template / Faker",
            Self::Dataset => "User Dataset / CRUD",
            Self::Script => "JS Script (deno_core)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MockResponseSpec {
    #[serde(default)]
    pub mode: MockResponseMode,
    #[serde(default = "default_status")]
    pub status: u16,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub script: String,
    #[serde(default)]
    pub dataset_name: Option<String>,
    #[serde(default)]
    pub delay_ms: Option<u64>,
}

fn default_status() -> u16 {
    200
}

impl Default for MockResponseSpec {
    fn default() -> Self {
        Self {
            mode: MockResponseMode::Static,
            status: 200,
            headers: HashMap::new(),
            body: "{\"ok\": true}".to_string(),
            script: String::new(),
            dataset_name: None,
            delay_ms: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MockRouteRule {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub priority: i32,
    pub rule: MockRequestRule,
    pub response: MockResponseSpec,
}

fn default_true() -> bool {
    true
}

impl MockRouteRule {
    pub fn simple(method: impl Into<String>, path: impl Into<String>, status: u16, body: impl Into<String>) -> Self {
        let m = method.into();
        let p = path.into();
        let path_matcher = if p.contains(':') || p.contains('{') || p.contains('*') {
            MockPathMatcher::Pattern(p.clone())
        } else {
            MockPathMatcher::Exact(p.clone())
        };
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: format!("{} {}", m.to_uppercase(), p),
            enabled: true,
            priority: 0,
            rule: MockRequestRule {
                methods: vec![m],
                path: path_matcher,
                ..Default::default()
            },
            response: MockResponseSpec {
                status,
                body: body.into(),
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MockDatasetSource {
    InlineJson {
        rows: Vec<serde_json::Value>,
        #[serde(default)]
        stateful_crud: bool,
        #[serde(default = "default_id_field")]
        id_field: String,
    },
    File {
        path: String,
        #[serde(default)]
        stateful_crud: bool,
        #[serde(default = "default_id_field")]
        id_field: String,
    },
}

fn default_id_field() -> String {
    "id".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MockDatasetConfig {
    pub name: String,
    pub source: MockDatasetSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MockUnmatchedPolicy {
    #[default]
    NotFound,
    ProxyPassthrough {
        target_base_url: String,
    },
    StaticOk,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MockServerConfigFile {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_max_body_bytes")]
    pub max_body_bytes: usize,
    #[serde(default)]
    pub unmatched: MockUnmatchedPolicy,
    #[serde(default)]
    pub datasets: Vec<MockDatasetConfig>,
    #[serde(default)]
    pub routes: Vec<MockRouteRule>,
}

fn default_version() -> u32 {
    1
}
fn default_host() -> String {
    "127.0.0.1".to_string()
}
fn default_port() -> u16 {
    18080
}
fn default_max_body_bytes() -> usize {
    DEFAULT_MAX_BODY_BYTES
}

impl Default for MockServerConfigFile {
    fn default() -> Self {
        Self {
            version: 1,
            host: default_host(),
            port: default_port(),
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
            unmatched: MockUnmatchedPolicy::NotFound,
            datasets: Vec::new(),
            routes: Vec::new(),
        }
    }
}

impl MockServerConfigFile {
    pub fn from_json_str(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(raw)
    }

    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[derive(Debug, Clone, Default)]
pub struct IncomingMockRequest {
    pub method: String,
    pub raw_path: String,
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl IncomingMockRequest {
    pub fn new(method: impl Into<String>, raw_path: impl Into<String>) -> Self {
        let raw = raw_path.into();
        let (path, query) = parse_query_string(&raw);
        Self {
            method: method.into().to_uppercase(),
            raw_path: raw,
            path,
            query,
            headers: HashMap::new(),
            body: String::new(),
        }
    }

    pub fn header_ci(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct MatchedRoute<'a> {
    pub rule: &'a MockRouteRule,
    pub path_params: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockHitLog {
    pub timestamp: String,
    pub method: String,
    pub path: String,
    pub matched_route: Option<String>,
    pub status: u16,
    pub duration_ms: u64,
}

/// Split `/users?role=admin&page=2` into `("/users", {"role": "admin", "page": "2"})`.
pub fn parse_query_string(raw_path: &str) -> (String, HashMap<String, String>) {
    let mut query = HashMap::new();
    let Some((path, qs)) = raw_path.split_once('?') else {
        return (raw_path.to_string(), query);
    };
    for pair in qs.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let k_dec = url::form_urlencoded::parse(k.as_bytes())
            .map(|(dk, _)| dk.into_owned())
            .next()
            .unwrap_or_else(|| k.to_string());
        let v_dec = url::form_urlencoded::parse(v.as_bytes())
            .map(|(dv, _)| dv.into_owned())
            .next()
            .unwrap_or_else(|| v.to_string());
        query.insert(k_dec, v_dec);
    }
    (path.to_string(), query)
}

/// Match a `MockPathMatcher` against a normalized URL path (without query string).
pub fn match_path(matcher: &MockPathMatcher, path: &str) -> Option<HashMap<String, String>> {
    match matcher {
        MockPathMatcher::Exact(expected) => {
            if expected == path {
                Some(HashMap::new())
            } else {
                None
            }
        }
        MockPathMatcher::Pattern(pattern) => match_pattern_segments(pattern, path),
        MockPathMatcher::Regex(pat) => {
            let re = Regex::new(pat).ok()?;
            let caps = re.captures(path)?;
            let mut params = HashMap::new();
            for name in re.capture_names().flatten() {
                if let Some(m) = caps.name(name) {
                    params.insert(name.to_string(), m.as_str().to_string());
                }
            }
            Some(params)
        }
    }
}

fn match_pattern_segments(pattern: &str, path: &str) -> Option<HashMap<String, String>> {
    let pat_segs: Vec<&str> = pattern.trim_start_matches('/').split('/').collect();
    let path_segs: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    let mut params = HashMap::new();

    if pat_segs.last() == Some(&"**") {
        let prefix_len = pat_segs.len() - 1;
        if path_segs.len() < prefix_len {
            return None;
        }
        for i in 0..prefix_len {
            if !match_single_segment(pat_segs[i], path_segs[i], &mut params) {
                return None;
            }
        }
        params.insert("*".to_string(), path_segs[prefix_len..].join("/"));
        return Some(params);
    }

    if pat_segs.len() != path_segs.len() {
        return None;
    }

    for (p, actual) in pat_segs.iter().zip(path_segs.iter()) {
        if !match_single_segment(p, actual, &mut params) {
            return None;
        }
    }
    Some(params)
}

fn match_single_segment(pat: &str, actual: &str, params: &mut HashMap<String, String>) -> bool {
    if pat == "*" {
        return true;
    }
    if let Some(name) = pat.strip_prefix(':') {
        if !name.is_empty() && !actual.is_empty() {
            params.insert(name.to_string(), actual.to_string());
            return true;
        }
        return false;
    }
    if let Some(name) = pat.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        if !name.is_empty() && !actual.is_empty() {
            params.insert(name.to_string(), actual.to_string());
            return true;
        }
        return false;
    }
    pat == actual
}

fn eval_operator(actual: Option<&str>, op: MatchOperator, expected: &str) -> bool {
    match op {
        MatchOperator::Exists => actual.is_some(),
        MatchOperator::Equals => actual.map(|v| v == expected).unwrap_or(false),
        MatchOperator::NotEquals => actual.map(|v| v != expected).unwrap_or(false),
        MatchOperator::Contains => actual.map(|v| v.contains(expected)).unwrap_or(false),
        MatchOperator::Regex => actual
            .and_then(|v| Regex::new(expected).ok().map(|re| re.is_match(v)))
            .unwrap_or(false),
    }
}

/// Extract a scalar/JSON value at a simple dot/bracket path such as `user.role` or `$.items[0].id`.
pub fn extract_json_path_value(root: &serde_json::Value, path: &str) -> Option<String> {
    let cleaned = path.trim().trim_start_matches('$').trim_start_matches('.');
    if cleaned.is_empty() {
        return Some(json_value_to_string(root));
    }
    let mut curr = root;
    for part in cleaned.split('.') {
        if let Some((field, rest)) = part.split_once('[') {
            if !field.is_empty() {
                curr = curr.get(field)?;
            }
            let idx_str = rest.strip_suffix(']')?;
            let idx: usize = idx_str.parse().ok()?;
            curr = curr.get(idx)?;
        } else {
            curr = curr.get(part)?;
        }
    }
    Some(json_value_to_string(curr))
}

fn json_value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

fn match_body(matcher: &MockBodyMatcher, body: &str) -> bool {
    match matcher {
        MockBodyMatcher::Exact { value } => body == value,
        MockBodyMatcher::Contains { value } => body.contains(value),
        MockBodyMatcher::Regex { pattern } => Regex::new(pattern)
            .map(|re| re.is_match(body))
            .unwrap_or(false),
        MockBodyMatcher::JsonPath {
            path,
            operator,
            value,
        } => {
            let Ok(json) = serde_json::from_str::<serde_json::Value>(body) else {
                return false;
            };
            let extracted = extract_json_path_value(&json, path);
            eval_operator(extracted.as_deref(), *operator, value)
        }
    }
}

/// Evaluate all enabled routes against an incoming request, choosing the highest-priority
/// and most-specific matching rule.
pub fn match_route<'a>(
    routes: &'a [MockRouteRule],
    req: &IncomingMockRequest,
) -> Option<MatchedRoute<'a>> {
    let mut candidates: Vec<(i32, u8, usize, MatchedRoute<'a>)> = Vec::new();

    for rule in routes {
        if !rule.enabled {
            continue;
        }
        let r = &rule.rule;
        let method_ok = r.methods.is_empty()
            || r.methods
                .iter()
                .any(|m| m == "*" || m.eq_ignore_ascii_case(&req.method));
        if !method_ok {
            continue;
        }

        let Some(path_params) = match_path(&r.path, &req.path) else {
            continue;
        };

        let query_ok = r.query.iter().all(|qm| {
            let actual = req.query.get(&qm.name).map(String::as_str);
            eval_operator(actual, qm.operator, &qm.value)
        });
        if !query_ok {
            continue;
        }

        let headers_ok = r.headers.iter().all(|hm| {
            let actual = req.header_ci(&hm.name);
            eval_operator(actual, hm.operator, &hm.value)
        });
        if !headers_ok {
            continue;
        }

        if let Some(bm) = &r.body {
            if !match_body(bm, &req.body) {
                continue;
            }
        }

        let matcher_count = r.query.len() + r.headers.len() + usize::from(r.body.is_some());
        candidates.push((
            rule.priority,
            r.path.specificity(),
            matcher_count,
            MatchedRoute { rule, path_params },
        ));
    }

    candidates.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.cmp(&a.1))
            .then_with(|| b.2.cmp(&a.2))
    });

    candidates.into_iter().next().map(|(_, _, _, matched)| matched)
}

/// Render a mock response template using `TemplateContext` plus injected request, path param,
/// and optional dataset row variables (`{{req.method}}`, `{{req.params.id}}`, `{{data.name}}`, `{{$faker.*}}`).
pub fn render_mock_template(
    template: &str,
    req: &IncomingMockRequest,
    path_params: &HashMap<String, String>,
    data_row: Option<&HashMap<String, String>>,
    extra_vars: &HashMap<String, String>,
) -> String {
    let mut ctx = TemplateContext::new();
    for (k, v) in extra_vars {
        ctx.collection_vars.insert(k.clone(), v.clone());
    }
    ctx.request_vars
        .insert("req.method".to_string(), req.method.clone());
    ctx.request_vars
        .insert("req.path".to_string(), req.path.clone());
    ctx.request_vars
        .insert("req.body".to_string(), req.body.clone());

    for (k, v) in path_params {
        ctx.request_vars.insert(format!("req.params.{k}"), v.clone());
    }
    for (k, v) in &req.query {
        ctx.request_vars.insert(format!("req.query.{k}"), v.clone());
    }
    for (k, v) in &req.headers {
        ctx.request_vars
            .insert(format!("req.headers.{}", k.to_lowercase()), v.clone());
    }
    if let Some(row) = data_row {
        for (k, v) in row {
            ctx.request_vars.insert(format!("data.{k}"), v.clone());
        }
    }

    ctx.render(template)
}

/// Convert Aura collection requests into a portable `MockServerConfigFile`.
pub fn generate_mock_config_from_requests(requests: &[AuraRequest]) -> MockServerConfigFile {
    let mut cfg = MockServerConfigFile::default();
    for req in requests {
        let path = extract_path_from_request_url(&req.url);
        let body = serde_json::json!({
            "mock": true,
            "request": req.name,
            "method": req.method.as_str(),
            "path": path,
            "id": "{{$uuid}}",
            "timestamp": "{{$isoTimestamp}}"
        })
        .to_string();
        let headers = HashMap::from([("Content-Type".to_string(), "application/json".to_string())]);

        let mut rule = MockRouteRule::simple(req.method.as_str(), path, 200, body);
        rule.name = req.name.clone();
        rule.response.headers = headers;
        rule.response.mode = MockResponseMode::Template;
        cfg.routes.push(rule);
    }
    cfg
}

fn extract_path_from_request_url(raw_url: &str) -> String {
    if let Ok(parsed) = Url::parse(raw_url) {
        return parsed.path().to_string();
    }
    // Handle templated URLs like `{{base_url}}/users/:id`
    if let Some(idx) = raw_url.find("://") {
        let rest = &raw_url[idx + 3..];
        if let Some(slash) = rest.find('/') {
            return rest[slash..].split('?').next().unwrap_or("/").to_string();
        }
        return "/".to_string();
    }
    if let Some(idx) = raw_url.find('/') {
        raw_url[idx..].split('?').next().unwrap_or("/").to_string()
    } else {
        "/".to_string()
    }
}

pub fn http_status_reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "OK",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_path_params_and_wildcards_with_priority() {
        let mut r_pattern = MockRouteRule::simple("GET", "/users/:id", 200, "pattern");
        r_pattern.priority = 0;

        let mut r_exact = MockRouteRule::simple("GET", "/users/me", 200, "exact");
        r_exact.priority = 0;

        let routes = vec![r_pattern, r_exact];
        let req_me = IncomingMockRequest::new("GET", "/users/me");
        let matched_me = match_route(&routes, &req_me).unwrap();
        assert_eq!(matched_me.rule.response.body, "exact");

        let req_42 = IncomingMockRequest::new("GET", "/users/42?verbose=true");
        let matched_42 = match_route(&routes, &req_42).unwrap();
        assert_eq!(matched_42.rule.response.body, "pattern");
        assert_eq!(matched_42.path_params.get("id").map(String::as_str), Some("42"));
    }

    #[test]
    fn matches_headers_query_and_json_body_rules() {
        let mut rule = MockRouteRule::simple("POST", "/orders", 201, "vip");
        rule.rule.headers.push(MockHeaderMatcher {
            name: "X-Tier".to_string(),
            operator: MatchOperator::Equals,
            value: "vip".to_string(),
        });
        rule.rule.query.push(MockQueryMatcher {
            name: "dry_run".to_string(),
            operator: MatchOperator::Equals,
            value: "false".to_string(),
        });
        rule.rule.body = Some(MockBodyMatcher::JsonPath {
            path: "$.customer.plan".to_string(),
            operator: MatchOperator::Equals,
            value: "enterprise".to_string(),
        });

        let mut req = IncomingMockRequest::new("POST", "/orders?dry_run=false");
        req.headers.insert("x-tier".to_string(), "vip".to_string());
        req.body = r#"{"customer":{"plan":"enterprise"}}"#.to_string();

        assert!(match_route(&[rule.clone()], &req).is_some());

        req.body = r#"{"customer":{"plan":"free"}}"#.to_string();
        assert!(match_route(&[rule], &req).is_none());
    }

    #[test]
    fn renders_request_vars_data_row_and_faker() {
        let mut req = IncomingMockRequest::new("GET", "/users/99?role=admin");
        req.headers.insert("x-req-id".to_string(), "abc".to_string());
        let params = HashMap::from([("id".to_string(), "99".to_string())]);
        let row = HashMap::from([("name".to_string(), "Alice".to_string())]);

        let out = render_mock_template(
            r#"{"id":"{{req.params.id}}","role":"{{req.query.role}}","user":"{{data.name}}","uuid":"{{$uuid}}"}"#,
            &req,
            &params,
            Some(&row),
            &HashMap::new(),
        );
        assert!(out.contains(r#""id":"99""#));
        assert!(out.contains(r#""role":"admin""#));
        assert!(out.contains(r#""user":"Alice""#));
        assert!(!out.contains("{{$uuid}}"));
    }
}
