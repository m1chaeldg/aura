//! Parsers and serializers for declarative assertion lines and `[assert]` key-value blocks.

use super::types::{AssertOperator, DeclarativeAssertion};

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

/// Parse an `expr` + `"op expected"` pair (TOML `[assert]` key-value syntax, Bruno style).
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
