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
        inputs.set_template_context(self.state.active_template_context());

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

        if self.state.is_ai_drawer_open {
            let draft = self.state.ai_prompt_draft.clone();
            inputs.ensure_input(
                InputKey::AiPromptInput,
                &draft,
                "Ask Aura AI to edit body, scripts, spec, or run commands...",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
        }

        if self.state.is_settings_modal_open && self.state.settings_tab_ai {
            let cli_name = self.state.ai_custom_cli_name.clone();
            let cli_cmd = self.state.ai_custom_cli_cmd.clone();
            let cli_args = self.state.ai_custom_cli_args.clone();
            let http_name = self.state.ai_http_name.clone();
            let http_url = self.state.ai_http_url.clone();
            let http_model = self.state.ai_http_model.clone();
            let http_key = self.state.ai_http_api_key.clone();
            inputs.ensure_input(
                InputKey::AiCustomCliName,
                &cli_name,
                "Display name (e.g. My Agent)",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::AiCustomCliCmd,
                &cli_cmd,
                "Command or path (e.g. claude)",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::AiCustomCliArgs,
                &cli_args,
                "Args template (e.g. -p {prompt})",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::AiHttpName,
                &http_name,
                "Endpoint name (e.g. OpenRouter)",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::AiHttpUrl,
                &http_url,
                "Base URL (e.g. https://api.openai.com/v1)",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::AiHttpModel,
                &http_model,
                "Model (e.g. gpt-4o)",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::AiHttpApiKey,
                &http_key,
                "API key or $ENV_VAR",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
        }

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
                graphql_schema: self.state.graphql_schemas.get(&tab.id).cloned(),
            });

            for (idx, a) in tab.request.scripts.assertions.iter().enumerate() {
                inputs.ensure_input(
                    InputKey::AssertExpr(idx),
                    &a.expression,
                    "res.status / res.body.id / res.duration_ms",
                    false,
                    window,
                    cx,
                    Self::handle_input_event,
                );
                inputs.ensure_input(
                    InputKey::AssertExpected(idx),
                    &a.expected,
                    "200 / \"value\"",
                    false,
                    window,
                    cx,
                    Self::handle_input_event,
                );
            }

            for (idx, ext) in tab.request.extractors.iter().enumerate() {
                inputs.ensure_input(
                    InputKey::ExtractorVar(idx),
                    &ext.variable,
                    "var_name",
                    false,
                    window,
                    cx,
                    Self::handle_input_event,
                );
                inputs.ensure_input(
                    InputKey::ExtractorExpr(idx),
                    &ext.expression,
                    "$.data.token / regex / header",
                    false,
                    window,
                    cx,
                    Self::handle_input_event,
                );
            }

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
                BodyType::Json(s) => {
                    inputs.ensure_editor(
                        InputKey::BodyText,
                        s,
                        "// Click or type to edit JSON body...",
                        "json",
                        window,
                        cx,
                        Self::handle_editor_event,
                    );
                }
                BodyType::Xml(s) => {
                    inputs.ensure_editor(
                        InputKey::BodyText,
                        s,
                        "<!-- Click or type to edit XML body... -->",
                        "html",
                        window,
                        cx,
                        Self::handle_editor_event,
                    );
                }
                BodyType::Text(s) => {
                    inputs.ensure_editor(
                        InputKey::BodyText,
                        s,
                        "// Click or type to edit request body...",
                        "plaintext",
                        window,
                        cx,
                        Self::handle_editor_event,
                    );
                }
                BodyType::Raw { content, .. } => {
                    inputs.ensure_editor(
                        InputKey::BodyText,
                        content,
                        "// Click or type to edit raw body...",
                        "plaintext",
                        window,
                        cx,
                        Self::handle_editor_event,
                    );
                }
                BodyType::GraphQL { query, variables } => {
                    inputs.ensure_editor(
                        InputKey::GraphQLQuery,
                        query,
                        "query {\n  \n}",
                        "plaintext",
                        window,
                        cx,
                        Self::handle_editor_event,
                    );
                    inputs.ensure_editor(
                        InputKey::GraphQLVariables,
                        variables,
                        "{}",
                        "json",
                        window,
                        cx,
                        Self::handle_editor_event,
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
                    auth_url,
                    redirect_uri,
                    scope,
                    username,
                    password,
                    ..
                } => {
                    let fields = [
                        (0, access_token.as_str(), "Enter Access Token...", false),
                        (1, grant_type.as_str(), "authorization_code | client_credentials | password | implicit", false),
                        (2, token_url.as_deref().unwrap_or(""), "https://auth.example.com/oauth/token", false),
                        (3, client_id.as_deref().unwrap_or(""), "Enter Client ID...", false),
                        (4, client_secret.as_deref().unwrap_or(""), "Enter Client Secret...", true),
                        (5, auth_url.as_deref().unwrap_or(""), "https://auth.example.com/authorize", false),
                        (6, redirect_uri.as_deref().unwrap_or(""), "http://127.0.0.1:8765/callback", false),
                        (7, scope.as_deref().unwrap_or(""), "read write", false),
                        (8, username.as_deref().unwrap_or(""), "Resource owner username", false),
                        (9, password.as_deref().unwrap_or(""), "Resource owner password", true),
                    ];
                    for (idx, val, ph, masked) in fields {
                        inputs.ensure_input(
                            InputKey::AuthOAuth2Field(idx),
                            val,
                            ph,
                            masked,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                    }
                }
                AuthConfig::Digest {
                    username,
                    password,
                    realm,
                    nonce,
                    qop,
                    algorithm,
                } => {
                    let fields = [
                        (0, username.as_str(), "Digest username", false),
                        (1, password.as_str(), "Digest password", true),
                        (2, realm.as_deref().unwrap_or(""), "Auto from 401 WWW-Authenticate", false),
                        (3, nonce.as_deref().unwrap_or(""), "Auto from 401 WWW-Authenticate", false),
                        (4, qop.as_deref().unwrap_or(""), "auth", false),
                        (5, algorithm.as_deref().unwrap_or(""), "MD5 | MD5-sess | SHA-256", false),
                    ];
                    for (idx, val, ph, masked) in fields {
                        inputs.ensure_input(
                            InputKey::AuthSchemeField(idx),
                            val,
                            ph,
                            masked,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                    }
                }
                AuthConfig::Ntlm {
                    username,
                    password,
                    domain,
                    workstation,
                } => {
                    let fields = [
                        (0, username.as_str(), "NTLM username", false),
                        (1, password.as_str(), "NTLM password", true),
                        (2, domain.as_str(), "DOMAIN (optional)", false),
                        (3, workstation.as_str(), "WORKSTATION", false),
                    ];
                    for (idx, val, ph, masked) in fields {
                        inputs.ensure_input(
                            InputKey::AuthSchemeField(idx),
                            val,
                            ph,
                            masked,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                    }
                }
                AuthConfig::OAuth1 {
                    consumer_key,
                    consumer_secret,
                    token,
                    token_secret,
                    callback_url,
                    realm,
                    ..
                } => {
                    let fields = [
                        (0, consumer_key.as_str(), "Consumer Key", false),
                        (1, consumer_secret.as_str(), "Consumer Secret", true),
                        (2, token.as_str(), "Access Token (optional)", false),
                        (3, token_secret.as_str(), "Token Secret (optional)", true),
                        (4, callback_url.as_deref().unwrap_or(""), "Callback URL (optional)", false),
                        (5, realm.as_deref().unwrap_or(""), "Realm (optional)", false),
                    ];
                    for (idx, val, ph, masked) in fields {
                        inputs.ensure_input(
                            InputKey::AuthSchemeField(idx),
                            val,
                            ph,
                            masked,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                    }
                }
                AuthConfig::Hawk {
                    id,
                    key,
                    ext,
                    app,
                    dlg,
                    ..
                } => {
                    let fields = [
                        (0, id.as_str(), "Hawk Auth ID", false),
                        (1, key.as_str(), "Hawk Auth Key", true),
                        (2, ext.as_deref().unwrap_or(""), "App-specific data (ext)", false),
                        (3, app.as_deref().unwrap_or(""), "Application ID (app)", false),
                        (4, dlg.as_deref().unwrap_or(""), "Delegated By (dlg)", false),
                    ];
                    for (idx, val, ph, masked) in fields {
                        inputs.ensure_input(
                            InputKey::AuthSchemeField(idx),
                            val,
                            ph,
                            masked,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                    }
                }
                AuthConfig::AkamaiEdgeGrid {
                    client_token,
                    client_secret,
                    access_token,
                    max_body,
                } => {
                    let max_b = max_body.to_string();
                    let fields = [
                        (0, client_token.as_str(), "Client Token", false),
                        (1, client_secret.as_str(), "Client Secret", true),
                        (2, access_token.as_str(), "Access Token", false),
                        (3, max_b.as_str(), "131072", false),
                    ];
                    for (idx, val, ph, masked) in fields {
                        inputs.ensure_input(
                            InputKey::AuthSchemeField(idx),
                            val,
                            ph,
                            masked,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                    }
                }
                AuthConfig::Wsse {
                    username,
                    password,
                    ..
                } => {
                    let fields = [
                        (0, username.as_str(), "WSSE Username", false),
                        (1, password.as_str(), "WSSE Password", true),
                    ];
                    for (idx, val, ph, masked) in fields {
                        inputs.ensure_input(
                            InputKey::AuthSchemeField(idx),
                            val,
                            ph,
                            masked,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                    }
                }
                AuthConfig::None | AuthConfig::Inherit => {}
            }

            // Per-request mTLS client certificate fields
            let req_cert = tab.request.settings.client_cert.as_ref();
            let cert_fields = [
                (0, req_cert.map(|c| c.cert_path.as_str()).unwrap_or(""), "Path to .pem, .pfx, or .p12 certificate", false),
                (1, req_cert.and_then(|c| c.key_path.as_deref()).unwrap_or(""), "Path to PEM private key (optional)", false),
                (2, req_cert.and_then(|c| c.passphrase.as_deref()).unwrap_or(""), "PKCS#12 passphrase (optional)", true),
            ];
            for (idx, val, ph, masked) in cert_fields {
                inputs.ensure_input(
                    InputKey::SettingCertField(idx),
                    val,
                    ph,
                    masked,
                    window,
                    cx,
                    Self::handle_input_event,
                );
            }

            // Per-request proxy fields
            let req_proxy = tab.request.settings.proxy.as_ref();
            let req_proxy_port = req_proxy
                .map(|p| p.port.to_string())
                .unwrap_or_else(|| "8080".to_string());
            let req_proxy_bypass = req_proxy
                .map(|p| p.bypass_list.join(", "))
                .unwrap_or_default();
            let proxy_fields = [
                (0, req_proxy.map(|p| p.host.as_str()).unwrap_or(""), "127.0.0.1 or proxy.corp.local", false),
                (1, req_proxy_port.as_str(), "8080", false),
                (2, req_proxy.map(|p| p.username.as_str()).unwrap_or(""), "Proxy username", false),
                (3, req_proxy.map(|p| p.password.as_str()).unwrap_or(""), "Proxy password", true),
                (4, req_proxy_bypass.as_str(), "localhost, 127.0.0.1, *.local", false),
            ];
            for (idx, val, ph, masked) in proxy_fields {
                inputs.ensure_input(
                    InputKey::SettingProxyField(idx),
                    val,
                    ph,
                    masked,
                    window,
                    cx,
                    Self::handle_input_event,
                );
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

            // Protocol-specific controls (WebSocket, gRPC, SOAP)
            inputs.ensure_editor(
                InputKey::WsMessageInput,
                &self.state.ws_composer_text,
                "{\"event\": \"ping\"}",
                "json",
                window,
                cx,
                Self::handle_editor_event,
            );
            inputs.ensure_input(
                InputKey::WsTemplateName,
                &self.state.ws_template_name,
                "Template name...",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            let grpc_svc = tab
                .request
                .grpc
                .as_ref()
                .map(|g| g.service.as_str())
                .unwrap_or("");
            let grpc_mth = tab
                .request
                .grpc
                .as_ref()
                .map(|g| g.method.as_str())
                .unwrap_or("");
            let grpc_payload = tab
                .request
                .grpc
                .as_ref()
                .map(|g| g.payload.as_str())
                .unwrap_or("{}");
            inputs.ensure_input(
                InputKey::GrpcService,
                grpc_svc,
                "e.g. helloworld.Greeter",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::GrpcMethod,
                grpc_mth,
                "e.g. SayHello",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_editor(
                InputKey::GrpcPayload,
                grpc_payload,
                "{\"name\": \"Aura\"}",
                "json",
                window,
                cx,
                Self::handle_editor_event,
            );
            inputs.ensure_input(
                InputKey::SoapOperation,
                &self.state.soap_operation,
                "Operation name",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::SoapNamespace,
                &self.state.soap_namespace,
                "http://tempuri.org/",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::SoapAction,
                &self.state.soap_action,
                "SOAPAction URI (optional)",
                false,
                window,
                cx,
                Self::handle_input_event,
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

        if let Some(ref dlg) = self.state.wsdl_import_dialog {
            inputs.ensure_editor(
                InputKey::WsdlImportText,
                &dlg.wsdl_text,
                "<!-- Paste WSDL XML here or click Choose .wsdl File -->",
                "html",
                window,
                cx,
                Self::handle_editor_event,
            );
        }

        inputs.sync_all_template_highlights(cx);
        if self.state.is_security_vault_modal_open {
            inputs.ensure_input(
                InputKey::KeychainService,
                &self.state.keychain_service,
                "Service (e.g. aura)",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::KeychainKey,
                &self.state.keychain_key,
                "Key name",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::KeychainValue,
                &self.state.keychain_value,
                "Secret value",
                true,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::VaultAddr,
                &self.state.vault_config.vault_addr,
                "https://vault.example.com:8200",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::VaultToken,
                &self.state.vault_config.vault_token,
                "Vault token",
                true,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::VaultNamespace,
                self.state
                    .vault_config
                    .vault_namespace
                    .as_deref()
                    .unwrap_or(""),
                "Namespace (optional)",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::VaultAwsRegion,
                &self.state.vault_config.aws_region,
                "AWS Region (e.g. us-east-1)",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::VaultAwsProfile,
                self.state.vault_config.aws_profile.as_deref().unwrap_or(""),
                "AWS Profile (default)",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::DomainCertDomain,
                &self.state.draft_cert_domain,
                "*.api.example.com",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::DomainCertPath,
                &self.state.draft_cert_path,
                "Cert path (.pem/.pfx/.p12)",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::DomainCertKeyPath,
                &self.state.draft_cert_key_path,
                "Key path (optional PEM)",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::DomainCertPassphrase,
                &self.state.draft_cert_passphrase,
                "Passphrase",
                true,
                window,
                cx,
                Self::handle_input_event,
            );
        }

        if self.state.is_proxy_modal_open {
            let gproxy_port = self.state.global_proxy.port.to_string();
            let gproxy_bypass = self.state.global_proxy.bypass_list.join(", ");
            inputs.ensure_input(
                InputKey::GlobalProxyHost,
                &self.state.global_proxy.host,
                "Proxy host (e.g. 127.0.0.1)",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::GlobalProxyPort,
                &gproxy_port,
                "8080",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::GlobalProxyUsername,
                &self.state.global_proxy.username,
                "Username",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::GlobalProxyPassword,
                &self.state.global_proxy.password,
                "Password",
                true,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::GlobalProxyBypass,
                &gproxy_bypass,
                "localhost, 127.0.0.1",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
        }

        if self.state.is_cookie_modal_open {
            inputs.ensure_input(
                InputKey::CookieFilter,
                &self.state.cookie_domain_filter,
                "Filter by domain...",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::CookieDraftDomain,
                &self.state.draft_cookie_domain,
                "example.com",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::CookieDraftPath,
                &self.state.draft_cookie_path,
                "/",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::CookieDraftName,
                &self.state.draft_cookie_name,
                "session_id",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::CookieDraftValue,
                &self.state.draft_cookie_value,
                "Cookie value",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
        }

        if self.state.is_interceptor_modal_open {
            inputs.ensure_input(
                InputKey::InterceptorBindAddr,
                &self.state.interceptor_bind_addr,
                "127.0.0.1:8888",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::InterceptorRuleMatch,
                &self.state.draft_rule_match,
                "Match URL substring (e.g. /v1/)",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::InterceptorRuleTarget,
                &self.state.draft_rule_target,
                "Rewrite replacement (optional)",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
        }

        if self.state.is_git_modal_open {
            inputs.ensure_input(
                InputKey::GitCommitMessage,
                &self.state.git_commit_message,
                "Commit message (e.g. Add auth requests)...",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::GitNewBranchName,
                &self.state.git_new_branch_name,
                "New branch name...",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
        }

        if self.state.is_load_tester_modal_open {
            inputs.ensure_input(
                InputKey::LoadIterations,
                &self.state.load_iterations_str,
                "20",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::LoadConcurrency,
                &self.state.load_concurrency_str,
                "4",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::LoadDelayMs,
                &self.state.load_delay_ms_str,
                "0",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::LoadRampUpSecs,
                &self.state.load_ramp_up_str,
                "5",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::LoadRampDownSecs,
                &self.state.load_ramp_down_str,
                "5",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::LoadDurationSecs,
                &self.state.load_duration_str,
                "20",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::LoadTargetRps,
                &self.state.load_target_rps_str,
                "50",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            for (idx, (vus, secs)) in self.state.load_stages.iter().enumerate() {
                inputs.ensure_input(
                    InputKey::LoadStageVus(idx),
                    vus,
                    "10",
                    false,
                    window,
                    cx,
                    Self::handle_input_event,
                );
                inputs.ensure_input(
                    InputKey::LoadStageDurationSecs(idx),
                    secs,
                    "15",
                    false,
                    window,
                    cx,
                    Self::handle_input_event,
                );
            }
            inputs.ensure_input(
                InputKey::LoadThinkA,
                &self.state.load_think_a_str,
                "200",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::LoadThinkB,
                &self.state.load_think_b_str,
                "600",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::LoadBarrierGroupSize,
                &self.state.load_barrier_size_str,
                "4",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::LoadBarrierTimeoutMs,
                &self.state.load_barrier_timeout_str,
                "5000",
                false,
                window,
                cx,
                Self::handle_input_event,
            );

            let active_flow = self
                .state
                .active_request()
                .and_then(|r| r.flow.as_ref());
            let w_str = active_flow
                .and_then(|f| f.weight_percent)
                .map(|v| v.to_string())
                .unwrap_or_default();
            let lc_str = active_flow
                .and_then(|f| f.loop_count)
                .map(|v| v.to_string())
                .unwrap_or_default();
            let if_str = active_flow
                .and_then(|f| f.condition_if.as_deref())
                .unwrap_or("");
            let while_str = active_flow
                .and_then(|f| f.loop_while.as_deref())
                .unwrap_or("");

            inputs.ensure_input(
                InputKey::FlowWeightPercent,
                &w_str,
                "50",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::FlowLoopCount,
                &lc_str,
                "1",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::FlowConditionIf,
                if_str,
                "prev.status == 200",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::FlowLoopWhile,
                while_str,
                "res.status == 202",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::LoadDataFilePath,
                &self.state.load_data_file_path,
                "Path to .csv or .json data file...",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::LoadMaxDurationMs,
                &self.state.load_max_duration_ms_str,
                "Max ms",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::LoadMinSizeBytes,
                &self.state.load_min_size_bytes_str,
                "Min B",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::LoadMaxSizeBytes,
                &self.state.load_max_size_bytes_str,
                "Max B",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::LoadSlaP95Ms,
                &self.state.load_sla_p95_str,
                "p95 ms",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::LoadSlaP99Ms,
                &self.state.load_sla_p99_str,
                "p99 ms",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
            inputs.ensure_input(
                InputKey::LoadSlaErrorPct,
                &self.state.load_sla_err_str,
                "Err %",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
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

