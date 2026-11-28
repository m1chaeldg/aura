//! Assertion operator and declarative rule types.

use serde::{Deserialize, Serialize};

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
