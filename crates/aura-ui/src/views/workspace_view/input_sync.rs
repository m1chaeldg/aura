use super::AuraWorkspaceView;
use crate::app_state::EditorDialogState;
use crate::input_registry::InputKey;
use crate::views::{
    SidebarView,
};
use aura_core::model::{AuthConfig, BodyType};
use gpui::*;
use std::rc::Rc;

impl AuraWorkspaceView {
    pub(super) fn sync_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut inputs = std::mem::take(&mut self.inputs);

        let url_val = self
            .state
            .tabs
            .get(self.state.active_tab_index)
            .map(|t| t.request.url.clone())
            .unwrap_or_default();
        inputs.ensure_input(
            InputKey::Url,
            &url_val,
            "Enter request URL (e.g. https://postman-echo.com/get)...",
            false,
            window,
            cx,
            Self::handle_input_event,
        );

        let palette_q = self.state.command_palette_search.clone();
        inputs.ensure_input(
            InputKey::CommandPaletteSearch,
            &palette_q,
            "Search requests or type command... (Esc to dismiss)",
            false,
            window,
            cx,
            Self::handle_input_event,
        );

        if let Some(ref rd) = self.state.rename_dialog {
            let v = rd.current_name.clone();
            inputs.ensure_input(
                InputKey::RenameDialog,
                &v,
                "Enter name...",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
        }

        if let Some(ref md) = self.state.move_to_dialog {
            let v = md.search_query.clone();
            inputs.ensure_input(
                InputKey::MoveToSearch,
                &v,
                "Type to filter destination folders...",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
        }

        if let Some(ref ir) = self.state.inline_rename {
            let v = ir.current_value.clone();
            inputs.ensure_input(
                InputKey::InlineRename,
                &v,
                "Enter name...",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
        }

        if let Some(ref dlg) = self.state.editor_dialog {
            let fields: Vec<(usize, String, &'static str)> = match dlg {
                EditorDialogState::AddHeader { key, value, .. }
                | EditorDialogState::EditHeader { key, value, .. } => vec![
                    (0, key.clone(), "Header Name"),
                    (1, value.clone(), "Header Value"),
                ],
                EditorDialogState::AddParam { key, value, .. }
                | EditorDialogState::EditParam { key, value, .. } => {
                    vec![(0, key.clone(), "Key"), (1, value.clone(), "Value")]
                }
                EditorDialogState::EditBody { content } => {
                    vec![(0, content.clone(), "{\n  \"key\": \"value\"\n}")]
                }
                EditorDialogState::EditBearerAuth { token } => {
                    vec![(0, token.clone(), "eyJhbGciOi...")]
                }
                EditorDialogState::EditBasicAuth {
                    username, password, ..
                } => vec![
                    (0, username.clone(), "Username"),
                    (1, password.clone(), "Password"),
                ],
                EditorDialogState::EditApiKeyAuth { key, value, .. } => vec![
                    (0, key.clone(), "e.g. X-API-Key"),
                    (1, value.clone(), "secret_key_123"),
                ],
                EditorDialogState::EditAwsSigV4 {
                    access_key,
                    secret_key,
                    region,
                    service,
                    session_token,
                    ..
                } => vec![
                    (0, access_key.clone(), "AKIA..."),
                    (1, secret_key.clone(), "wJalrXUtnFEMI..."),
                    (2, region.clone(), "us-east-1"),
                    (3, service.clone(), "execute-api"),
                    (4, session_token.clone(), "AQoDYXdz..."),
                ],
                EditorDialogState::EditOAuth2 {
                    access_token,
                    grant_type,
                    token_url,
                    client_id,
                    client_secret,
                    ..
                } => vec![
                    (0, access_token.clone(), "ya29..."),
                    (1, grant_type.clone(), "authorization_code"),
                    (2, token_url.clone(), "https://auth.example.com/token"),
                    (3, client_id.clone(), "client_123"),
                    (4, client_secret.clone(), "secret_456"),
                ],
                EditorDialogState::AddFormParam { key, value, .. } => {
                    vec![(0, key.clone(), "key"), (1, value.clone(), "value")]
                }
                EditorDialogState::AddMultipartField {
                    name,
                    value,
                    file_path,
                    ..
                } => vec![
                    (0, name.clone(), "name"),
                    (1, value.clone(), "value"),
                    (2, file_path.clone(), "path/to/file.bin"),
                ],
            };
            for (idx, val, ph) in fields {
                inputs.ensure_input(
                    InputKey::EditorDialogField(idx),
                    &val,
                    ph,
                    false,
                    window,
                    cx,
                    Self::handle_input_event,
                );
            }
        }

        if let Some(tab) = self.state.tabs.get(self.state.active_tab_index).cloned() {
            inputs.set_editor_context(crate::editor_support::EditorContextData {
                variables: tab.variable_rows.clone(),
                query_params: tab
                    .request
                    .params
                    .iter()
                    .map(|p| (p.key.clone(), p.value.clone(), p.enabled))
                    .collect(),
                headers: tab
                    .request
                    .headers
                    .iter()
                    .map(|h| (h.key.clone(), h.value.clone(), h.enabled))
                    .collect(),
            });

            for (idx, p) in tab.request.params.iter().enumerate() {
                inputs.ensure_input(
                    InputKey::ParamKey(idx),
                    &p.key,
                    "Key",
                    false,
                    window,
                    cx,
                    Self::handle_input_event,
                );
                inputs.ensure_input(
                    InputKey::ParamValue(idx),
                    &p.value,
                    "Value",
                    false,
                    window,
                    cx,
                    Self::handle_input_event,
                );
            }

            for (idx, h) in tab.request.headers.iter().enumerate() {
                inputs.ensure_input(
                    InputKey::HeaderKey(idx),
                    &h.key,
                    "Header Name",
                    false,
                    window,
                    cx,
                    Self::handle_input_event,
                );
                inputs.ensure_input(
                    InputKey::HeaderValue(idx),
                    &h.value,
                    "Header Value",
                    false,
                    window,
                    cx,
                    Self::handle_input_event,
                );
            }

            for (idx, (k, v, _)) in tab.variable_rows.iter().enumerate() {
                inputs.ensure_input(
                    InputKey::VariableKey(idx),
                    k,
                    "Variable Name",
                    false,
                    window,
                    cx,
                    Self::handle_input_event,
                );
                inputs.ensure_input(
                    InputKey::VariableValue(idx),
                    v,
                    "Variable Value",
                    false,
                    window,
                    cx,
                    Self::handle_input_event,
                );
            }

            let timeout_str = self
                .state
                .setting_timeout_draft
                .clone()
                .unwrap_or_else(|| tab.request.settings.timeout_ms.to_string());
            inputs.ensure_input(
                InputKey::SettingTimeout,
                &timeout_str,
                "30000",
                false,
                window,
                cx,
                Self::handle_input_event,
            );

            let max_redirects_str = self
                .state
                .setting_max_redirects_draft
                .clone()
                .unwrap_or_else(|| tab.request.settings.max_redirects.to_string());
            inputs.ensure_input(
                InputKey::SettingMaxRedirects,
                &max_redirects_str,
                "10",
                false,
                window,
                cx,
                Self::handle_input_event,
            );

            match &tab.request.body {
                BodyType::Json(s) | BodyType::Xml(s) | BodyType::Text(s) => {
                    inputs.ensure_textarea(
                        InputKey::BodyText,
                        s,
                        "// Click or type to edit request body...",
                        window,
                        cx,
                        Self::handle_textarea_event,
                    );
                }
                BodyType::Raw { content, .. } => {
                    inputs.ensure_textarea(
                        InputKey::BodyText,
                        content,
                        "// Click or type to edit request body...",
                        window,
                        cx,
                        Self::handle_textarea_event,
                    );
                }
                BodyType::FormUrlEncoded(items) => {
                    for (idx, item) in items.iter().enumerate() {
                        inputs.ensure_input(
                            InputKey::FormParamKey(idx),
                            &item.key,
                            "Field Key",
                            false,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                        inputs.ensure_input(
                            InputKey::FormParamValue(idx),
                            &item.value,
                            "Field Value",
                            false,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                    }
                }
                BodyType::Form(map) => {
                    for (idx, (k, v)) in map.iter().enumerate() {
                        inputs.ensure_input(
                            InputKey::FormParamKey(idx),
                            k,
                            "Field Key",
                            false,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                        inputs.ensure_input(
                            InputKey::FormParamValue(idx),
                            v,
                            "Field Value",
                            false,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                    }
                }
                _ => {}
            }

            match &tab.request.auth {
                AuthConfig::Bearer { token } => {
                    inputs.ensure_input(
                        InputKey::AuthBearerToken,
                        token,
                        "Enter Bearer token or paste JWT here...",
                        false,
                        window,
                        cx,
                        Self::handle_input_event,
                    );
                }
                AuthConfig::Basic { username, password } => {
                    inputs.ensure_input(
                        InputKey::AuthBasicUsername,
                        username,
                        "Enter username...",
                        false,
                        window,
                        cx,
                        Self::handle_input_event,
                    );
                    inputs.ensure_input(
                        InputKey::AuthBasicPassword,
                        password,
                        "Enter password...",
                        true,
                        window,
                        cx,
                        Self::handle_input_event,
                    );
                }
                AuthConfig::ApiKey { key, value, .. } => {
                    inputs.ensure_input(
                        InputKey::AuthApiKeyName,
                        key,
                        "e.g. X-API-Key",
                        false,
                        window,
                        cx,
                        Self::handle_input_event,
                    );
                    inputs.ensure_input(
                        InputKey::AuthApiKeyValue,
                        value,
                        "secret_key_123...",
                        false,
                        window,
                        cx,
                        Self::handle_input_event,
                    );
                }
                AuthConfig::AwsSigV4 {
                    use_profile: _,
                    profile,
                    credentials_path,
                    access_key,
                    secret_key,
                    region,
                    service,
                    session_token,
                } => {
                    let fields = [
                        (0, access_key.as_str(), "Enter Access Key..."),
                        (1, secret_key.as_str(), "Enter Secret Key..."),
                        (2, region.as_str(), "us-east-1"),
                        (3, service.as_str(), "execute-api"),
                        (4, session_token.as_deref().unwrap_or(""), "Enter Session Token..."),
                        (5, profile.as_str(), "default"),
                        (6, credentials_path.as_deref().unwrap_or(""), "~/.aws/credentials"),
                    ];
                    for (idx, val, ph) in fields {
                        inputs.ensure_input(
                            InputKey::AuthAwsField(idx),
                            val,
                            ph,
                            false,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                    }
                }
                AuthConfig::OAuth2 {
                    grant_type,
                    access_token,
                    token_url,
                    client_id,
                    client_secret,
                } => {
                    let fields = [
                        (0, access_token.as_str(), "Enter Access Token..."),
                        (1, grant_type.as_str(), "Enter Grant Type..."),
                        (2, token_url.as_deref().unwrap_or(""), "Enter Token URL..."),
                        (3, client_id.as_deref().unwrap_or(""), "Enter Client ID..."),
                        (4, client_secret.as_deref().unwrap_or(""), "Enter Client Secret..."),
                    ];
                    for (idx, val, ph) in fields {
                        inputs.ensure_input(
                            InputKey::AuthOAuth2Field(idx),
                            val,
                            ph,
                            false,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                    }
                }
                AuthConfig::None | AuthConfig::Inherit => {}
            }

            inputs.ensure_editor(
                InputKey::PreRequestScript,
                &tab.request.scripts.pre_request,
                "// Write pre-request script here...",
                InputKey::PreRequestScript.editor_language().unwrap_or("javascript"),
                window,
                cx,
                Self::handle_editor_event,
            );
            inputs.ensure_editor(
                InputKey::PostResponseScript,
                &tab.request.scripts.post_response,
                "// Write post-response script here...",
                InputKey::PostResponseScript.editor_language().unwrap_or("javascript"),
                window,
                cx,
                Self::handle_editor_event,
            );
            inputs.ensure_editor(
                InputKey::TestScript,
                &tab.request.scripts.tests,
                "// Write Jasmine/Chai assertions...",
                InputKey::TestScript.editor_language().unwrap_or("javascript"),
                window,
                cx,
                Self::handle_editor_event,
            );

            let spec_val = tab
                .spec_buffer
                .clone()
                .unwrap_or_else(|| aura_parser::request_to_spec(&tab.request));
            inputs.ensure_editor(
                InputKey::SpecText,
                &spec_val,
                "# Empty .aura specification...",
                InputKey::SpecText.editor_language().unwrap_or("toml"),
                window,
                cx,
                Self::handle_editor_event,
            );

            if let Some(ref resp) = tab.response {
                let content_type = resp
                    .headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                    .map(|(_, v)| v.as_str());
                let detected_lang =
                    crate::services::FormatService::detect_language(&resp.body, content_type);
                let effective_lang = self.state.response_language_override.unwrap_or(detected_lang);
                let body_formatted = crate::services::FormatService::format_response_body(
                    &resp.body,
                    effective_lang,
                    self.state.active_body_mode,
                );
                let body_display = if body_formatted.is_empty() {
                    "// Response body is empty".to_string()
                } else {
                    body_formatted
                };
                inputs.ensure_readonly_editor(
                    InputKey::ResponseBody,
                    &body_display,
                    "",
                    effective_lang.tree_sitter_language(),
                    self.state.response_soft_wrap,
                    window,
                    cx,
                );

                let mut raw_http = format!("HTTP/1.1 {} {}\n", resp.status, resp.status_text);
                let mut header_entries: Vec<_> = resp.headers.iter().collect();
                header_entries.sort_by_key(|(k, _)| (*k).clone());
                for (k, v) in header_entries {
                    raw_http.push_str(&format!("{}: {}\n", k, v));
                }
                raw_http.push('\n');
                raw_http.push_str(&resp.body);
                inputs.ensure_readonly_editor(
                    InputKey::ResponseRaw,
                    &raw_http,
                    "",
                    "text",
                    self.state.response_soft_wrap,
                    window,
                    cx,
                );

                let tests_text = crate::views::ResponsePaneView::format_tests_text(resp);
                inputs.ensure_readonly_editor(
                    InputKey::ResponseTests,
                    &tests_text,
                    "",
                    "text",
                    self.state.response_soft_wrap,
                    window,
                    cx,
                );

                let console_text = crate::views::ResponsePaneView::format_console_text(resp);
                inputs.ensure_readonly_editor(
                    InputKey::ResponseConsole,
                    &console_text,
                    "",
                    "text",
                    self.state.response_soft_wrap,
                    window,
                    cx,
                );

                inputs.ensure_input(
                    InputKey::ResponseTableSearch,
                    &self.state.response_table_search,
                    "Filter rows...",
                    false,
                    window,
                    cx,
                    Self::handle_input_event,
                );
            }
        }

        if self.state.is_log_viewer_open && self.state.is_log_viewer_text_mode {
            let log_text = self.state.format_filtered_logs_text();
            inputs.ensure_textarea(
                InputKey::LogViewerText,
                &log_text,
                "No log entries matching criteria.",
                window,
                cx,
                Self::handle_textarea_event,
            );
        }

        if let Some(ref env_ed) = self.state.environment_editor {
            match env_ed.mode {
                crate::app_state::EnvironmentEditorMode::Ui => {
                    inputs.ensure_input(
                        InputKey::EnvName,
                        &env_ed.profile.name,
                        "Environment name (e.g. Production)...",
                        false,
                        window,
                        cx,
                        Self::handle_input_event,
                    );
                    for (idx, var) in env_ed.profile.variables.iter().enumerate() {
                        inputs.ensure_input(
                            InputKey::EnvVarKey(idx),
                            &var.key,
                            "VARIABLE_KEY",
                            false,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                        inputs.ensure_input(
                            InputKey::EnvVarValue(idx),
                            &var.value,
                            "value",
                            false,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                    }
                }
                crate::app_state::EnvironmentEditorMode::Code => {
                    inputs.ensure_editor(
                        InputKey::EnvCode,
                        &env_ed.code_buffer,
                        "name = \"Production\"\n\n[variables]\nBASE_URL = \"https://api.example.com\"\n",
                        InputKey::EnvCode.editor_language().unwrap_or("toml"),
                        window,
                        cx,
                        Self::handle_editor_event,
                    );
                }
            }
        }

        self.inputs = inputs;
        self.sync_sidebar_tree(cx);
    }

    pub(super) fn sync_sidebar_tree(&mut self, cx: &mut Context<Self>) {
        let mut sig = String::new();
        for node in &self.state.workspace_store.tree {
            append_tree_sig(node, &mut sig);
        }
        if sig.is_empty() {
            for col in &self.state.collections {
                sig.push_str(&col.id);
                sig.push(':');
                for req in &col.requests {
                    sig.push_str(&req.id);
                    sig.push(',');
                }
            }
        }
        if sig != self.sidebar_tree_sig {
            self.sidebar_tree_sig = sig;
            let (items, meta) = SidebarView::build_tree_items(&self.state);
            self.sidebar_tree_meta = Rc::new(meta);
            self.sidebar_tree_state.update(cx, |ts, cx| {
                ts.set_items(items, cx);
            });
        }
    }
}

fn append_tree_sig(node: &crate::stores::workspace_store::WorkspaceTreeNode, out: &mut String) {
    match node {
        crate::stores::workspace_store::WorkspaceTreeNode::Folder {
            path,
            is_expanded,
            children,
            ..
        } => {
            out.push_str("D:");
            out.push_str(&path.to_string_lossy());
            out.push(if *is_expanded { '1' } else { '0' });
            out.push(';');
            for c in children {
                append_tree_sig(c, out);
            }
        }
        crate::stores::workspace_store::WorkspaceTreeNode::Request { path, method, .. } => {
            out.push_str("F:");
            out.push_str(&path.to_string_lossy());
            out.push(':');
            out.push_str(method.as_str());
            out.push(';');
        }
    }
}

