use super::{
    AuraAppState, EditorDialogState,
};

impl AuraAppState {
    pub fn open_add_header_dialog(&mut self) {
        self.editor_dialog = Some(EditorDialogState::AddHeader {
            key: String::new(),
            value: String::new(),
            is_value_focused: false,
        });
    }

    pub fn open_edit_header_dialog(&mut self, index: usize) {
        if let Some(tab) = self.tabs.get(self.active_tab_index) {
            if let Some(h) = tab.request.headers.get(index) {
                self.editor_dialog = Some(EditorDialogState::EditHeader {
                    index,
                    key: h.key.clone(),
                    value: h.value.clone(),
                    is_value_focused: false,
                });
            }
        }
    }

    pub fn open_add_param_dialog(&mut self) {
        self.editor_dialog = Some(EditorDialogState::AddParam {
            key: String::new(),
            value: String::new(),
            is_value_focused: false,
        });
    }

    pub fn open_edit_param_dialog(&mut self, index: usize) {
        if let Some(tab) = self.tabs.get(self.active_tab_index) {
            if let Some(p) = tab.request.params.get(index) {
                self.editor_dialog = Some(EditorDialogState::EditParam {
                    index,
                    key: p.key.clone(),
                    value: p.value.clone(),
                    is_value_focused: false,
                });
            }
        }
    }

    pub fn open_edit_body_dialog(&mut self) {
        let content = if let Some(tab) = self.tabs.get(self.active_tab_index) {
            tab.request.body.content_str().to_string()
        } else {
            String::new()
        };
        self.editor_dialog = Some(EditorDialogState::EditBody { content });
    }

    pub fn open_edit_auth_dialog(&mut self) {
        if let Some(tab) = self.tabs.get(self.active_tab_index) {
            match &tab.request.auth {
                aura_core::model::AuthConfig::Bearer { token } => {
                    self.editor_dialog = Some(EditorDialogState::EditBearerAuth {
                        token: token.clone(),
                    });
                }
                aura_core::model::AuthConfig::Basic { username, password } => {
                    self.editor_dialog = Some(EditorDialogState::EditBasicAuth {
                        username: username.clone(),
                        password: password.clone(),
                        is_password_focused: false,
                    });
                }
                aura_core::model::AuthConfig::ApiKey {
                    key,
                    value,
                    in_header,
                } => {
                    self.editor_dialog = Some(EditorDialogState::EditApiKeyAuth {
                        key: key.clone(),
                        value: value.clone(),
                        in_header: *in_header,
                        is_value_focused: false,
                    });
                }
                aura_core::model::AuthConfig::AwsSigV4 {
                    access_key,
                    secret_key,
                    region,
                    service,
                    session_token,
                    ..
                } => {
                    self.editor_dialog = Some(EditorDialogState::EditAwsSigV4 {
                        access_key: access_key.clone(),
                        secret_key: secret_key.clone(),
                        region: region.clone(),
                        service: service.clone(),
                        session_token: session_token.clone().unwrap_or_default(),
                        active_field: 0,
                    });
                }
                aura_core::model::AuthConfig::OAuth2 {
                    grant_type,
                    access_token,
                    token_url,
                    client_id,
                    client_secret,
                } => {
                    self.editor_dialog = Some(EditorDialogState::EditOAuth2 {
                        grant_type: grant_type.clone(),
                        access_token: access_token.clone(),
                        token_url: token_url.clone().unwrap_or_default(),
                        client_id: client_id.clone().unwrap_or_default(),
                        client_secret: client_secret.clone().unwrap_or_default(),
                        active_field: 0,
                    });
                }
                _ => {}
            }
        }
    }

    pub fn open_add_form_param_dialog(&mut self) {
        self.editor_dialog = Some(EditorDialogState::AddFormParam {
            key: String::new(),
            value: String::new(),
            is_value_focused: false,
        });
    }

    pub fn open_add_multipart_field_dialog(&mut self) {
        self.editor_dialog = Some(EditorDialogState::AddMultipartField {
            name: String::new(),
            value: String::new(),
            file_path: String::new(),
            is_file: false,
            active_field: 0,
        });
    }

    pub fn cancel_editor_dialog(&mut self) {
        self.editor_dialog = None;
    }

    pub fn confirm_editor_dialog(&mut self) {
        if let Some(dialog) = self.editor_dialog.take() {
            match dialog {
                EditorDialogState::AddHeader { key, value, .. } => {
                    let k = key.trim();
                    if !k.is_empty() {
                        self.add_header(k, value);
                    }
                }
                EditorDialogState::EditHeader {
                    index, key, value, ..
                } => {
                    self.update_header(index, key, value);
                }
                EditorDialogState::AddParam { key, value, .. } => {
                    let k = key.trim();
                    if !k.is_empty() {
                        self.add_param(k, value);
                    }
                }
                EditorDialogState::EditParam {
                    index, key, value, ..
                } => {
                    self.update_param(index, key, value);
                }
                EditorDialogState::EditBody { content } => {
                    self.set_body_content(content);
                }
                EditorDialogState::EditBearerAuth { token } => {
                    self.update_bearer_token(token);
                }
                EditorDialogState::EditBasicAuth {
                    username, password, ..
                } => {
                    self.update_basic_auth(username, password);
                }
                EditorDialogState::EditApiKeyAuth {
                    key,
                    value,
                    in_header,
                    ..
                } => {
                    self.update_api_key(key, value, in_header);
                }
                EditorDialogState::EditAwsSigV4 {
                    access_key,
                    secret_key,
                    region,
                    service,
                    session_token,
                    ..
                } => {
                    let st = if session_token.trim().is_empty() {
                        None
                    } else {
                        Some(session_token)
                    };
                    self.update_aws_sigv4(access_key, secret_key, region, service, st);
                }
                EditorDialogState::EditOAuth2 {
                    grant_type,
                    access_token,
                    token_url,
                    client_id,
                    client_secret,
                    ..
                } => {
                    let tu = if token_url.trim().is_empty() {
                        None
                    } else {
                        Some(token_url)
                    };
                    let ci = if client_id.trim().is_empty() {
                        None
                    } else {
                        Some(client_id)
                    };
                    let cs = if client_secret.trim().is_empty() {
                        None
                    } else {
                        Some(client_secret)
                    };
                    self.update_oauth2(grant_type, access_token, tu, ci, cs);
                }
                EditorDialogState::AddFormParam { key, value, .. } => {
                    let k = key.trim();
                    if !k.is_empty() {
                        self.add_form_urlencoded_param(k, value);
                    }
                }
                EditorDialogState::AddMultipartField {
                    name,
                    value,
                    file_path,
                    is_file,
                    ..
                } => {
                    let n = name.trim();
                    if !n.is_empty() {
                        if is_file {
                            self.add_multipart_field(n.to_string(), None, Some(file_path), None);
                        } else {
                            self.add_multipart_field(n.to_string(), Some(value), None, None);
                        }
                    }
                }
            }
        }
    }

    pub fn dialog_append_char(&mut self, c: char) {
        if let Some(ref mut dialog) = self.editor_dialog {
            match dialog {
                EditorDialogState::AddHeader {
                    key,
                    value,
                    is_value_focused,
                }
                | EditorDialogState::EditHeader {
                    key,
                    value,
                    is_value_focused,
                    ..
                }
                | EditorDialogState::AddParam {
                    key,
                    value,
                    is_value_focused,
                }
                | EditorDialogState::EditParam {
                    key,
                    value,
                    is_value_focused,
                    ..
                }
                | EditorDialogState::AddFormParam {
                    key,
                    value,
                    is_value_focused,
                } => {
                    if *is_value_focused {
                        value.push(c);
                    } else {
                        key.push(c);
                    }
                }
                EditorDialogState::EditBody { content } => {
                    content.push(c);
                }
                EditorDialogState::EditBearerAuth { token } => {
                    token.push(c);
                }
                EditorDialogState::EditBasicAuth {
                    username,
                    password,
                    is_password_focused,
                } => {
                    if *is_password_focused {
                        password.push(c);
                    } else {
                        username.push(c);
                    }
                }
                EditorDialogState::EditApiKeyAuth {
                    key,
                    value,
                    is_value_focused,
                    ..
                } => {
                    if *is_value_focused {
                        value.push(c);
                    } else {
                        key.push(c);
                    }
                }
                EditorDialogState::EditAwsSigV4 {
                    access_key,
                    secret_key,
                    region,
                    service,
                    session_token,
                    active_field,
                } => match active_field {
                    0 => access_key.push(c),
                    1 => secret_key.push(c),
                    2 => region.push(c),
                    3 => service.push(c),
                    _ => session_token.push(c),
                },
                EditorDialogState::EditOAuth2 {
                    grant_type,
                    access_token,
                    token_url,
                    client_id,
                    client_secret,
                    active_field,
                } => match active_field {
                    0 => access_token.push(c),
                    1 => grant_type.push(c),
                    2 => token_url.push(c),
                    3 => client_id.push(c),
                    _ => client_secret.push(c),
                },
                EditorDialogState::AddMultipartField {
                    name,
                    value,
                    file_path,
                    active_field,
                    ..
                } => match active_field {
                    0 => name.push(c),
                    1 => value.push(c),
                    _ => file_path.push(c),
                },
            }
        }
    }

    pub fn dialog_backspace(&mut self) {
        if let Some(ref mut dialog) = self.editor_dialog {
            match dialog {
                EditorDialogState::AddHeader {
                    key,
                    value,
                    is_value_focused,
                }
                | EditorDialogState::EditHeader {
                    key,
                    value,
                    is_value_focused,
                    ..
                }
                | EditorDialogState::AddParam {
                    key,
                    value,
                    is_value_focused,
                }
                | EditorDialogState::EditParam {
                    key,
                    value,
                    is_value_focused,
                    ..
                }
                | EditorDialogState::AddFormParam {
                    key,
                    value,
                    is_value_focused,
                } => {
                    if *is_value_focused {
                        value.pop();
                    } else {
                        key.pop();
                    }
                }
                EditorDialogState::EditBody { content } => {
                    content.pop();
                }
                EditorDialogState::EditBearerAuth { token } => {
                    token.pop();
                }
                EditorDialogState::EditBasicAuth {
                    username,
                    password,
                    is_password_focused,
                } => {
                    if *is_password_focused {
                        password.pop();
                    } else {
                        username.pop();
                    }
                }
                EditorDialogState::EditApiKeyAuth {
                    key,
                    value,
                    is_value_focused,
                    ..
                } => {
                    if *is_value_focused {
                        value.pop();
                    } else {
                        key.pop();
                    }
                }
                EditorDialogState::EditAwsSigV4 {
                    access_key,
                    secret_key,
                    region,
                    service,
                    session_token,
                    active_field,
                } => match active_field {
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
                        session_token.pop();
                    }
                },
                EditorDialogState::EditOAuth2 {
                    grant_type,
                    access_token,
                    token_url,
                    client_id,
                    client_secret,
                    active_field,
                } => match active_field {
                    0 => {
                        access_token.pop();
                    }
                    1 => {
                        grant_type.pop();
                    }
                    2 => {
                        token_url.pop();
                    }
                    3 => {
                        client_id.pop();
                    }
                    _ => {
                        client_secret.pop();
                    }
                },
                EditorDialogState::AddMultipartField {
                    name,
                    value,
                    file_path,
                    active_field,
                    ..
                } => match active_field {
                    0 => {
                        name.pop();
                    }
                    1 => {
                        value.pop();
                    }
                    _ => {
                        file_path.pop();
                    }
                },
            }
        }
    }

    /// Retrieves the current text of active dialog field for clipboard copy
    pub fn dialog_get_text(&self) -> Option<String> {
        let dialog = self.editor_dialog.as_ref()?;
        match dialog {
            EditorDialogState::AddHeader { key, value, is_value_focused }
            | EditorDialogState::EditHeader { key, value, is_value_focused, .. }
            | EditorDialogState::AddParam { key, value, is_value_focused }
            | EditorDialogState::EditParam { key, value, is_value_focused, .. }
            | EditorDialogState::AddFormParam { key, value, is_value_focused } => {
                if *is_value_focused {
                    Some(value.clone())
                } else {
                    Some(key.clone())
                }
            }
            EditorDialogState::EditBody { content } => Some(content.clone()),
            EditorDialogState::EditBearerAuth { token } => Some(token.clone()),
            EditorDialogState::EditBasicAuth { username, password, is_password_focused } => {
                if *is_password_focused {
                    Some(password.clone())
                } else {
                    Some(username.clone())
                }
            }
            EditorDialogState::EditApiKeyAuth { key, value, is_value_focused, .. } => {
                if *is_value_focused {
                    Some(value.clone())
                } else {
                    Some(key.clone())
                }
            }
            EditorDialogState::EditAwsSigV4 { access_key, secret_key, region, service, session_token, active_field } => {
                match active_field {
                    0 => Some(access_key.clone()),
                    1 => Some(secret_key.clone()),
                    2 => Some(region.clone()),
                    3 => Some(service.clone()),
                    _ => Some(session_token.clone()),
                }
            }
            EditorDialogState::EditOAuth2 { grant_type, access_token, token_url, client_id, client_secret, active_field } => {
                match active_field {
                    0 => Some(access_token.clone()),
                    1 => Some(grant_type.clone()),
                    2 => Some(token_url.clone()),
                    3 => Some(client_id.clone()),
                    _ => Some(client_secret.clone()),
                }
            }
            EditorDialogState::AddMultipartField { name, value, file_path, active_field, .. } => {
                match active_field {
                    0 => Some(name.clone()),
                    1 => Some(value.clone()),
                    _ => Some(file_path.clone()),
                }
            }
        }
    }

    /// Sets the text of active dialog field completely (used by cut / paste / replace)
    pub fn dialog_set_text(&mut self, text: String) {
        if let Some(ref mut dialog) = self.editor_dialog {
            match dialog {
                EditorDialogState::AddHeader { key, value, is_value_focused }
                | EditorDialogState::EditHeader { key, value, is_value_focused, .. }
                | EditorDialogState::AddParam { key, value, is_value_focused }
                | EditorDialogState::EditParam { key, value, is_value_focused, .. }
                | EditorDialogState::AddFormParam { key, value, is_value_focused } => {
                    if *is_value_focused {
                        *value = text;
                    } else {
                        *key = text;
                    }
                }
                EditorDialogState::EditBody { content } => *content = text,
                EditorDialogState::EditBearerAuth { token } => *token = text,
                EditorDialogState::EditBasicAuth { username, password, is_password_focused } => {
                    if *is_password_focused {
                        *password = text;
                    } else {
                        *username = text;
                    }
                }
                EditorDialogState::EditApiKeyAuth { key, value, is_value_focused, .. } => {
                    if *is_value_focused {
                        *value = text;
                    } else {
                        *key = text;
                    }
                }
                EditorDialogState::EditAwsSigV4 { access_key, secret_key, region, service, session_token, active_field } => {
                    match active_field {
                        0 => *access_key = text,
                        1 => *secret_key = text,
                        2 => *region = text,
                        3 => *service = text,
                        _ => *session_token = text,
                    }
                }
                EditorDialogState::EditOAuth2 { grant_type, access_token, token_url, client_id, client_secret, active_field } => {
                    match active_field {
                        0 => *access_token = text,
                        1 => *grant_type = text,
                        2 => *token_url = text,
                        3 => *client_id = text,
                        _ => *client_secret = text,
                    }
                }
                EditorDialogState::AddMultipartField { name, value, file_path, active_field, .. } => {
                    match active_field {
                        0 => *name = text,
                        1 => *value = text,
                        _ => *file_path = text,
                    }
                }
            }
        }
    }

    /// Pastes text into the currently active dialog field
    pub fn dialog_paste_text(&mut self, text: &str) {
        if let Some(ref mut dialog) = self.editor_dialog {
            match dialog {
                EditorDialogState::AddHeader { key, value, is_value_focused }
                | EditorDialogState::EditHeader { key, value, is_value_focused, .. }
                | EditorDialogState::AddParam { key, value, is_value_focused }
                | EditorDialogState::EditParam { key, value, is_value_focused, .. }
                | EditorDialogState::AddFormParam { key, value, is_value_focused } => {
                    if *is_value_focused {
                        value.push_str(text);
                    } else {
                        key.push_str(text);
                    }
                }
                EditorDialogState::EditBody { content } => content.push_str(text),
                EditorDialogState::EditBearerAuth { token } => token.push_str(text),
                EditorDialogState::EditBasicAuth { username, password, is_password_focused } => {
                    if *is_password_focused {
                        password.push_str(text);
                    } else {
                        username.push_str(text);
                    }
                }
                EditorDialogState::EditApiKeyAuth { key, value, is_value_focused, .. } => {
                    if *is_value_focused {
                        value.push_str(text);
                    } else {
                        key.push_str(text);
                    }
                }
                EditorDialogState::EditAwsSigV4 { access_key, secret_key, region, service, session_token, active_field } => {
                    match active_field {
                        0 => access_key.push_str(text),
                        1 => secret_key.push_str(text),
                        2 => region.push_str(text),
                        3 => service.push_str(text),
                        _ => session_token.push_str(text),
                    }
                }
                EditorDialogState::EditOAuth2 { grant_type, access_token, token_url, client_id, client_secret, active_field } => {
                    match active_field {
                        0 => access_token.push_str(text),
                        1 => grant_type.push_str(text),
                        2 => token_url.push_str(text),
                        3 => client_id.push_str(text),
                        _ => client_secret.push_str(text),
                    }
                }
                EditorDialogState::AddMultipartField { name, value, file_path, active_field, .. } => {
                    match active_field {
                        0 => name.push_str(text),
                        1 => value.push_str(text),
                        _ => file_path.push_str(text),
                    }
                }
            }
        }
    }

    pub fn dialog_tab_next_field(&mut self) {
        if let Some(ref mut dialog) = self.editor_dialog {
            match dialog {
                EditorDialogState::AddHeader {
                    is_value_focused, ..
                }
                | EditorDialogState::EditHeader {
                    is_value_focused, ..
                }
                | EditorDialogState::AddParam {
                    is_value_focused, ..
                }
                | EditorDialogState::EditParam {
                    is_value_focused, ..
                }
                | EditorDialogState::AddFormParam {
                    is_value_focused, ..
                }
                | EditorDialogState::EditBasicAuth {
                    is_password_focused: is_value_focused,
                    ..
                }
                | EditorDialogState::EditApiKeyAuth {
                    is_value_focused, ..
                } => {
                    *is_value_focused = !*is_value_focused;
                }
                EditorDialogState::EditAwsSigV4 { active_field, .. } => {
                    *active_field = (*active_field + 1) % 5;
                }
                EditorDialogState::EditOAuth2 { active_field, .. } => {
                    *active_field = (*active_field + 1) % 5;
                }
                EditorDialogState::AddMultipartField { active_field, .. } => {
                    *active_field = (*active_field + 1) % 3;
                }
                _ => {}
            }
        }
    }

}
