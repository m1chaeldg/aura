//! Zero-JS Native Response Extractors (JSONPath, Regex, Header, Cookie).
//! Executed in pure Rust directly on `AuraResponse` without starting a V8 isolate,
//! enabling high-concurrency correlation at load-test speeds.

use crate::assertions::traverse_json_path;
use crate::model::AuraResponse;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum NativeExtractorType {
    #[default]
    JsonPath,
    RegexBody,
    RegexHeader,
    Header,
    Cookie,
}

impl NativeExtractorType {
    pub const ALL: [Self; 5] = [
        Self::JsonPath,
        Self::RegexBody,
        Self::RegexHeader,
        Self::Header,
        Self::Cookie,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::JsonPath => "JSONPath",
            Self::RegexBody => "Regex Body",
            Self::RegexHeader => "Regex Header",
            Self::Header => "Header",
            Self::Cookie => "Cookie",
        }
    }

    pub fn prefix(&self) -> &'static str {
        match self {
            Self::JsonPath => "jsonpath",
            Self::RegexBody => "regex",
            Self::RegexHeader => "regex-header",
            Self::Header => "header",
            Self::Cookie => "cookie",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeExtractor {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub variable: String,
    #[serde(default)]
    pub kind: NativeExtractorType,
    /// JSONPath (`$.data.token`), header/cookie name, or regex pattern (`regex` / `header:pattern`).
    pub expression: String,
    #[serde(default = "default_group_one")]
    pub group: usize,
    #[serde(default)]
    pub default_value: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_group_one() -> usize {
    1
}

impl Default for NativeExtractor {
    fn default() -> Self {
        Self {
            enabled: true,
            variable: "token".to_string(),
            kind: NativeExtractorType::JsonPath,
            expression: "$.token".to_string(),
            group: 1,
            default_value: None,
        }
    }
}

impl NativeExtractor {
    pub fn new(
        variable: impl Into<String>,
        kind: NativeExtractorType,
        expression: impl Into<String>,
    ) -> Self {
        Self {
            enabled: true,
            variable: variable.into(),
            kind,
            expression: expression.into(),
            group: 1,
            default_value: None,
        }
    }
}

/// Evaluates all enabled native extractors against `resp` in pure Rust (zero V8 overhead).
pub fn evaluate_native_extractors(
    extractors: &[NativeExtractor],
    resp: &AuraResponse,
) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if extractors.is_empty() {
        return out;
    }

    let needs_json = extractors
        .iter()
        .any(|e| e.enabled && e.kind == NativeExtractorType::JsonPath);
    let parsed_body = if needs_json {
        serde_json::from_str::<Value>(&resp.body).ok()
    } else {
        None
    };

    for ext in extractors {
        if !ext.enabled {
            continue;
        }
        let var_name = ext.variable.trim();
        if var_name.is_empty() {
            continue;
        }

        let extracted = match ext.kind {
            NativeExtractorType::JsonPath => extract_jsonpath(&ext.expression, parsed_body.as_ref()),
            NativeExtractorType::RegexBody => {
                extract_regex(&ext.expression, &resp.body, ext.group)
            }
            NativeExtractorType::RegexHeader => {
                // Syntax: `header_name:regex` OR searches across `Header: Value` lines
                if let Some((hdr, pat)) = ext.expression.split_once(':') {
                    let val = find_header_ci(&resp.headers, hdr.trim());
                    val.and_then(|v| extract_regex(pat.trim(), v, ext.group))
                } else {
                    let all_headers = resp
                        .headers
                        .iter()
                        .map(|(k, v)| format!("{k}: {v}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    extract_regex(&ext.expression, &all_headers, ext.group)
                }
            }
            NativeExtractorType::Header => {
                find_header_ci(&resp.headers, ext.expression.trim()).cloned()
            }
            NativeExtractorType::Cookie => {
                let target = ext.expression.trim();
                resp.captured_cookies
                    .iter()
                    .find(|c| c.name == target)
                    .map(|c| c.value.clone())
                    .or_else(|| {
                        resp.headers
                            .iter()
                            .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
                            .find_map(|(_, raw)| parse_cookie_pair(raw, target))
                    })
            }
        };

        if let Some(val) = extracted.or_else(|| ext.default_value.clone()) {
            out.insert(var_name.to_string(), val);
        }
    }

    out
}

fn find_header_ci<'a>(headers: &'a HashMap<String, String>, key: &str) -> Option<&'a String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v)
}

fn parse_cookie_pair(set_cookie_raw: &str, name: &str) -> Option<String> {
    let first_pair = set_cookie_raw.split(';').next()?.trim();
    let (k, v) = first_pair.split_once('=')?;
    if k.trim() == name {
        Some(v.trim().to_string())
    } else {
        None
    }
}

fn extract_jsonpath(expr: &str, parsed: Option<&Value>) -> Option<String> {
    let root = parsed?;
    let trimmed = expr.trim();
    let norm = trimmed
        .strip_prefix("$.")
        .or_else(|| trimmed.strip_prefix('$'))
        .unwrap_or(trimmed);
    let val = if norm.is_empty() {
        Some(root.clone())
    } else {
        traverse_json_path(root, norm)
    }?;
    Some(match val {
        Value::String(s) => s,
        Value::Null => "null".to_string(),
        other => other.to_string(),
    })
}

fn extract_regex(pattern: &str, haystack: &str, group: usize) -> Option<String> {
    let re = regex::Regex::new(pattern.trim()).ok()?;
    let caps = re.captures(haystack)?;
    let chosen = caps
        .get(group)
        .or_else(|| caps.get(0))?;
    Some(chosen.as_str().to_string())
}

/// Parses `.aura` `[extractors]` KV entry (`var = "jsonpath:$.data.token"` etc.).
pub fn parse_extractor_kv(var_key: &str, rhs: &str) -> Option<NativeExtractor> {
    let (enabled, clean_var) = if let Some(v) = var_key.strip_prefix('~') {
        (false, v.trim())
    } else if let Some(v) = var_key.strip_suffix('~') {
        (false, v.trim())
    } else {
        (true, var_key.trim())
    };
    if clean_var.is_empty() {
        return None;
    }
    let rhs_t = rhs.trim();
    let (kind, expr) = if let Some(rest) = rhs_t.strip_prefix("jsonpath:") {
        (NativeExtractorType::JsonPath, rest.trim())
    } else if let Some(rest) = rhs_t.strip_prefix("regex-header:") {
        (NativeExtractorType::RegexHeader, rest.trim())
    } else if let Some(rest) = rhs_t.strip_prefix("regex:") {
        (NativeExtractorType::RegexBody, rest.trim())
    } else if let Some(rest) = rhs_t.strip_prefix("header:") {
        (NativeExtractorType::Header, rest.trim())
    } else if let Some(rest) = rhs_t.strip_prefix("cookie:") {
        (NativeExtractorType::Cookie, rest.trim())
    } else {
        (NativeExtractorType::JsonPath, rhs_t)
    };
    Some(NativeExtractor {
        enabled,
        variable: clean_var.to_string(),
        kind,
        expression: expr.to_string(),
        group: 1,
        default_value: None,
    })
}

pub fn serialize_extractor_kv(ext: &NativeExtractor) -> (String, String) {
    let k = if ext.enabled {
        ext.variable.trim().to_string()
    } else {
        format!("~{}", ext.variable.trim())
    };
    let v = format!("{}:{}", ext.kind.prefix(), ext.expression.trim());
    (k, v)
}
