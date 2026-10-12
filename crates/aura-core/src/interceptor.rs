//! Network Interceptor and Request Rewriter proxy.

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct InterceptorRule {
    pub match_url_pattern: String,
    pub rewrite_url_target: Option<String>,
    pub override_headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct InterceptorManager {
    rules: Arc<parking_lot::RwLock<Vec<InterceptorRule>>>,
}

impl InterceptorManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_rule(&self, rule: InterceptorRule) {
        self.rules.write().push(rule);
    }

    pub fn apply_rewrites(&self, url: &str, headers: &mut HashMap<String, String>) -> String {
        let mut final_url = url.to_string();
        let rules = self.rules.read();

        for rule in rules.iter() {
            if final_url.contains(&rule.match_url_pattern) {
                if let Some(ref target) = rule.rewrite_url_target {
                    final_url = final_url.replace(&rule.match_url_pattern, target);
                }
                for (k, v) in &rule.override_headers {
                    headers.insert(k.clone(), v.clone());
                }
            }
        }

        final_url
    }
}
