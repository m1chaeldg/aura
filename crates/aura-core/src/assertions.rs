//! Declarative / No-Code Assertions (Bruno-style `assert` rules).
//! Evaluates lightweight assertions such as:
//! - `res.status == 200`
//! - `res.body.id isNumber`
//! - `res.body.items[0].name contains "widget"`
//! - `res.headers.content-type contains "application/json"`
//! - `res.duration_ms < 500`

use crate::model::{AuraResponse, TestAssertion};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AssertOperator {
    #[default]
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
    NotContains,
    StartsWith,
    EndsWith,
    Matches,
    In,
    NotIn,
    IsDefined,
    IsUndefined,
    IsNull,
    IsNotNull,
    IsEmpty,
    IsNotEmpty,
    IsNumber,
    IsString,
    IsBoolean,
    IsArray,
    IsObject,
    LengthEq,
}

impl AssertOperator {
    pub const ALL: [Self; 25] = [
        Self::Eq,
        Self::Neq,
        Self::Gt,
        Self::Gte,
        Self::Lt,
        Self::Lte,
        Self::Contains,
        Self::NotContains,
        Self::StartsWith,
        Self::EndsWith,
        Self::Matches,
        Self::In,
        Self::NotIn,
        Self::IsDefined,
        Self::IsUndefined,
        Self::IsNull,
        Self::IsNotNull,
        Self::IsEmpty,
        Self::IsNotEmpty,
        Self::IsNumber,
        Self::IsString,
        Self::IsBoolean,
        Self::IsArray,
        Self::IsObject,
        Self::LengthEq,
    ];

    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Eq => "==",
            Self::Neq => "!=",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Lt => "<",
            Self::Lte => "<=",
            Self::Contains => "contains",
            Self::NotContains => "notContains",
            Self::StartsWith => "startsWith",
            Self::EndsWith => "endsWith",
            Self::Matches => "matches",
            Self::In => "in",
            Self::NotIn => "notIn",
            Self::IsDefined => "isDefined",
            Self::IsUndefined => "isUndefined",
            Self::IsNull => "isNull",
            Self::IsNotNull => "isNotNull",
            Self::IsEmpty => "isEmpty",
            Self::IsNotEmpty => "isNotEmpty",
            Self::IsNumber => "isNumber",
            Self::IsString => "isString",
            Self::IsBoolean => "isBoolean",
            Self::IsArray => "isArray",
            Self::IsObject => "isObject",
            Self::LengthEq => "length",
        }
    }

    pub fn is_unary(&self) -> bool {
        matches!(
            self,
            Self::IsDefined
                | Self::IsUndefined
                | Self::IsNull
                | Self::IsNotNull
                | Self::IsEmpty
                | Self::IsNotEmpty
                | Self::IsNumber
                | Self::IsString
                | Self::IsBoolean
                | Self::IsArray
                | Self::IsObject
        )
    }

    pub fn from_token(tok: &str) -> Option<Self> {
        match tok.trim().to_ascii_lowercase().as_str() {
            "==" | "===" | "eq" | "equals" => Some(Self::Eq),
            "!=" | "!==" | "neq" | "not_equals" => Some(Self::Neq),
            ">" | "gt" => Some(Self::Gt),
            ">=" | "gte" => Some(Self::Gte),
            "<" | "lt" => Some(Self::Lt),
            "<=" | "lte" => Some(Self::Lte),
            "contains" => Some(Self::Contains),
            "notcontains" | "not_contains" | "!contains" => Some(Self::NotContains),
            "startswith" | "starts_with" => Some(Self::StartsWith),
            "endswith" | "ends_with" => Some(Self::EndsWith),
            "matches" | "regex" => Some(Self::Matches),
            "in" => Some(Self::In),
            "notin" | "not_in" => Some(Self::NotIn),
            "isdefined" | "is_defined" => Some(Self::IsDefined),
            "isundefined" | "is_undefined" => Some(Self::IsUndefined),
            "isnull" | "is_null" => Some(Self::IsNull),
            "isnotnull" | "is_not_null" => Some(Self::IsNotNull),
            "isempty" | "is_empty" => Some(Self::IsEmpty),
            "isnotempty" | "is_not_empty" => Some(Self::IsNotEmpty),
            "isnumber" | "is_number" => Some(Self::IsNumber),
            "isstring" | "is_string" => Some(Self::IsString),
            "isboolean" | "is_boolean" | "isbool" => Some(Self::IsBoolean),
            "isarray" | "is_array" => Some(Self::IsArray),
            "isobject" | "is_object" | "isjson" => Some(Self::IsObject),
            "length" | "len" => Some(Self::LengthEq),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclarativeAssertion {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub expression: String,
    #[serde(default)]
    pub operator: AssertOperator,
    #[serde(default)]
    pub expected: String,
}

fn default_true() -> bool {
    true
}

impl Default for DeclarativeAssertion {
    fn default() -> Self {
        Self {
            enabled: true,
            expression: "res.status".to_string(),
            operator: AssertOperator::Eq,
            expected: "200".to_string(),
        }
    }
}

impl DeclarativeAssertion {
    pub fn new(
        expression: impl Into<String>,
        operator: AssertOperator,
        expected: impl Into<String>,
    ) -> Self {
        Self {
            enabled: true,
            expression: expression.into(),
            operator,
            expected: expected.into(),
        }
    }

    pub fn summary(&self) -> String {
        if self.operator.is_unary() {
            format!("{} {}", self.expression.trim(), self.operator.symbol())
        } else {
            format!(
                "{} {} {}",
                self.expression.trim(),
                self.operator.symbol(),
                self.expected.trim()
            )
        }
    }
}

/// Parse a single assertion line such as:
/// - `res.status == 200`
/// - `res.body.id isNumber`
/// - `res.headers.content-type contains "application/json"`
pub fn parse_assertion_line(raw: &str) -> Option<DeclarativeAssertion> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
        return None;
    }

    let (enabled, body) = if let Some(rest) = trimmed.strip_prefix('~') {
        (false, rest.trim())
    } else {
        (true, trimmed)
    };

    // Split into (expr, op_token, expected)
    // Support symbol operators even without whitespace (e.g. `res.status==200`, `res.duration_ms<500`)
    for sym in ["===", "!==", "==", "!=", ">=", "<=", ">", "<"] {
        if let Some(pos) = body.find(sym) {
            let expr = body[..pos].trim().to_string();
            let rest = body[pos + sym.len()..].trim();
            let op = AssertOperator::from_token(sym)?;
            if !expr.is_empty() {
                return Some(DeclarativeAssertion {
                    enabled,
                    expression: expr,
                    operator: op,
                    expected: unquote(rest),
                });
            }
        }
    }

    let mut parts = body.splitn(3, char::is_whitespace);
    let expr = parts.next()?.trim().to_string();
    let op_tok = parts.next()?.trim();
    let op = AssertOperator::from_token(op_tok)?;
    let expected = parts.next().map(unquote).unwrap_or_default();

    Some(DeclarativeAssertion {
        enabled,
        expression: expr,
        operator: op,
        expected,
    })
}

/// Parse a `expr` + `"op expected"` pair (TOML `[assert]` key-value syntax, Bruno style).
pub fn parse_assertion_kv(expr: &str, rhs: &str) -> Option<DeclarativeAssertion> {
    let (enabled, clean_expr) = if let Some(e) = expr.strip_prefix('~') {
        (false, e.trim())
    } else if let Some(e) = expr.strip_suffix('~') {
        (false, e.trim())
    } else {
        (true, expr.trim())
    };
    if clean_expr.is_empty() {
        return None;
    }
    let rhs_trim = rhs.trim();
    let mut parts = rhs_trim.splitn(2, char::is_whitespace);
    let op_tok = parts.next()?;
    let (op, expected) = if let Some(op) = AssertOperator::from_token(op_tok) {
        (op, parts.next().map(unquote).unwrap_or_default())
    } else {
        (AssertOperator::Eq, unquote(rhs_trim))
    };

    Some(DeclarativeAssertion {
        enabled,
        expression: clean_expr.to_string(),
        operator: op,
        expected,
    })
}

pub fn parse_assert_block(block: &str) -> Vec<DeclarativeAssertion> {
    block.lines().filter_map(parse_assertion_line).collect()
}

pub fn serialize_assertion_kv(rule: &DeclarativeAssertion) -> (String, String) {
    let key = if rule.enabled {
        rule.expression.trim().to_string()
    } else {
        format!("~{}", rule.expression.trim())
    };
    let val = if rule.operator.is_unary() {
        rule.operator.symbol().to_string()
    } else {
        format!("{} {}", rule.operator.symbol(), rule.expected.trim())
    };
    (key, val)
}

fn unquote(s: &str) -> String {
    let t = s.trim();
    if t.len() >= 2
        && ((t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')))
    {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

/// Evaluate all enabled declarative assertion rules against `resp`.
pub fn evaluate_assertions(
    rules: &[DeclarativeAssertion],
    resp: &AuraResponse,
) -> Vec<TestAssertion> {
    let parsed_body = serde_json::from_str::<Value>(&resp.body).ok();
    let mut out = Vec::new();

    for rule in rules {
        if !rule.enabled {
            continue;
        }
        let started = std::time::Instant::now();
        let actual = resolve_expression(&rule.expression, resp, parsed_body.as_ref());
        let (passed, message) = eval_rule(&actual, rule.operator, &rule.expected);
        out.push(TestAssertion {
            name: rule.summary(),
            passed,
            message,
            duration_ms: started.elapsed().as_millis() as u64,
        });
    }

    out
}

fn resolve_expression(expr: &str, resp: &AuraResponse, parsed_body: Option<&Value>) -> Option<Value> {
    let trimmed = expr.trim();
    let rest = trimmed
        .strip_prefix("res.")
        .or_else(|| trimmed.strip_prefix("response."))
        .unwrap_or(trimmed);

    match rest {
        "status" => return Some(Value::Number(resp.status.into())),
        "statusText" | "status_text" => return Some(Value::String(resp.status_text.clone())),
        "responseTime" | "duration_ms" | "time" => return Some(Value::Number(resp.duration_ms.into())),
        "size" | "size_bytes" => return Some(Value::Number((resp.size_bytes as u64).into())),
        "body" => {
            return Some(
                parsed_body
                    .cloned()
                    .unwrap_or_else(|| Value::String(resp.body.clone())),
            );
        }
        _ => {}
    }

    if let Some(hdr_part) = rest.strip_prefix("headers") {
        let hdr_key = hdr_part
            .trim_start_matches('.')
            .trim_start_matches('[')
            .trim_end_matches(']')
            .trim_matches(['"', '\'']);
        return resp
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(hdr_key))
            .map(|(_, v)| Value::String(v.clone()));
    }

    if let Some(body_path) = rest.strip_prefix("body.").or_else(|| rest.strip_prefix("body[")) {
        let root = parsed_body?;
        let path_norm = if rest.starts_with("body[") {
            format!("[{body_path}")
        } else {
            body_path.to_string()
        };
        return traverse_json_path(root, &path_norm);
    }

    // Fallback: direct JSON path against response body
    parsed_body.and_then(|root| traverse_json_path(root, rest))
}

fn traverse_json_path(root: &Value, path: &str) -> Option<Value> {
    let mut current = root;
    let tokens = parse_path_segments(path);
    for seg in tokens {
        match seg {
            PathSeg::Key(k) => {
                current = current.get(&k)?;
            }
            PathSeg::Index(idx) => {
                current = current.get(idx)?;
            }
        }
    }
    Some(current.clone())
}

enum PathSeg {
    Key(String),
    Index(usize),
}

fn parse_path_segments(path: &str) -> Vec<PathSeg> {
    let mut segs = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = path.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '.' => {
                if !cur.is_empty() {
                    segs.push(PathSeg::Key(std::mem::take(&mut cur)));
                }
                i += 1;
            }
            '[' => {
                if !cur.is_empty() {
                    segs.push(PathSeg::Key(std::mem::take(&mut cur)));
                }
                i += 1;
                let mut inner = String::new();
                while i < chars.len() && chars[i] != ']' {
                    inner.push(chars[i]);
                    i += 1;
                }
                if i < chars.len() && chars[i] == ']' {
                    i += 1;
                }
                let trimmed = inner.trim().trim_matches(['"', '\'']);
                if let Ok(idx) = trimmed.parse::<usize>() {
                    segs.push(PathSeg::Index(idx));
                } else if !trimmed.is_empty() {
                    segs.push(PathSeg::Key(trimmed.to_string()));
                }
            }
            ch => {
                cur.push(ch);
                i += 1;
            }
        }
    }
    if !cur.is_empty() {
        segs.push(PathSeg::Key(cur));
    }
    segs
}

fn value_as_f64(val: &Value) -> Option<f64> {
    match val {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn value_as_string(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

fn values_equal(actual: &Value, expected_raw: &str) -> bool {
    let exp_trim = expected_raw.trim();
    match actual {
        Value::Null => exp_trim.eq_ignore_ascii_case("null"),
        Value::Bool(b) => exp_trim.eq_ignore_ascii_case(&b.to_string()),
        Value::Number(n) => {
            if let Ok(exp_num) = exp_trim.parse::<f64>() {
                n.as_f64().is_some_and(|a| (a - exp_num).abs() < f64::EPSILON)
            } else {
                false
            }
        }
        Value::String(s) => s == expected_raw || s == exp_trim,
        other => other.to_string() == exp_trim,
    }
}

fn eval_rule(actual: &Option<Value>, op: AssertOperator, expected: &str) -> (bool, Option<String>) {
    match op {
        AssertOperator::IsDefined => {
            let ok = actual.is_some();
            (ok, (!ok).then(|| "Expected value to be defined".to_string()))
        }
        AssertOperator::IsUndefined => {
            let ok = actual.is_none();
            (ok, (!ok).then(|| format!("Expected undefined, got {:?}", actual)))
        }
        AssertOperator::IsNull => {
            let ok = matches!(actual, Some(Value::Null));
            (ok, (!ok).then(|| format!("Expected null, got {:?}", actual)))
        }
        AssertOperator::IsNotNull => {
            let ok = actual.as_ref().is_some_and(|v| !v.is_null());
            (ok, (!ok).then(|| "Expected non-null value".to_string()))
        }
        AssertOperator::IsNumber => {
            let ok = matches!(actual, Some(Value::Number(_)));
            (ok, (!ok).then(|| format!("Expected number, got {:?}", actual)))
        }
        AssertOperator::IsString => {
            let ok = matches!(actual, Some(Value::String(_)));
            (ok, (!ok).then(|| format!("Expected string, got {:?}", actual)))
        }
        AssertOperator::IsBoolean => {
            let ok = matches!(actual, Some(Value::Bool(_)));
            (ok, (!ok).then(|| format!("Expected boolean, got {:?}", actual)))
        }
        AssertOperator::IsArray => {
            let ok = matches!(actual, Some(Value::Array(_)));
            (ok, (!ok).then(|| format!("Expected array, got {:?}", actual)))
        }
        AssertOperator::IsObject => {
            let ok = matches!(actual, Some(Value::Object(_)));
            (ok, (!ok).then(|| format!("Expected object, got {:?}", actual)))
        }
        AssertOperator::IsEmpty | AssertOperator::IsNotEmpty => {
            let empty = match actual {
                None | Some(Value::Null) => true,
                Some(Value::String(s)) => s.is_empty(),
                Some(Value::Array(a)) => a.is_empty(),
                Some(Value::Object(m)) => m.is_empty(),
                Some(_) => false,
            };
            let ok = if op == AssertOperator::IsEmpty { empty } else { !empty };
            (ok, (!ok).then(|| format!("Empty check failed for {:?}", actual)))
        }
        _ => {
            let Some(val) = actual else {
                return (false, Some("Expression resolved to undefined".to_string()));
            };
            match op {
                AssertOperator::Eq => {
                    let ok = values_equal(val, expected);
                    (
                        ok,
                        (!ok).then(|| format!("Expected {expected}, got {}", value_as_string(val))),
                    )
                }
                AssertOperator::Neq => {
                    let ok = !values_equal(val, expected);
                    (
                        ok,
                        (!ok).then(|| format!("Expected not equal to {expected}")),
                    )
                }
                AssertOperator::Gt | AssertOperator::Gte | AssertOperator::Lt | AssertOperator::Lte => {
                    let Some(a) = value_as_f64(val) else {
                        return (
                            false,
                            Some(format!("Left value '{}' is not numeric", value_as_string(val))),
                        );
                    };
                    let Ok(b) = expected.trim().parse::<f64>() else {
                        return (
                            false,
                            Some(format!("Expected value '{expected}' is not numeric")),
                        );
                    };
                    let ok = match op {
                        AssertOperator::Gt => a > b,
                        AssertOperator::Gte => a >= b,
                        AssertOperator::Lt => a < b,
                        AssertOperator::Lte => a <= b,
                        _ => false,
                    };
                    (ok, (!ok).then(|| format!("Expected {a} {} {b}", op.symbol())))
                }
                AssertOperator::Contains | AssertOperator::NotContains => {
                    let found = match val {
                        Value::Array(items) => items.iter().any(|it| values_equal(it, expected)),
                        _ => value_as_string(val).contains(expected),
                    };
                    let ok = if op == AssertOperator::Contains { found } else { !found };
                    (
                        ok,
                        (!ok).then(|| {
                            format!(
                                "Expected '{}' {} '{expected}'",
                                value_as_string(val),
                                op.symbol()
                            )
                        }),
                    )
                }
                AssertOperator::StartsWith => {
                    let s = value_as_string(val);
                    let ok = s.starts_with(expected);
                    (ok, (!ok).then(|| format!("Expected '{s}' to start with '{expected}'")))
                }
                AssertOperator::EndsWith => {
                    let s = value_as_string(val);
                    let ok = s.ends_with(expected);
                    (ok, (!ok).then(|| format!("Expected '{s}' to end with '{expected}'")))
                }
                AssertOperator::Matches => {
                    let s = value_as_string(val);
                    match regex::Regex::new(expected) {
                        Ok(re) => {
                            let ok = re.is_match(&s);
                            (ok, (!ok).then(|| format!("Expected '{s}' to match /{expected}/")))
                        }
                        Err(e) => (false, Some(format!("Invalid regex '{expected}': {e}"))),
                    }
                }
                AssertOperator::In | AssertOperator::NotIn => {
                    let list: Vec<&str> = expected
                        .trim_start_matches('[')
                        .trim_end_matches(']')
                        .split(',')
                        .map(|s| s.trim().trim_matches(['"', '\'']))
                        .collect();
                    let present = list.iter().any(|item| values_equal(val, item));
                    let ok = if op == AssertOperator::In { present } else { !present };
                    (
                        ok,
                        (!ok).then(|| format!("Expected '{}' {} [{expected}]", value_as_string(val), op.symbol())),
                    )
                }
                AssertOperator::LengthEq => {
                    let Ok(target_len) = expected.trim().parse::<usize>() else {
                        return (false, Some(format!("Invalid length '{expected}'")));
                    };
                    let actual_len = match val {
                        Value::String(s) => s.len(),
                        Value::Array(a) => a.len(),
                        Value::Object(m) => m.len(),
                        _ => value_as_string(val).len(),
                    };
                    let ok = actual_len == target_len;
                    (ok, (!ok).then(|| format!("Expected length {target_len}, got {actual_len}")))
                }
                _ => (false, None),
            }
        }
    }
}
