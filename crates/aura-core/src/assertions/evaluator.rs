//! Evaluation engine for declarative assertions and flow-control conditions.

use super::parser::parse_assertion_line;
use super::types::{AssertOperator, DeclarativeAssertion};
use crate::model::{AuraResponse, TestAssertion};
use serde_json::Value;

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

/// Evaluate a non-code logic-controller condition (JMeter `If` / `While` controller)
/// against an optional response (`res.*` / `prev.*` / `response.*`) and variable scope (`vars.*` / `var.*`).
pub fn evaluate_flow_condition(
    expr: &str,
    resp: Option<&AuraResponse>,
    vars: &std::collections::HashMap<String, String>,
) -> bool {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return true;
    }

    if let Some(rule) = parse_assertion_line(trimmed) {
        let actual = resolve_condition_lhs(&rule.expression, resp, vars);
        let (passed, _) = eval_rule(&actual, rule.operator, &rule.expected);
        return passed;
    }

    let (negated, key_expr) = if let Some(rest) = trimmed.strip_prefix('!') {
        (true, rest.trim())
    } else {
        (false, trimmed)
    };
    let val = resolve_condition_lhs(key_expr, resp, vars);
    let truthy = match val {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => b,
        Some(Value::String(s)) => {
            !s.is_empty() && !s.eq_ignore_ascii_case("false") && s != "0"
        }
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0) != 0.0,
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(m)) => !m.is_empty(),
    };
    if negated { !truthy } else { truthy }
}

fn resolve_condition_lhs(
    lhs: &str,
    resp: Option<&AuraResponse>,
    vars: &std::collections::HashMap<String, String>,
) -> Option<Value> {
    let trimmed = lhs.trim();
    if let Some(var_name) = trimmed
        .strip_prefix("vars.")
        .or_else(|| trimmed.strip_prefix("var."))
    {
        return vars.get(var_name.trim()).map(|s| {
            serde_json::from_str::<Value>(s).unwrap_or_else(|_| Value::String(s.clone()))
        });
    }
    let normalized = if let Some(rest) = trimmed.strip_prefix("prev.") {
        format!("res.{rest}")
    } else {
        trimmed.to_string()
    };
    let r = resp?;
    let parsed_body = serde_json::from_str::<Value>(&r.body).ok();
    resolve_expression(&normalized, r, parsed_body.as_ref())
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

    parsed_body.and_then(|root| traverse_json_path(root, rest))
}

pub(crate) fn traverse_json_path(root: &Value, path: &str) -> Option<Value> {
    let mut current = root;
    for seg in parse_path_segments(path) {
        match seg {
            PathSeg::Key(k) => current = current.get(&k)?,
            PathSeg::Index(idx) => current = current.get(idx)?,
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
            eval_binary_rule(val, op, expected)
        }
    }
}

fn eval_binary_rule(val: &Value, op: AssertOperator, expected: &str) -> (bool, Option<String>) {
    match op {
        AssertOperator::Eq => {
            let ok = values_equal(val, expected);
            (ok, (!ok).then(|| format!("Expected {expected}, got {}", value_as_string(val))))
        }
        AssertOperator::Neq => {
            let ok = !values_equal(val, expected);
            (ok, (!ok).then(|| format!("Expected not equal to {expected}")))
        }
        AssertOperator::Gt | AssertOperator::Gte | AssertOperator::Lt | AssertOperator::Lte => {
            let Some(a) = value_as_f64(val) else {
                return (false, Some(format!("Left value '{}' is not numeric", value_as_string(val))));
            };
            let Ok(b) = expected.trim().parse::<f64>() else {
                return (false, Some(format!("Expected value '{expected}' is not numeric")));
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
            (ok, (!ok).then(|| format!("Expected '{}' {} '{expected}'", value_as_string(val), op.symbol())))
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
            (ok, (!ok).then(|| format!("Expected '{}' {} [{expected}]", value_as_string(val), op.symbol())))
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
