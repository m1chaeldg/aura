//! Handlebars and dynamic variable templating engine for Aura.
//! Resolves `{{var}}` syntax across URLs, params, headers, and body payloads,
//! with cascading lookup: Request -> Environment -> Collection -> OS Environment.

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

static VAR_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{([a-zA-Z0-9_$.-]+)\}\}").unwrap());

/// Context for resolving variables in a request
#[derive(Debug, Clone, Default)]
pub struct TemplateContext {
    pub request_vars: HashMap<String, String>,
    pub environment_vars: HashMap<String, String>,
    pub collection_vars: HashMap<String, String>,
}

impl TemplateContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve a single variable name following the cascade:
    /// Request -> Environment -> Collection -> OS environment ($VAR)
    pub fn resolve_variable(&self, key: &str) -> Option<String> {
        // 1. Request variables (highest precedence)
        if let Some(val) = self.request_vars.get(key) {
            return Some(val.clone());
        }

        // 2. Environment variables
        if let Some(val) = self.environment_vars.get(key) {
            return Some(val.clone());
        }

        // 3. Collection variables
        if let Some(val) = self.collection_vars.get(key) {
            return Some(val.clone());
        }

        // 4. OS Environment variables: if prefixed with $ e.g. $USER, $OS_USER, or directly
        if let Some(stripped) = key.strip_prefix('$') {
            if let Ok(val) = std::env::var(stripped) {
                return Some(val);
            }
        }
        if let Ok(val) = std::env::var(key) {
            return Some(val);
        }

        None
    }

    /// Render a templated string by substituting all `{{...}}` tokens
    pub fn render(&self, template: &str) -> String {
        VAR_REGEX
            .replace_all(template, |caps: &regex::Captures| {
                let var_name = &caps[1];
                if let Some(val) = self.resolve_variable(var_name) {
                    val
                } else {
                    caps[0].to_string()
                }
            })
            .to_string()
    }
}
