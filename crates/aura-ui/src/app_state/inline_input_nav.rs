use super::{
    ActiveInputTarget, AuraAppState,
};

impl AuraAppState {
    /// Pastes a text string (such as from system clipboard) into the currently active input target
    pub fn paste_text_into_active(&mut self, text: &str) {
        let target = match &self.active_input_target {
            Some(t) => t.clone(),
            None => return,
        };

        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            match target {
                ActiveInputTarget::Url => {
                    tab.request.url.push_str(text);
                    tab.mark_dirty(true);
                    Self::sync_url_to_params(&mut tab.request);
                }
                ActiveInputTarget::ParamKey(idx) => {
                    if let Some(p) = tab.request.params.get_mut(idx) {
                        p.key.push_str(text);
                        tab.mark_dirty(true);
                        Self::sync_params_to_url(&mut tab.request);
                    }
                }
                ActiveInputTarget::ParamValue(idx) => {
                    if let Some(p) = tab.request.params.get_mut(idx) {
                        p.value.push_str(text);
                        tab.mark_dirty(true);
                        Self::sync_params_to_url(&mut tab.request);
                    }
                }
                ActiveInputTarget::HeaderKey(idx) => {
                    if let Some(h) = tab.request.headers.get_mut(idx) {
                        h.key.push_str(text);
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::HeaderValue(idx) => {
                    if let Some(h) = tab.request.headers.get_mut(idx) {
                        h.value.push_str(text);
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::FormParamKey(idx) => {
                    if let aura_core::model::BodyType::FormUrlEncoded(items) = &mut tab.request.body {
                        if let Some(item) = items.get_mut(idx) {
                            item.key.push_str(text);
                            tab.mark_dirty(true);
                        }
                    }
                }
                ActiveInputTarget::FormParamValue(idx) => {
                    if let aura_core::model::BodyType::FormUrlEncoded(items) = &mut tab.request.body {
                        if let Some(item) = items.get_mut(idx) {
                            item.value.push_str(text);
                            tab.mark_dirty(true);
                        }
                    }
                }
                ActiveInputTarget::AuthBearerToken => {
                    match &mut tab.request.auth {
                        aura_core::model::AuthConfig::Bearer { token } => token.push_str(text),
                        _ => tab.request.auth = aura_core::model::AuthConfig::Bearer { token: text.to_string() },
                    }
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::AuthBasicUsername => {
                    match &mut tab.request.auth {
                        aura_core::model::AuthConfig::Basic { username, .. } => username.push_str(text),
                        _ => tab.request.auth = aura_core::model::AuthConfig::Basic { username: text.to_string(), password: String::new() },
                    }
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::AuthBasicPassword => {
                    match &mut tab.request.auth {
                        aura_core::model::AuthConfig::Basic { password, .. } => password.push_str(text),
                        _ => tab.request.auth = aura_core::model::AuthConfig::Basic { username: String::new(), password: text.to_string() },
                    }
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::AuthApiKeyName => {
                    match &mut tab.request.auth {
                        aura_core::model::AuthConfig::ApiKey { key, .. } => key.push_str(text),
                        _ => tab.request.auth = aura_core::model::AuthConfig::ApiKey { key: text.to_string(), value: String::new(), in_header: true },
                    }
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::AuthApiKeyValue => {
                    match &mut tab.request.auth {
                        aura_core::model::AuthConfig::ApiKey { value, .. } => value.push_str(text),
                        _ => tab.request.auth = aura_core::model::AuthConfig::ApiKey { key: String::new(), value: text.to_string(), in_header: true },
                    }
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::AuthAwsField(field_idx) => {
                    if let aura_core::model::AuthConfig::AwsSigV4 {
                        access_key,
                        secret_key,
                        region,
                        service,
                        session_token,
                    } = &mut tab.request.auth
                    {
                        match field_idx {
                            0 => access_key.push_str(text),
                            1 => secret_key.push_str(text),
                            2 => region.push_str(text),
                            3 => service.push_str(text),
                            _ => {
                                if let Some(st) = session_token {
                                    st.push_str(text);
                                } else {
                                    *session_token = Some(text.to_string());
                                }
                            }
                        }
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::AuthOAuth2Field(field_idx) => {
                    if let aura_core::model::AuthConfig::OAuth2 {
                        grant_type,
                        access_token,
                        token_url,
                        client_id,
                        client_secret,
                    } = &mut tab.request.auth
                    {
                        match field_idx {
                            0 => access_token.push_str(text),
                            1 => grant_type.push_str(text),
                            2 => {
                                if let Some(tu) = token_url {
                                    tu.push_str(text);
                                } else {
                                    *token_url = Some(text.to_string());
                                }
                            }
                            3 => {
                                if let Some(ci) = client_id {
                                    ci.push_str(text);
                                } else {
                                    *client_id = Some(text.to_string());
                                }
                            }
                            _ => {
                                if let Some(cs) = client_secret {
                                    cs.push_str(text);
                                } else {
                                    *client_secret = Some(text.to_string());
                                }
                            }
                        }
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::BodyText => {
                    match &mut tab.request.body {
                        aura_core::model::BodyType::Json(s)
                        | aura_core::model::BodyType::Xml(s)
                        | aura_core::model::BodyType::Text(s) => {
                            s.push_str(text);
                        }
                        aura_core::model::BodyType::Raw { content: s, .. } => {
                            s.push_str(text);
                        }
                        _ => {
                            tab.request.body = aura_core::model::BodyType::Json(text.to_string());
                        }
                    }
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::PreRequestScript => {
                    tab.request.scripts.pre_request.push_str(text);
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::PostResponseScript => {
                    tab.request.scripts.post_response.push_str(text);
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::TestScript => {
                    tab.request.scripts.tests.push_str(text);
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::SpecText => {
                    tab.ensure_spec_buffer();
                    let mut curr = tab.spec_buffer.clone().unwrap_or_default();
                    curr.push_str(text);
                    tab.update_spec_buffer(curr);
                }
                ActiveInputTarget::VariableKey(idx) => {
                    if let Some((k, _)) = tab.variable_rows.get_mut(idx) {
                        k.push_str(text);
                        tab.sync_variables_from_rows();
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::VariableValue(idx) => {
                    if let Some((_, v)) = tab.variable_rows.get_mut(idx) {
                        v.push_str(text);
                        tab.sync_variables_from_rows();
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::SettingTimeout | ActiveInputTarget::SettingMaxRedirects => {}
            }
        }
    }

    pub fn tab_next_input(&mut self, shift: bool) {
        let target = match &self.active_input_target {
            Some(t) => t.clone(),
            None => return,
        };

        if shift {
            match target {
                ActiveInputTarget::ParamValue(idx) => {
                    self.active_input_target = Some(ActiveInputTarget::ParamKey(idx));
                }
                ActiveInputTarget::ParamKey(idx) if idx > 0 => {
                    self.active_input_target = Some(ActiveInputTarget::ParamValue(idx - 1));
                }
                ActiveInputTarget::HeaderValue(idx) => {
                    self.active_input_target = Some(ActiveInputTarget::HeaderKey(idx));
                }
                ActiveInputTarget::HeaderKey(idx) if idx > 0 => {
                    self.active_input_target = Some(ActiveInputTarget::HeaderValue(idx - 1));
                }
                ActiveInputTarget::FormParamValue(idx) => {
                    self.active_input_target = Some(ActiveInputTarget::FormParamKey(idx));
                }
                ActiveInputTarget::FormParamKey(idx) if idx > 0 => {
                    self.active_input_target = Some(ActiveInputTarget::FormParamValue(idx - 1));
                }
                ActiveInputTarget::AuthBasicPassword => {
                    self.active_input_target = Some(ActiveInputTarget::AuthBasicUsername);
                }
                ActiveInputTarget::AuthApiKeyValue => {
                    self.active_input_target = Some(ActiveInputTarget::AuthApiKeyName);
                }
                ActiveInputTarget::AuthAwsField(idx) if idx > 0 => {
                    self.active_input_target = Some(ActiveInputTarget::AuthAwsField(idx - 1));
                }
                ActiveInputTarget::AuthOAuth2Field(idx) if idx > 0 => {
                    self.active_input_target = Some(ActiveInputTarget::AuthOAuth2Field(idx - 1));
                }
                _ => {}
            }
        } else {
            match target {
                ActiveInputTarget::ParamKey(idx) => {
                    self.active_input_target = Some(ActiveInputTarget::ParamValue(idx));
                }
                ActiveInputTarget::ParamValue(idx) => {
                    let total = self
                        .tabs
                        .get(self.active_tab_index)
                        .map(|t| t.request.params.len())
                        .unwrap_or(0);
                    if idx + 1 < total {
                        self.active_input_target = Some(ActiveInputTarget::ParamKey(idx + 1));
                    } else {
                        self.add_param(String::new(), String::new());
                        self.active_input_target = Some(ActiveInputTarget::ParamKey(idx + 1));
                    }
                }
                ActiveInputTarget::HeaderKey(idx) => {
                    self.active_input_target = Some(ActiveInputTarget::HeaderValue(idx));
                }
                ActiveInputTarget::HeaderValue(idx) => {
                    let total = self
                        .tabs
                        .get(self.active_tab_index)
                        .map(|t| t.request.headers.len())
                        .unwrap_or(0);
                    if idx + 1 < total {
                        self.active_input_target = Some(ActiveInputTarget::HeaderKey(idx + 1));
                    } else {
                        self.add_header(String::new(), String::new());
                        self.active_input_target = Some(ActiveInputTarget::HeaderKey(idx + 1));
                    }
                }
                ActiveInputTarget::FormParamKey(idx) => {
                    self.active_input_target = Some(ActiveInputTarget::FormParamValue(idx));
                }
                ActiveInputTarget::FormParamValue(idx) => {
                    let total = self
                        .tabs
                        .get(self.active_tab_index)
                        .and_then(|t| match &t.request.body {
                            aura_core::model::BodyType::FormUrlEncoded(items) => Some(items.len()),
                            _ => None,
                        })
                        .unwrap_or(0);
                    if idx + 1 < total {
                        self.active_input_target = Some(ActiveInputTarget::FormParamKey(idx + 1));
                    } else {
                        self.add_form_urlencoded_param(String::new(), String::new());
                        self.active_input_target = Some(ActiveInputTarget::FormParamKey(idx + 1));
                    }
                }
                ActiveInputTarget::AuthBasicUsername => {
                    self.active_input_target = Some(ActiveInputTarget::AuthBasicPassword);
                }
                ActiveInputTarget::AuthBasicPassword => {
                    self.active_input_target = Some(ActiveInputTarget::AuthBasicUsername);
                }
                ActiveInputTarget::AuthApiKeyName => {
                    self.active_input_target = Some(ActiveInputTarget::AuthApiKeyValue);
                }
                ActiveInputTarget::AuthApiKeyValue => {
                    self.active_input_target = Some(ActiveInputTarget::AuthApiKeyName);
                }
                ActiveInputTarget::AuthAwsField(idx) => {
                    self.active_input_target = Some(ActiveInputTarget::AuthAwsField((idx + 1) % 5));
                }
                ActiveInputTarget::AuthOAuth2Field(idx) => {
                    self.active_input_target =
                        Some(ActiveInputTarget::AuthOAuth2Field((idx + 1) % 5));
                }
                ActiveInputTarget::BodyText
                | ActiveInputTarget::PreRequestScript
                | ActiveInputTarget::PostResponseScript
                | ActiveInputTarget::TestScript
                | ActiveInputTarget::SpecText => {
                    self.type_char_into_active(' ');
                    self.type_char_into_active(' ');
                }
                _ => {}
            }
        }
    }

    pub fn enter_active_input(&mut self) {
        let target = match &self.active_input_target {
            Some(t) => t.clone(),
            None => return,
        };

        match target {
            ActiveInputTarget::BodyText
            | ActiveInputTarget::PreRequestScript
            | ActiveInputTarget::PostResponseScript
            | ActiveInputTarget::TestScript
            | ActiveInputTarget::SpecText => {
                self.type_char_into_active('\n');
            }
            ActiveInputTarget::ParamKey(idx) => {
                self.active_input_target = Some(ActiveInputTarget::ParamValue(idx));
            }
            ActiveInputTarget::ParamValue(idx) => {
                let total = self
                    .tabs
                    .get(self.active_tab_index)
                    .map(|t| t.request.params.len())
                    .unwrap_or(0);
                if idx + 1 < total {
                    self.active_input_target = Some(ActiveInputTarget::ParamKey(idx + 1));
                } else {
                    self.add_param(String::new(), String::new());
                    self.active_input_target = Some(ActiveInputTarget::ParamKey(idx + 1));
                }
            }
            ActiveInputTarget::HeaderKey(idx) => {
                self.active_input_target = Some(ActiveInputTarget::HeaderValue(idx));
            }
            ActiveInputTarget::HeaderValue(idx) => {
                let total = self
                    .tabs
                    .get(self.active_tab_index)
                    .map(|t| t.request.headers.len())
                    .unwrap_or(0);
                if idx + 1 < total {
                    self.active_input_target = Some(ActiveInputTarget::HeaderKey(idx + 1));
                } else {
                    self.add_header(String::new(), String::new());
                    self.active_input_target = Some(ActiveInputTarget::HeaderKey(idx + 1));
                }
            }
            _ => {
                self.blur_active_input();
            }
        }
    }

    pub fn add_new_param_and_focus(&mut self) {
        let new_idx = self
            .tabs
            .get(self.active_tab_index)
            .map(|t| t.request.params.len())
            .unwrap_or(0);
        self.add_param(String::new(), String::new());
        self.active_input_target = Some(ActiveInputTarget::ParamKey(new_idx));
    }

    pub fn add_new_header_and_focus(&mut self) {
        let new_idx = self
            .tabs
            .get(self.active_tab_index)
            .map(|t| t.request.headers.len())
            .unwrap_or(0);
        self.add_header(String::new(), String::new());
        self.active_input_target = Some(ActiveInputTarget::HeaderKey(new_idx));
    }

    pub fn add_new_form_param_and_focus(&mut self) {
        let new_idx = self
            .tabs
            .get(self.active_tab_index)
            .and_then(|t| match &t.request.body {
                aura_core::model::BodyType::FormUrlEncoded(items) => Some(items.len()),
                _ => None,
            })
            .unwrap_or(0);
        self.add_form_urlencoded_param(String::new(), String::new());
        self.active_input_target = Some(ActiveInputTarget::FormParamKey(new_idx));
    }

}
