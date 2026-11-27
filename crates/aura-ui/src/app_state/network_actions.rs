//! Network UI state actions: Cookie Jar management, global & per-request HTTP/SOCKS proxy,
//! MITM Capture Proxy / Interceptor management, and Code Snippet generation.

use super::security_actions::FieldOp;
use super::AuraAppState;
use crate::preferences::AuraPreferences;
use crate::stores::WorkspaceTab;
use aura_core::codegen::{generate_snippet, CodegenLanguage};
use aura_core::cookies::CookieEntry;
use aura_core::interceptor::InterceptorRule;
use aura_core::model::{ProxyConfig, ProxyProtocol};
use chrono::Utc;
use std::collections::HashMap;

impl AuraAppState {
    pub fn persist_network_preferences(&self) {
        let mut prefs = AuraPreferences::load();
        prefs.global_proxy = self.global_proxy.clone();
        prefs.cookie_jar = self.cookie_jar.clone();
        let _ = prefs.save();
    }

    // --- Cookie Jar Manager ---

    pub fn ingest_captured_cookies(&mut self, cookies: Vec<CookieEntry>) {
        if cookies.is_empty() {
            return;
        }
        let now = Utc::now();
        for c in cookies {
            self.cookie_jar.set_cookie(c, now);
        }
        self.persist_network_preferences();
    }

    pub fn save_draft_cookie(&mut self) -> Result<(), String> {
        let domain = self.draft_cookie_domain.trim().trim_start_matches('.').to_ascii_lowercase();
        let name = self.draft_cookie_name.trim().to_string();
        let path = if self.draft_cookie_path.trim().is_empty() {
            "/".to_string()
        } else {
            self.draft_cookie_path.trim().to_string()
        };
        if domain.is_empty() || name.is_empty() {
            return Err("Cookie domain and name are required".to_string());
        }
        let entry = CookieEntry {
            name,
            value: self.draft_cookie_value.clone(),
            domain,
            path,
            expires: None,
            max_age: None,
            secure: self.draft_cookie_secure,
            http_only: false,
            same_site: Some("Lax".to_string()),
            host_only: true,
            expires_at: None,
        };
        self.cookie_jar.set_cookie(entry, Utc::now());
        self.draft_cookie_name.clear();
        self.draft_cookie_value.clear();
        self.persist_network_preferences();
        Ok(())
    }

    pub fn remove_cookie_entry(&mut self, domain: &str, path: &str, name: &str) {
        self.cookie_jar.remove_cookie(domain, path, name);
        self.persist_network_preferences();
    }

    pub fn clear_filtered_cookies(&mut self) {
        let filter = self.cookie_domain_filter.trim().to_string();
        if filter.is_empty() {
            self.cookie_jar.clear_all();
        } else {
            self.cookie_jar.clear_domain(&filter);
        }
        self.persist_network_preferences();
    }

    // --- Proxy Configuration ---

    pub fn toggle_global_proxy_enabled(&mut self) {
        self.global_proxy.enabled = !self.global_proxy.enabled;
        self.persist_network_preferences();
    }

    pub fn set_global_proxy_protocol(&mut self, proto: ProxyProtocol) {
        self.global_proxy.protocol = proto;
        self.persist_network_preferences();
    }

    pub fn toggle_global_proxy_auth(&mut self) {
        self.global_proxy.auth_enabled = !self.global_proxy.auth_enabled;
        self.persist_network_preferences();
    }

    pub fn set_global_proxy_bypass_csv(&mut self, csv: &str) {
        self.global_proxy.bypass_list = csv
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    pub fn toggle_request_proxy_enabled(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let proxy = tab.request.settings.proxy.get_or_insert_with(ProxyConfig::default);
            proxy.enabled = !proxy.enabled;
            tab.mark_dirty(true);
        }
    }

    pub fn set_request_proxy_protocol(&mut self, proto: ProxyProtocol) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let proxy = tab.request.settings.proxy.get_or_insert_with(ProxyConfig::default);
            proxy.protocol = proto;
            tab.mark_dirty(true);
        }
    }

    pub fn toggle_request_proxy_auth(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let proxy = tab.request.settings.proxy.get_or_insert_with(ProxyConfig::default);
            proxy.auth_enabled = !proxy.auth_enabled;
            tab.mark_dirty(true);
        }
    }

    // --- Network Interceptor / Capture Proxy ---

    pub fn add_interceptor_rule_from_draft(&mut self) -> Result<(), String> {
        let m = self.draft_rule_match.trim().to_string();
        if m.is_empty() {
            return Err("URL match pattern is required".to_string());
        }
        let target = if self.draft_rule_target.trim().is_empty() {
            None
        } else {
            Some(self.draft_rule_target.trim().to_string())
        };
        self.interceptor_manager.add_rule(InterceptorRule {
            match_url_pattern: m,
            rewrite_url_target: target,
            override_headers: HashMap::new(),
        });
        self.draft_rule_match.clear();
        self.draft_rule_target.clear();
        Ok(())
    }

    pub fn open_captured_transaction_in_tab(&mut self, idx: usize) {
        let captures = self.interceptor_manager.list_captures();
        let Some(tx) = captures.get(idx) else {
            return;
        };
        let req = tx.to_aura_request();
        for tab in &mut self.tabs {
            tab.is_active = false;
        }
        self.tabs.push(WorkspaceTab::new(req));
        self.active_tab_index = self.tabs.len() - 1;
        self.is_interceptor_modal_open = false;
        self.status_message = Some(format!("Opened captured request: {}", tx.url));
    }

    // --- Code Snippet Generator ---

    pub fn active_code_snippet(&self) -> String {
        let Some(tab) = self.active_tab() else {
            return "// No active request".to_string();
        };
        let ctx = if self.codegen_resolve_vars {
            Some(self.active_template_context())
        } else {
            None
        };
        generate_snippet(&tab.request, ctx.as_ref(), self.codegen_language)
    }

    pub fn set_codegen_language(&mut self, lang: CodegenLanguage) {
        self.codegen_language = lang;
    }
}

pub(super) fn edit_request_proxy_field(
    proxy_opt: &mut Option<ProxyConfig>,
    idx: usize,
    op: FieldOp<'_>,
) {
    let proxy = proxy_opt.get_or_insert_with(ProxyConfig::default);
    match idx {
        0 => apply_req_str(&mut proxy.host, op),
        1 => {
            let mut buf = proxy.port.to_string();
            apply_req_str(&mut buf, op);
            proxy.port = buf.trim().parse::<u16>().unwrap_or(0);
        }
        2 => apply_req_str(&mut proxy.username, op),
        3 => apply_req_str(&mut proxy.password, op),
        _ => {
            let mut csv = proxy.bypass_list.join(", ");
            apply_req_str(&mut csv, op);
            proxy.bypass_list = csv
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }
}

pub(super) fn read_request_proxy_field(proxy_opt: Option<&ProxyConfig>, idx: usize) -> String {
    let Some(proxy) = proxy_opt else {
        return match idx {
            1 => "8080".to_string(),
            _ => String::new(),
        };
    };
    match idx {
        0 => proxy.host.clone(),
        1 => proxy.port.to_string(),
        2 => proxy.username.clone(),
        3 => proxy.password.clone(),
        _ => proxy.bypass_list.join(", "),
    }
}

fn apply_req_str(s: &mut String, op: FieldOp<'_>) {
    match op {
        FieldOp::PushChar(c) => s.push(c),
        FieldOp::PopChar => {
            s.pop();
        }
        FieldOp::PushStr(t) => s.push_str(t),
        FieldOp::SetStr(t) => *s = t,
    }
}
