use super::AuraAppState;

impl AuraAppState {
    pub fn append_script_snippet(&mut self, is_pre_request: bool, snippet: &str) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let target = if is_pre_request {
                &mut tab.request.scripts.pre_request
            } else {
                &mut tab.request.scripts.post_response
            };
            if !target.is_empty() && !target.ends_with('\n') {
                target.push('\n');
            }
            target.push_str(snippet);
            tab.mark_dirty(true);
        }
    }

    pub fn append_test_snippet(&mut self, snippet: &str) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if !tab.request.scripts.tests.is_empty() && !tab.request.scripts.tests.ends_with('\n') {
                tab.request.scripts.tests.push('\n');
            }
            tab.request.scripts.tests.push_str(snippet);
            tab.mark_dirty(true);
        }
    }

    pub fn run_active_tests(&mut self) -> Result<usize, String> {
        let result = if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if tab.request.scripts.tests.trim().is_empty() {
                return Err("No test script defined".to_string());
            }
            if let Some(ref mut resp) = tab.response {
                let working_vars = tab.request.variables.clone();
                let mut script_rt = aura_scripting::AuraScriptRuntime::new(working_vars)
                    .map_err(|e| format!("Script runtime error: {e}"))?;
                script_rt
                    .inject_response_context_with_duration(
                        resp.status,
                        &resp.body,
                        &resp.headers,
                        resp.duration_ms,
                    )
                    .map_err(|e| format!("Response injection error: {e}"))?;
                let test_res = script_rt
                    .run(&tab.request.scripts.tests)
                    .map_err(|e| format!("Test execution error: {e}"))?;
                let count = test_res.test_assertions.len();
                resp.assertions = test_res.test_assertions;
                resp.console_logs.extend(test_res.console_logs);
                Ok(count)
            } else {
                Err("No response available. Send the request first.".to_string())
            }
        } else {
            Err("No active tab".to_string())
        };
        if result.is_ok() {
            self.active_response_tab = crate::stores::ResponseTab::Tests;
        }
        result
    }

    // =========================================================================
    // Request Settings & Variables Operations
    // =========================================================================

    pub fn clear_setting_drafts(&mut self) {
        self.setting_timeout_draft = None;
        self.setting_timeout_error = None;
        self.setting_max_redirects_draft = None;
        self.setting_max_redirects_error = None;
    }

    pub fn update_setting_timeout(&mut self, text: &str) -> bool {
        self.setting_timeout_draft = Some(text.to_string());
        let trimmed = text.trim();
        if trimmed.is_empty() {
            self.setting_timeout_error = Some("Timeout is required (100 - 300000 ms)".to_string());
            return false;
        }
        match trimmed.parse::<u64>() {
            Ok(v) if (100..=300_000).contains(&v) => {
                self.setting_timeout_error = None;
                if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
                    if tab.request.settings.timeout_ms != v {
                        tab.request.settings.timeout_ms = v;
                        tab.mark_dirty(true);
                    }
                }
                true
            }
            Ok(_) => {
                self.setting_timeout_error = Some("Timeout must be between 100 and 300000 ms".to_string());
                false
            }
            Err(_) => {
                self.setting_timeout_error = Some("Timeout must be a valid integer (100 - 300000)".to_string());
                false
            }
        }
    }

    pub fn update_setting_max_redirects(&mut self, text: &str) -> bool {
        self.setting_max_redirects_draft = Some(text.to_string());
        let trimmed = text.trim();
        if trimmed.is_empty() {
            self.setting_max_redirects_error = Some("Max redirects is required (0 - 50)".to_string());
            return false;
        }
        match trimmed.parse::<usize>() {
            Ok(v) if v <= 50 => {
                self.setting_max_redirects_error = None;
                if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
                    if tab.request.settings.max_redirects != v {
                        tab.request.settings.max_redirects = v;
                        tab.mark_dirty(true);
                    }
                }
                true
            }
            Ok(_) => {
                self.setting_max_redirects_error = Some("Max redirects must be between 0 and 50".to_string());
                false
            }
            Err(_) => {
                self.setting_max_redirects_error = Some("Max redirects must be a valid integer (0 - 50)".to_string());
                false
            }
        }
    }

    pub fn toggle_setting_follow_redirects(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.settings.follow_redirects = !tab.request.settings.follow_redirects;
            tab.mark_dirty(true);
        }
    }

    pub fn toggle_setting_verify_ssl(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.settings.verify_ssl = !tab.request.settings.verify_ssl;
            tab.mark_dirty(true);
        }
    }

    pub fn toggle_setting_inherit_auth(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.settings.inherit_auth = !tab.request.settings.inherit_auth;
            tab.mark_dirty(true);
        }
    }

    pub fn add_variable_row(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.variable_rows.push((String::new(), String::new()));
            tab.sync_variables_from_rows();
            tab.mark_dirty(true);
        }
    }

    pub fn remove_variable_row(&mut self, index: usize) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if index < tab.variable_rows.len() {
                tab.variable_rows.remove(index);
                tab.sync_variables_from_rows();
                tab.mark_dirty(true);
            }
        }
    }

    // =========================================================================
    // Request Header Operations
    // =========================================================================

    pub fn add_header(&mut self, key: impl Into<String>, value: impl Into<String>) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.add_header(key, value);
            tab.mark_dirty(true);
        }
    }

    pub fn remove_header(&mut self, index: usize) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.remove_header(index);
            tab.mark_dirty(true);
        }
    }

    pub fn toggle_header(&mut self, index: usize) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.toggle_header(index);
            tab.mark_dirty(true);
        }
    }

    pub fn update_header(&mut self, index: usize, key: String, value: String) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if let Some(h) = tab.request.headers.get_mut(index) {
                h.key = key;
                h.value = value;
                tab.mark_dirty(true);
            }
        }
    }

    // =========================================================================
    // Query Parameter Operations
    // =========================================================================

    pub fn add_param(&mut self, key: impl Into<String>, value: impl Into<String>) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.add_param(key, value);
            Self::sync_params_to_url(&mut tab.request);
            tab.mark_dirty(true);
        }
    }

    pub fn remove_param(&mut self, index: usize) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.remove_param(index);
            Self::sync_params_to_url(&mut tab.request);
            tab.mark_dirty(true);
        }
    }

    pub fn toggle_param(&mut self, index: usize) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.toggle_param(index);
            Self::sync_params_to_url(&mut tab.request);
            tab.mark_dirty(true);
        }
    }

    pub fn update_param(&mut self, index: usize, key: String, value: String) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if let Some(p) = tab.request.params.get_mut(index) {
                p.key = key;
                p.value = value;
                Self::sync_params_to_url(&mut tab.request);
                tab.mark_dirty(true);
            }
        }
    }

    pub fn sync_url_to_params(req: &mut aura_core::model::AuraRequest) {
        let url = &req.url;
        if let Some(pos) = url.find('?') {
            let query = &url[pos + 1..];
            let mut new_params = Vec::new();
            if !query.is_empty() {
                for pair in query.split('&') {
                    if let Some(eq_pos) = pair.find('=') {
                        let k = &pair[..eq_pos];
                        let v = &pair[eq_pos + 1..];
                        new_params.push(aura_core::model::ParamItem::new(k.to_string(), v.to_string()));
                    } else if !pair.is_empty() {
                        new_params.push(aura_core::model::ParamItem::new(pair.to_string(), String::new()));
                    }
                }
            }
            req.params = new_params;
        } else {
            req.params.clear();
        }
    }

    pub fn sync_params_to_url(req: &mut aura_core::model::AuraRequest) {
        let base_url = match req.url.find('?') {
            Some(pos) => req.url[..pos].to_string(),
            None => req.url.clone(),
        };

        let enabled_params: Vec<_> = req.params.iter().filter(|p| p.enabled && !p.key.is_empty()).collect();
        if enabled_params.is_empty() {
            req.url = base_url;
        } else {
            let mut query = String::new();
            for (idx, p) in enabled_params.iter().enumerate() {
                if idx > 0 {
                    query.push('&');
                }
                query.push_str(&p.key);
                query.push('=');
                query.push_str(&p.value);
            }
            req.url = format!("{base_url}?{query}");
        }
    }

    // =========================================================================
    // Request Body Operations
    // =========================================================================

    pub fn set_body(&mut self, body: aura_core::model::BodyType) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.body = body;
            tab.mark_dirty(true);
        }
    }

    pub fn set_body_type(&mut self, type_name: &str) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.body = aura_core::model::BodyType::default_for_type(type_name);
            tab.mark_dirty(true);
        }
    }

    pub fn set_body_content(&mut self, content: String) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            match &mut tab.request.body {
                aura_core::model::BodyType::Json(s)
                | aura_core::model::BodyType::Xml(s)
                | aura_core::model::BodyType::Text(s) => {
                    *s = content;
                }
                aura_core::model::BodyType::Raw { content: c, .. } => {
                    *c = content;
                }
                _ => {
                    tab.request.body = aura_core::model::BodyType::Json(content);
                }
            }
            tab.mark_dirty(true);
        }
    }

    pub fn format_active_json_body(&mut self) -> Result<(), String> {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let formatted = match &tab.request.body {
                aura_core::model::BodyType::Json(s) => {
                    crate::services::FormatService::format_json(s)
                }
                aura_core::model::BodyType::Raw { content, .. } => {
                    crate::services::FormatService::format_json(content)
                }
                _ => return Err("Body is not JSON format".to_string()),
            };
            match formatted {
                Ok(pretty) => {
                    match &mut tab.request.body {
                        aura_core::model::BodyType::Json(s) => *s = pretty,
                        aura_core::model::BodyType::Raw { content, .. } => *content = pretty,
                        _ => {}
                    }
                    tab.mark_dirty(true);
                    Ok(())
                }
                Err(e) => Err(format!("Invalid JSON: {e}")),
            }
        } else {
            Err("No active request tab".to_string())
        }
    }

    pub fn add_form_urlencoded_param(&mut self, key: impl Into<String>, value: impl Into<String>) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            match &mut tab.request.body {
                aura_core::model::BodyType::FormUrlEncoded(items) => {
                    items.push(aura_core::model::ParamItem::new(key, value));
                }
                _ => {
                    tab.request.body = aura_core::model::BodyType::FormUrlEncoded(vec![
                        aura_core::model::ParamItem::new(key, value),
                    ]);
                }
            }
            tab.mark_dirty(true);
        }
    }

    pub fn remove_form_urlencoded_param(&mut self, index: usize) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if let aura_core::model::BodyType::FormUrlEncoded(items) = &mut tab.request.body {
                if index < items.len() {
                    items.remove(index);
                    tab.mark_dirty(true);
                }
            }
        }
    }

    pub fn toggle_form_urlencoded_param(&mut self, index: usize) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if let aura_core::model::BodyType::FormUrlEncoded(items) = &mut tab.request.body {
                if let Some(item) = items.get_mut(index) {
                    item.enabled = !item.enabled;
                    tab.mark_dirty(true);
                }
            }
        }
    }

    pub fn add_multipart_field(
        &mut self,
        name: String,
        value: Option<String>,
        file_path: Option<String>,
        content_type: Option<String>,
    ) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let part = aura_core::model::MultipartPart {
                name,
                value,
                file_path,
                content_type,
            };
            match &mut tab.request.body {
                aura_core::model::BodyType::Multipart(parts) => {
                    parts.push(part);
                }
                _ => {
                    tab.request.body = aura_core::model::BodyType::Multipart(vec![part]);
                }
            }
            tab.mark_dirty(true);
        }
    }

    pub fn remove_multipart_field(&mut self, index: usize) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if let aura_core::model::BodyType::Multipart(parts) = &mut tab.request.body {
                if index < parts.len() {
                    parts.remove(index);
                    tab.mark_dirty(true);
                }
            }
        }
    }

    // =========================================================================
    // Authentication Operations
    // =========================================================================

    pub fn set_auth(&mut self, auth: aura_core::model::AuthConfig) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.auth = auth;
            tab.mark_dirty(true);
        }
    }

    pub fn set_auth_type(&mut self, type_name: &str) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.auth = aura_core::model::AuthConfig::default_for_type(type_name);
            tab.mark_dirty(true);
        }
    }

    pub fn update_bearer_token(&mut self, token: String) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.auth = aura_core::model::AuthConfig::Bearer { token };
            tab.mark_dirty(true);
        }
    }

    pub fn update_basic_auth(&mut self, username: String, password: String) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.auth = aura_core::model::AuthConfig::Basic { username, password };
            tab.mark_dirty(true);
        }
    }

    pub fn update_api_key(&mut self, key: String, value: String, in_header: bool) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.auth = aura_core::model::AuthConfig::ApiKey {
                key,
                value,
                in_header,
            };
            tab.mark_dirty(true);
        }
    }

    pub fn update_aws_sigv4(
        &mut self,
        access_key: String,
        secret_key: String,
        region: String,
        service: String,
        session_token: Option<String>,
    ) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.auth = aura_core::model::AuthConfig::AwsSigV4 {
                access_key,
                secret_key,
                region,
                service,
                session_token,
            };
            tab.mark_dirty(true);
        }
    }

    pub fn update_oauth2(
        &mut self,
        grant_type: String,
        access_token: String,
        token_url: Option<String>,
        client_id: Option<String>,
        client_secret: Option<String>,
    ) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.auth = aura_core::model::AuthConfig::OAuth2 {
                grant_type,
                access_token,
                token_url,
                client_id,
                client_secret,
            };
            tab.mark_dirty(true);
        }
    }

    // =========================================================================
    // Editor Dialog Operations
    // =========================================================================

}
