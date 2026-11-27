use super::AuraAppState;
use crate::services::{AiContextSnapshot, AuraAgentAction};
use aura_core::model::{AuraRequest, BodyType, HeaderItem, HttpMethod, ParamItem};
use std::collections::BTreeMap;

impl AuraAppState {
    /// Toggles the collapsible right-side AI Assistant drawer.
    pub fn toggle_ai_drawer(&mut self) {
        self.is_ai_drawer_open = !self.is_ai_drawer_open;
    }

    /// Returns a display label for the currently selected AI provider.
    pub fn active_ai_provider_label(&self) -> String {
        let Some(ref id) = self.ai_prefs.active_provider_id else {
            return "No Provider Selected".to_string();
        };
        if let Some(cli) = self.ai_prefs.cli_profiles.iter().find(|p| &p.id == id) {
            return cli.name.clone();
        }
        if let Some(http) = self.ai_prefs.http_profiles.iter().find(|p| &p.id == id) {
            return format!("{} ({})", http.name, http.model);
        }
        id.clone()
    }

    /// Snapshots the active request, response, spec, environment, and available Aura commands
    /// for sending to an AI agent.
    pub fn build_ai_context_snapshot(&self) -> AiContextSnapshot {
        let mut snap = AiContextSnapshot::default();

        if let Some(tab) = self.tabs.get(self.active_tab_index) {
            let req = &tab.request;
            snap.request_name = Some(req.name.clone());
            snap.method = Some(req.method.as_str().to_string());
            snap.url = Some(req.url.clone());
            snap.headers = req
                .headers
                .iter()
                .filter(|r| r.enabled && !r.key.trim().is_empty())
                .map(|r| (r.key.clone(), r.value.clone()))
                .collect();
            snap.params = req
                .params
                .iter()
                .filter(|r| r.enabled && !r.key.trim().is_empty())
                .map(|r| (r.key.clone(), r.value.clone()))
                .collect();
            snap.body = body_to_option_string(&req.body);
            if !req.scripts.pre_request.trim().is_empty() {
                snap.pre_script = Some(req.scripts.pre_request.clone());
            }
            if !req.scripts.post_response.trim().is_empty() {
                snap.post_script = Some(req.scripts.post_response.clone());
            }
            if !req.scripts.tests.trim().is_empty() {
                snap.test_script = Some(req.scripts.tests.clone());
            }
            if let Some(ref buf) = tab.spec_buffer {
                if !buf.trim().is_empty() {
                    snap.spec = Some(buf.clone());
                }
            }
            if snap.spec.is_none() {
                snap.spec = Some(aura_parser::request_to_spec(req));
            }

            if let Some(ref resp) = tab.response {
                snap.response_status = Some(resp.status);
                snap.response_headers = resp
                    .headers
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                snap.response_body = Some(resp.body.clone());
            }
        }

        if let Some(env) = self.active_environment() {
            snap.env_name = Some(env.name.clone());
            let mut vars = BTreeMap::new();
            for v in &env.variables {
                if v.enabled && !v.key.trim().is_empty() {
                    vars.insert(v.key.clone(), v.value.clone());
                }
            }
            snap.env_vars = vars;
        }

        snap.available_commands = vec![
            ("cmd:send_request".into(), "Send active HTTP request".into()),
            ("cmd:new_request".into(), "Create a new request tab".into()),
            ("cmd:save_request".into(), "Save active request to disk".into()),
            ("cmd:export_code".into(), "Open Code Export modal".into()),
            ("cmd:cookies".into(), "Open Cookie Jar Manager".into()),
            ("cmd:load_tester".into(), "Open Load Tester".into()),
        ];

        snap
    }

    /// Applies an `AuraAgentAction` to the active tab/workspace state.
    /// Returns `Ok(Some(cmd_id))` if a workspace command should be executed by the view,
    /// or `Ok(None)` if state was mutated directly.
    pub fn apply_agent_action(&mut self, action: &AuraAgentAction) -> Result<Option<String>, String> {
        match action {
            AuraAgentAction::UpdateBody {
                content,
                content_type,
            } => {
                let tab = self
                    .tabs
                    .get_mut(self.active_tab_index)
                    .ok_or_else(|| "No active request tab".to_string())?;
                tab.request.body = make_body_type(content.clone(), content_type.as_deref());
                tab.spec_buffer = Some(aura_parser::request_to_spec(&tab.request));
                tab.spec_error = None;
                tab.is_dirty = true;
                Ok(None)
            }
            AuraAgentAction::UpdatePreScript { script } => {
                let tab = self
                    .tabs
                    .get_mut(self.active_tab_index)
                    .ok_or_else(|| "No active request tab".to_string())?;
                tab.request.scripts.pre_request = script.clone();
                tab.spec_buffer = Some(aura_parser::request_to_spec(&tab.request));
                tab.spec_error = None;
                tab.is_dirty = true;
                Ok(None)
            }
            AuraAgentAction::UpdatePostScript { script } => {
                let tab = self
                    .tabs
                    .get_mut(self.active_tab_index)
                    .ok_or_else(|| "No active request tab".to_string())?;
                tab.request.scripts.post_response = script.clone();
                tab.spec_buffer = Some(aura_parser::request_to_spec(&tab.request));
                tab.spec_error = None;
                tab.is_dirty = true;
                Ok(None)
            }
            AuraAgentAction::UpdateTestScript { script } => {
                let tab = self
                    .tabs
                    .get_mut(self.active_tab_index)
                    .ok_or_else(|| "No active request tab".to_string())?;
                tab.request.scripts.tests = script.clone();
                tab.spec_buffer = Some(aura_parser::request_to_spec(&tab.request));
                tab.spec_error = None;
                tab.is_dirty = true;
                Ok(None)
            }
            AuraAgentAction::UpdateSpec { spec } => {
                let parsed = aura_parser::parse_request(spec)
                    .map_err(|e| format!("Invalid .aura spec from AI: {e}"))?;
                let tab = self
                    .tabs
                    .get_mut(self.active_tab_index)
                    .ok_or_else(|| "No active request tab".to_string())?;
                tab.request = parsed;
                tab.spec_buffer = Some(spec.clone());
                tab.spec_error = None;
                tab.is_dirty = true;
                Ok(None)
            }
            AuraAgentAction::UpdateHeaders { headers } => {
                let tab = self
                    .tabs
                    .get_mut(self.active_tab_index)
                    .ok_or_else(|| "No active request tab".to_string())?;
                tab.request.headers = headers
                    .iter()
                    .map(|h| HeaderItem::new(h.key.clone(), h.value.clone()))
                    .collect();
                tab.spec_buffer = Some(aura_parser::request_to_spec(&tab.request));
                tab.spec_error = None;
                tab.is_dirty = true;
                Ok(None)
            }
            AuraAgentAction::UpdateParams { params } => {
                let tab = self
                    .tabs
                    .get_mut(self.active_tab_index)
                    .ok_or_else(|| "No active request tab".to_string())?;
                tab.request.params = params
                    .iter()
                    .map(|p| ParamItem::new(p.key.clone(), p.value.clone()))
                    .collect();
                tab.spec_buffer = Some(aura_parser::request_to_spec(&tab.request));
                tab.spec_error = None;
                tab.is_dirty = true;
                Ok(None)
            }
            AuraAgentAction::CreateRequest {
                name,
                method,
                url,
                body,
                pre_script,
                test_script,
            } => {
                let http_method = parse_http_method(method);
                let mut req = AuraRequest::new(name.clone(), http_method, url.clone());
                if let Some(b) = body {
                    req.body = BodyType::Json(b.clone());
                }
                if let Some(pre) = pre_script {
                    req.scripts.pre_request = pre.clone();
                }
                if let Some(tests) = test_script {
                    req.scripts.tests = tests.clone();
                }
                self.open_request(req);
                Ok(None)
            }
            AuraAgentAction::RunCommand { command_id, .. } => Ok(Some(command_id.clone())),
        }
    }
}

fn body_to_option_string(body: &BodyType) -> Option<String> {
    match body {
        BodyType::None => None,
        BodyType::Json(s) | BodyType::Xml(s) | BodyType::Text(s) => Some(s.clone()),
        BodyType::Raw { content, .. } => Some(content.clone()),
        BodyType::GraphQL { query, variables } => Some(format!("{query}\n\n# Variables:\n{variables}")),
        _ => Some(format!("{body:?}")),
    }
}

fn make_body_type(content: String, content_type: Option<&str>) -> BodyType {
    match content_type.map(|s| s.to_ascii_lowercase()) {
        Some(ct) if ct.contains("xml") => BodyType::Xml(content),
        Some(ct) if ct.contains("text") => BodyType::Text(content),
        Some(ct) if ct.contains("graphql") => BodyType::GraphQL {
            query: content,
            variables: String::new(),
        },
        _ => BodyType::Json(content),
    }
}

fn parse_http_method(s: &str) -> HttpMethod {
    match s.trim().to_ascii_uppercase().as_str() {
        "POST" => HttpMethod::POST,
        "PUT" => HttpMethod::PUT,
        "PATCH" => HttpMethod::PATCH,
        "DELETE" => HttpMethod::DELETE,
        "HEAD" => HttpMethod::HEAD,
        "OPTIONS" => HttpMethod::OPTIONS,
        _ => HttpMethod::GET,
    }
}
