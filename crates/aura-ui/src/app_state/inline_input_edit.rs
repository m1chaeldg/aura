use super::{
    ActiveInputTarget, AuraAppState,
};

impl AuraAppState {
    pub fn type_char_into_active(&mut self, c: char) {
        let target = match &self.active_input_target {
            Some(t) => t.clone(),
            None => return,
        };

        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            match target {
                ActiveInputTarget::Url => {
                    tab.request.url.push(c);
                    tab.mark_dirty(true);
                    Self::sync_url_to_params(&mut tab.request);
                }
                ActiveInputTarget::ParamKey(idx) => {
                    if let Some(p) = tab.request.params.get_mut(idx) {
                        p.key.push(c);
                        tab.mark_dirty(true);
                        Self::sync_params_to_url(&mut tab.request);
                    }
                }
                ActiveInputTarget::ParamValue(idx) => {
                    if let Some(p) = tab.request.params.get_mut(idx) {
                        p.value.push(c);
                        tab.mark_dirty(true);
                        Self::sync_params_to_url(&mut tab.request);
                    }
                }
                ActiveInputTarget::HeaderKey(idx) => {
                    if let Some(h) = tab.request.headers.get_mut(idx) {
                        h.key.push(c);
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::HeaderValue(idx) => {
                    if let Some(h) = tab.request.headers.get_mut(idx) {
                        h.value.push(c);
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::FormParamKey(idx) => {
                    if let aura_core::model::BodyType::FormUrlEncoded(items) = &mut tab.request.body
                    {
                        if let Some(item) = items.get_mut(idx) {
                            item.key.push(c);
                            tab.mark_dirty(true);
                        }
                    }
                }
                ActiveInputTarget::FormParamValue(idx) => {
                    if let aura_core::model::BodyType::FormUrlEncoded(items) = &mut tab.request.body
                    {
                        if let Some(item) = items.get_mut(idx) {
                            item.value.push(c);
                            tab.mark_dirty(true);
                        }
                    }
                }
                ActiveInputTarget::AuthBearerToken => {
                    match &mut tab.request.auth {
                        aura_core::model::AuthConfig::Bearer { token } => {
                            token.push(c);
                        }
                        _ => {
                            tab.request.auth = aura_core::model::AuthConfig::Bearer {
                                token: c.to_string(),
                            };
                        }
                    }
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::AuthBasicUsername => {
                    match &mut tab.request.auth {
                        aura_core::model::AuthConfig::Basic { username, .. } => {
                            username.push(c);
                        }
                        _ => {
                            tab.request.auth = aura_core::model::AuthConfig::Basic {
                                username: c.to_string(),
                                password: String::new(),
                            };
                        }
                    }
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::AuthBasicPassword => {
                    match &mut tab.request.auth {
                        aura_core::model::AuthConfig::Basic { password, .. } => {
                            password.push(c);
                        }
                        _ => {
                            tab.request.auth = aura_core::model::AuthConfig::Basic {
                                username: String::new(),
                                password: c.to_string(),
                            };
                        }
                    }
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::AuthApiKeyName => {
                    match &mut tab.request.auth {
                        aura_core::model::AuthConfig::ApiKey { key, .. } => {
                            key.push(c);
                        }
                        _ => {
                            tab.request.auth = aura_core::model::AuthConfig::ApiKey {
                                key: c.to_string(),
                                value: String::new(),
                                in_header: true,
                            };
                        }
                    }
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::AuthApiKeyValue => {
                    match &mut tab.request.auth {
                        aura_core::model::AuthConfig::ApiKey { value, .. } => {
                            value.push(c);
                        }
                        _ => {
                            tab.request.auth = aura_core::model::AuthConfig::ApiKey {
                                key: String::new(),
                                value: c.to_string(),
                                in_header: true,
                            };
                        }
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
                            0 => access_key.push(c),
                            1 => secret_key.push(c),
                            2 => region.push(c),
                            3 => service.push(c),
                            _ => {
                                if let Some(st) = session_token {
                                    st.push(c);
                                } else {
                                    *session_token = Some(c.to_string());
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
                            0 => access_token.push(c),
                            1 => grant_type.push(c),
                            2 => {
                                if let Some(tu) = token_url {
                                    tu.push(c);
                                } else {
                                    *token_url = Some(c.to_string());
                                }
                            }
                            3 => {
                                if let Some(ci) = client_id {
                                    ci.push(c);
                                } else {
                                    *client_id = Some(c.to_string());
                                }
                            }
                            _ => {
                                if let Some(cs) = client_secret {
                                    cs.push(c);
                                } else {
                                    *client_secret = Some(c.to_string());
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
                            s.push(c);
                        }
                        aura_core::model::BodyType::Raw { content: s, .. } => {
                            s.push(c);
                        }
                        _ => {
                            tab.request.body = aura_core::model::BodyType::Json(c.to_string());
                        }
                    }
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::PreRequestScript => {
                    tab.request.scripts.pre_request.push(c);
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::PostResponseScript => {
                    tab.request.scripts.post_response.push(c);
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::TestScript => {
                    tab.request.scripts.tests.push(c);
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::SpecText => {
                    tab.ensure_spec_buffer();
                    let mut curr = tab.spec_buffer.clone().unwrap_or_default();
                    curr.push(c);
                    tab.update_spec_buffer(curr);
                }
                ActiveInputTarget::VariableKey(idx) => {
                    if let Some((k, _, _)) = tab.variable_rows.get_mut(idx) {
                        k.push(c);
                        tab.sync_variables_from_rows();
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::VariableValue(idx) => {
                    if let Some((_, v, _)) = tab.variable_rows.get_mut(idx) {
                        v.push(c);
                        tab.sync_variables_from_rows();
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::SettingTimeout | ActiveInputTarget::SettingMaxRedirects => {}
            }
        }
    }

    pub fn backspace_active(&mut self) {
        let target = match &self.active_input_target {
            Some(t) => t.clone(),
            None => return,
        };

        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            match target {
                ActiveInputTarget::Url => {
                    tab.request.url.pop();
                    tab.mark_dirty(true);
                    Self::sync_url_to_params(&mut tab.request);
                }
                ActiveInputTarget::ParamKey(idx) => {
                    if let Some(p) = tab.request.params.get_mut(idx) {
                        p.key.pop();
                        tab.mark_dirty(true);
                        Self::sync_params_to_url(&mut tab.request);
                    }
                }
                ActiveInputTarget::ParamValue(idx) => {
                    if let Some(p) = tab.request.params.get_mut(idx) {
                        p.value.pop();
                        tab.mark_dirty(true);
                        Self::sync_params_to_url(&mut tab.request);
                    }
                }
                ActiveInputTarget::HeaderKey(idx) => {
                    if let Some(h) = tab.request.headers.get_mut(idx) {
                        h.key.pop();
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::HeaderValue(idx) => {
                    if let Some(h) = tab.request.headers.get_mut(idx) {
                        h.value.pop();
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::FormParamKey(idx) => {
                    if let aura_core::model::BodyType::FormUrlEncoded(items) = &mut tab.request.body
                    {
                        if let Some(item) = items.get_mut(idx) {
                            item.key.pop();
                            tab.mark_dirty(true);
                        }
                    }
                }
                ActiveInputTarget::FormParamValue(idx) => {
                    if let aura_core::model::BodyType::FormUrlEncoded(items) = &mut tab.request.body
                    {
                        if let Some(item) = items.get_mut(idx) {
                            item.value.pop();
                            tab.mark_dirty(true);
                        }
                    }
                }
                ActiveInputTarget::AuthBearerToken => {
                    if let aura_core::model::AuthConfig::Bearer { token } = &mut tab.request.auth {
                        token.pop();
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::AuthBasicUsername => {
                    if let aura_core::model::AuthConfig::Basic { username, .. } =
                        &mut tab.request.auth
                    {
                        username.pop();
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::AuthBasicPassword => {
                    if let aura_core::model::AuthConfig::Basic { password, .. } =
                        &mut tab.request.auth
                    {
                        password.pop();
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::AuthApiKeyName => {
                    if let aura_core::model::AuthConfig::ApiKey { key, .. } = &mut tab.request.auth
                    {
                        key.pop();
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::AuthApiKeyValue => {
                    if let aura_core::model::AuthConfig::ApiKey { value, .. } =
                        &mut tab.request.auth
                    {
                        value.pop();
                        tab.mark_dirty(true);
                    }
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
                            0 => {
                                access_key.pop();
                            }
                            1 => {
                                secret_key.pop();
                            }
                            2 => {
                                region.pop();
                            }
                            3 => {
                                service.pop();
                            }
                            _ => {
                                if let Some(st) = session_token {
                                    st.pop();
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
                            0 => {
                                access_token.pop();
                            }
                            1 => {
                                grant_type.pop();
                            }
                            2 => {
                                if let Some(tu) = token_url {
                                    tu.pop();
                                }
                            }
                            3 => {
                                if let Some(ci) = client_id {
                                    ci.pop();
                                }
                            }
                            _ => {
                                if let Some(cs) = client_secret {
                                    cs.pop();
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
                            s.pop();
                        }
                        aura_core::model::BodyType::Raw { content: s, .. } => {
                            s.pop();
                        }
                        _ => {}
                    }
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::PreRequestScript => {
                    tab.request.scripts.pre_request.pop();
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::PostResponseScript => {
                    tab.request.scripts.post_response.pop();
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::TestScript => {
                    tab.request.scripts.tests.pop();
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::SpecText => {
                    tab.ensure_spec_buffer();
                    let mut curr = tab.spec_buffer.clone().unwrap_or_default();
                    curr.pop();
                    tab.update_spec_buffer(curr);
                }
                ActiveInputTarget::VariableKey(idx) => {
                    if let Some((k, _, _)) = tab.variable_rows.get_mut(idx) {
                        k.pop();
                        tab.sync_variables_from_rows();
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::VariableValue(idx) => {
                    if let Some((_, v, _)) = tab.variable_rows.get_mut(idx) {
                        v.pop();
                        tab.sync_variables_from_rows();
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::SettingTimeout | ActiveInputTarget::SettingMaxRedirects => {}
            }
        }
    }

    /// Retrieves current text content of the active input target for clipboard copying
    pub fn get_active_input_text(&self) -> Option<String> {
        let target = self.active_input_target.as_ref()?;
        let tab = self.tabs.get(self.active_tab_index)?;

        match target {
            ActiveInputTarget::Url => Some(tab.request.url.clone()),
            ActiveInputTarget::ParamKey(idx) => tab.request.params.get(*idx).map(|p| p.key.clone()),
            ActiveInputTarget::ParamValue(idx) => tab.request.params.get(*idx).map(|p| p.value.clone()),
            ActiveInputTarget::HeaderKey(idx) => tab.request.headers.get(*idx).map(|h| h.key.clone()),
            ActiveInputTarget::HeaderValue(idx) => tab.request.headers.get(*idx).map(|h| h.value.clone()),
            ActiveInputTarget::FormParamKey(idx) => {
                if let aura_core::model::BodyType::FormUrlEncoded(items) = &tab.request.body {
                    items.get(*idx).map(|item| item.key.clone())
                } else {
                    None
                }
            }
            ActiveInputTarget::FormParamValue(idx) => {
                if let aura_core::model::BodyType::FormUrlEncoded(items) = &tab.request.body {
                    items.get(*idx).map(|item| item.value.clone())
                } else {
                    None
                }
            }
            ActiveInputTarget::AuthBearerToken => match &tab.request.auth {
                aura_core::model::AuthConfig::Bearer { token } => Some(token.clone()),
                _ => None,
            },
            ActiveInputTarget::AuthBasicUsername => match &tab.request.auth {
                aura_core::model::AuthConfig::Basic { username, .. } => Some(username.clone()),
                _ => None,
            },
            ActiveInputTarget::AuthBasicPassword => match &tab.request.auth {
                aura_core::model::AuthConfig::Basic { password, .. } => Some(password.clone()),
                _ => None,
            },
            ActiveInputTarget::AuthApiKeyName => match &tab.request.auth {
                aura_core::model::AuthConfig::ApiKey { key, .. } => Some(key.clone()),
                _ => None,
            },
            ActiveInputTarget::AuthApiKeyValue => match &tab.request.auth {
                aura_core::model::AuthConfig::ApiKey { value, .. } => Some(value.clone()),
                _ => None,
            },
            ActiveInputTarget::AuthAwsField(field_idx) => match &tab.request.auth {
                aura_core::model::AuthConfig::AwsSigV4 {
                    access_key,
                    secret_key,
                    region,
                    service,
                    session_token,
                } => match *field_idx {
                    0 => Some(access_key.clone()),
                    1 => Some(secret_key.clone()),
                    2 => Some(region.clone()),
                    3 => Some(service.clone()),
                    _ => Some(session_token.clone().unwrap_or_default()),
                },
                _ => None,
            },
            ActiveInputTarget::AuthOAuth2Field(field_idx) => match &tab.request.auth {
                aura_core::model::AuthConfig::OAuth2 {
                    grant_type,
                    access_token,
                    token_url,
                    client_id,
                    client_secret,
                } => match *field_idx {
                    0 => Some(access_token.clone()),
                    1 => Some(grant_type.clone()),
                    2 => Some(token_url.clone().unwrap_or_default()),
                    3 => Some(client_id.clone().unwrap_or_default()),
                    _ => Some(client_secret.clone().unwrap_or_default()),
                },
                _ => None,
            },
            ActiveInputTarget::BodyText => match &tab.request.body {
                aura_core::model::BodyType::Json(s)
                | aura_core::model::BodyType::Xml(s)
                | aura_core::model::BodyType::Text(s) => Some(s.clone()),
                aura_core::model::BodyType::Raw { content, .. } => Some(content.clone()),
                _ => Some(tab.request.body.content_str().to_string()),
            },
            ActiveInputTarget::PreRequestScript => Some(tab.request.scripts.pre_request.clone()),
            ActiveInputTarget::PostResponseScript => Some(tab.request.scripts.post_response.clone()),
            ActiveInputTarget::TestScript => Some(tab.request.scripts.tests.clone()),
            ActiveInputTarget::SpecText => {
                Some(tab.spec_buffer.clone().unwrap_or_else(|| aura_parser::request_to_spec(&tab.request)))
            }
            ActiveInputTarget::VariableKey(idx) => {
                tab.variable_rows.get(*idx).map(|(k, _, _)| k.clone())
            }
            ActiveInputTarget::VariableValue(idx) => {
                tab.variable_rows.get(*idx).map(|(_, v, _)| v.clone())
            }
            ActiveInputTarget::SettingTimeout => Some(
                self.setting_timeout_draft
                    .clone()
                    .unwrap_or_else(|| tab.request.settings.timeout_ms.to_string()),
            ),
            ActiveInputTarget::SettingMaxRedirects => Some(
                self.setting_max_redirects_draft
                    .clone()
                    .unwrap_or_else(|| tab.request.settings.max_redirects.to_string()),
            ),
        }
    }

    /// Sets the text content of a specific input target without altering `active_input_target`.
    pub fn set_input_text_for(&mut self, target: ActiveInputTarget, text: String) {
        let prev = self.active_input_target.clone();
        self.active_input_target = Some(target);
        self.set_active_input_text(text);
        self.active_input_target = prev;
    }

    /// Sets the text content of the active input target completely (used by cut / paste / replace)
    pub fn set_active_input_text(&mut self, text: String) {
        let target = match &self.active_input_target {
            Some(t) => t.clone(),
            None => return,
        };

        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            match target {
                ActiveInputTarget::Url => {
                    tab.request.url = text;
                    tab.mark_dirty(true);
                    Self::sync_url_to_params(&mut tab.request);
                }
                ActiveInputTarget::ParamKey(idx) => {
                    if let Some(p) = tab.request.params.get_mut(idx) {
                        p.key = text;
                        tab.mark_dirty(true);
                        Self::sync_params_to_url(&mut tab.request);
                    }
                }
                ActiveInputTarget::ParamValue(idx) => {
                    if let Some(p) = tab.request.params.get_mut(idx) {
                        p.value = text;
                        tab.mark_dirty(true);
                        Self::sync_params_to_url(&mut tab.request);
                    }
                }
                ActiveInputTarget::HeaderKey(idx) => {
                    if let Some(h) = tab.request.headers.get_mut(idx) {
                        h.key = text;
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::HeaderValue(idx) => {
                    if let Some(h) = tab.request.headers.get_mut(idx) {
                        h.value = text;
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::FormParamKey(idx) => {
                    if let aura_core::model::BodyType::FormUrlEncoded(items) = &mut tab.request.body {
                        if let Some(item) = items.get_mut(idx) {
                            item.key = text;
                            tab.mark_dirty(true);
                        }
                    }
                }
                ActiveInputTarget::FormParamValue(idx) => {
                    if let aura_core::model::BodyType::FormUrlEncoded(items) = &mut tab.request.body {
                        if let Some(item) = items.get_mut(idx) {
                            item.value = text;
                            tab.mark_dirty(true);
                        }
                    }
                }
                ActiveInputTarget::AuthBearerToken => {
                    tab.request.auth = aura_core::model::AuthConfig::Bearer { token: text };
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::AuthBasicUsername => {
                    let pwd = match &tab.request.auth {
                        aura_core::model::AuthConfig::Basic { password, .. } => password.clone(),
                        _ => String::new(),
                    };
                    tab.request.auth = aura_core::model::AuthConfig::Basic { username: text, password: pwd };
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::AuthBasicPassword => {
                    let user = match &tab.request.auth {
                        aura_core::model::AuthConfig::Basic { username, .. } => username.clone(),
                        _ => String::new(),
                    };
                    tab.request.auth = aura_core::model::AuthConfig::Basic { username: user, password: text };
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::AuthApiKeyName => {
                    let (val, in_hdr) = match &tab.request.auth {
                        aura_core::model::AuthConfig::ApiKey { value, in_header, .. } => (value.clone(), *in_header),
                        _ => (String::new(), true),
                    };
                    tab.request.auth = aura_core::model::AuthConfig::ApiKey { key: text, value: val, in_header: in_hdr };
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::AuthApiKeyValue => {
                    let (k, in_hdr) = match &tab.request.auth {
                        aura_core::model::AuthConfig::ApiKey { key, in_header, .. } => (key.clone(), *in_header),
                        _ => (String::new(), true),
                    };
                    tab.request.auth = aura_core::model::AuthConfig::ApiKey { key: k, value: text, in_header: in_hdr };
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
                            0 => *access_key = text,
                            1 => *secret_key = text,
                            2 => *region = text,
                            3 => *service = text,
                            _ => *session_token = if text.is_empty() { None } else { Some(text) },
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
                            0 => *access_token = text,
                            1 => *grant_type = text,
                            2 => *token_url = if text.is_empty() { None } else { Some(text) },
                            3 => *client_id = if text.is_empty() { None } else { Some(text) },
                            _ => *client_secret = if text.is_empty() { None } else { Some(text) },
                        }
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::BodyText => {
                    match &mut tab.request.body {
                        aura_core::model::BodyType::Json(s)
                        | aura_core::model::BodyType::Xml(s)
                        | aura_core::model::BodyType::Text(s) => {
                            *s = text;
                        }
                        aura_core::model::BodyType::Raw { content: s, .. } => {
                            *s = text;
                        }
                        _ => {
                            tab.request.body = aura_core::model::BodyType::Json(text);
                        }
                    }
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::PreRequestScript => {
                    tab.request.scripts.pre_request = text;
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::PostResponseScript => {
                    tab.request.scripts.post_response = text;
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::TestScript => {
                    tab.request.scripts.tests = text;
                    tab.mark_dirty(true);
                }
                ActiveInputTarget::SpecText => {
                    tab.update_spec_buffer(text);
                }
                ActiveInputTarget::VariableKey(idx) => {
                    if let Some((k, _, _)) = tab.variable_rows.get_mut(idx) {
                        *k = text;
                        tab.sync_variables_from_rows();
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::VariableValue(idx) => {
                    if let Some((_, v, _)) = tab.variable_rows.get_mut(idx) {
                        *v = text;
                        tab.sync_variables_from_rows();
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::SettingTimeout => {
                    self.update_setting_timeout(&text);
                }
                ActiveInputTarget::SettingMaxRedirects => {
                    self.update_setting_max_redirects(&text);
                }
            }
        }
    }

}
