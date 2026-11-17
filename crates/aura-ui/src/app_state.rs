//! Main application state for UI coordinating Domain Stores and Services.

use crate::keybindings::{Action, KeybindingPreset};
use crate::stores::{
    EditorTab, LayoutOrientation, ResponseBodyMode, ResponseTab, WorkspaceStore, WorkspaceTab,
};
use aura_core::model::{AuraRequest, Collection, Folder, HttpMethod};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Sub-tabs within the Scripts panel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ScriptSubTab {
    #[default]
    PreRequest,
    PostResponse,
}

/// Active input focus target for modal-free inline editing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActiveInputTarget {
    ParamKey(usize),
    ParamValue(usize),
    HeaderKey(usize),
    HeaderValue(usize),
    FormParamKey(usize),
    FormParamValue(usize),
    AuthBearerToken,
    AuthBasicUsername,
    AuthBasicPassword,
    AuthApiKeyName,
    AuthApiKeyValue,
    AuthAwsField(usize),
    AuthOAuth2Field(usize),
    BodyText,
    PreRequestScript,
    PostResponseScript,
    TestScript,
}

/// Drag and drop payload for sidebar items
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DragItemPayload {
    pub path: PathBuf,
    pub is_folder: bool,
}

/// State for active inline renaming in the sidebar
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineRenameState {
    pub path: PathBuf,
    pub current_value: String,
}

/// State for active rename modal dialog
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameDialogState {
    pub path: PathBuf,
    pub original_name: String,
    pub current_name: String,
    pub is_folder: bool,
}

/// State for active request editor modal dialogs (headers, params, body, auth)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorDialogState {
    AddHeader {
        key: String,
        value: String,
        is_value_focused: bool,
    },
    EditHeader {
        index: usize,
        key: String,
        value: String,
        is_value_focused: bool,
    },
    AddParam {
        key: String,
        value: String,
        is_value_focused: bool,
    },
    EditParam {
        index: usize,
        key: String,
        value: String,
        is_value_focused: bool,
    },
    EditBody {
        content: String,
    },
    EditBearerAuth {
        token: String,
    },
    EditBasicAuth {
        username: String,
        password: String,
        is_password_focused: bool,
    },
    EditApiKeyAuth {
        key: String,
        value: String,
        in_header: bool,
        is_value_focused: bool,
    },
    EditAwsSigV4 {
        access_key: String,
        secret_key: String,
        region: String,
        service: String,
        session_token: String,
        active_field: usize,
    },
    EditOAuth2 {
        grant_type: String,
        access_token: String,
        token_url: String,
        client_id: String,
        client_secret: String,
        active_field: usize,
    },
    AddFormParam {
        key: String,
        value: String,
        is_value_focused: bool,
    },
    AddMultipartField {
        name: String,
        value: String,
        file_path: String,
        is_file: bool,
        active_field: usize,
    },
}

/// Command palette entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPaletteItem {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub method: Option<HttpMethod>,
}

/// Main application state for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuraAppState {
    pub tabs: Vec<WorkspaceTab>,
    pub active_tab_index: usize,
    pub split_ratio: f32, // 0.1 to 0.9 (default 0.5)
    pub orientation: LayoutOrientation,
    pub active_editor_tab: EditorTab,
    pub active_response_tab: ResponseTab,
    pub active_body_mode: ResponseBodyMode,
    pub is_code_mode: bool, // Dual-mode: Visual vs Code Editor (TOML)
    pub is_command_palette_open: bool,
    pub is_log_viewer_open: bool,
    pub log_filter_level: Option<aura_engine::LogLevel>,
    pub log_filter_subsystem: Option<String>,
    pub log_search: String,
    pub command_palette_search: String,
    pub selected_palette_index: usize,
    pub collections: Vec<Collection>,
    pub is_dragging_divider: bool,
    pub keybinding_preset: KeybindingPreset,
    pub status_message: Option<String>,
    pub workspace_store: WorkspaceStore,
    pub inline_rename: Option<InlineRenameState>,
    pub rename_dialog: Option<RenameDialogState>,
    pub drag_target: Option<PathBuf>,
    pub pending_delete_path: Option<PathBuf>,
    pub editor_dialog: Option<EditorDialogState>,
    pub active_input_target: Option<ActiveInputTarget>,
    pub active_script_sub_tab: ScriptSubTab,
    #[serde(skip, default = "default_workspace_service")]
    pub workspace_service: std::sync::Arc<crate::services::WorkspaceService>,
    #[serde(skip, default = "default_execution_service")]
    pub execution_service: std::sync::Arc<crate::services::ExecutionService>,
}

fn default_workspace_service() -> std::sync::Arc<crate::services::WorkspaceService> {
    std::sync::Arc::new(crate::services::WorkspaceService::new())
}

fn default_execution_service() -> std::sync::Arc<crate::services::ExecutionService> {
    std::sync::Arc::new(crate::services::ExecutionService::new())
}

impl Default for AuraAppState {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab_index: 0,
            split_ratio: 0.5,
            orientation: LayoutOrientation::Horizontal,
            active_editor_tab: EditorTab::Params,
            active_response_tab: ResponseTab::Body,
            active_body_mode: ResponseBodyMode::Pretty,
            is_code_mode: false,
            is_command_palette_open: false,
            is_log_viewer_open: false,
            log_filter_level: None,
            log_filter_subsystem: None,
            log_search: String::new(),
            command_palette_search: String::new(),
            selected_palette_index: 0,
            collections: Vec::new(),
            is_dragging_divider: false,
            keybinding_preset: KeybindingPreset::VsCode,
            status_message: None,
            workspace_store: WorkspaceStore::new(),
            inline_rename: None,
            rename_dialog: None,
            drag_target: None,
            pending_delete_path: None,
            editor_dialog: None,
            active_input_target: None,
            active_script_sub_tab: ScriptSubTab::PreRequest,
            workspace_service: default_workspace_service(),
            execution_service: default_execution_service(),
        }
    }
}

impl AuraAppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_editor_tab(&mut self, tab: EditorTab) {
        self.active_editor_tab = tab;
    }

    pub fn set_response_tab(&mut self, tab: ResponseTab) {
        self.active_response_tab = tab;
    }

    pub fn toggle_response_body_mode(&mut self) {
        self.active_body_mode = match self.active_body_mode {
            ResponseBodyMode::Pretty => ResponseBodyMode::Raw,
            ResponseBodyMode::Raw => ResponseBodyMode::Pretty,
        };
    }

    pub fn cancel_active_request(&self) {
        self.execution_service.cancel();
    }

    // =========================================================================
    // Active Inline Input & Navigation Operations
    // =========================================================================

    pub fn set_script_sub_tab(&mut self, tab: ScriptSubTab) {
        self.active_script_sub_tab = tab;
    }

    pub fn set_active_input(&mut self, target: Option<ActiveInputTarget>) {
        self.active_input_target = target;
    }

    pub fn blur_active_input(&mut self) {
        self.active_input_target = None;
    }

    pub fn type_char_into_active(&mut self, c: char) {
        let target = match &self.active_input_target {
            Some(t) => t.clone(),
            None => return,
        };

        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            match target {
                ActiveInputTarget::ParamKey(idx) => {
                    if let Some(p) = tab.request.params.get_mut(idx) {
                        p.key.push(c);
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::ParamValue(idx) => {
                    if let Some(p) = tab.request.params.get_mut(idx) {
                        p.value.push(c);
                        tab.mark_dirty(true);
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
                ActiveInputTarget::ParamKey(idx) => {
                    if let Some(p) = tab.request.params.get_mut(idx) {
                        p.key.pop();
                        tab.mark_dirty(true);
                    }
                }
                ActiveInputTarget::ParamValue(idx) => {
                    if let Some(p) = tab.request.params.get_mut(idx) {
                        p.value.pop();
                        tab.mark_dirty(true);
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
                | ActiveInputTarget::TestScript => {
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
            | ActiveInputTarget::TestScript => {
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
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if tab.request.scripts.tests.trim().is_empty() {
                return Err("No test script defined".to_string());
            }
            if let Some(ref mut resp) = tab.response {
                let working_vars = std::collections::HashMap::new();
                let mut script_rt = aura_scripting::AuraScriptRuntime::new(working_vars)
                    .map_err(|e| format!("Script runtime error: {e}"))?;
                script_rt
                    .inject_response_context(resp.status, &resp.body, &resp.headers)
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
            tab.mark_dirty(true);
        }
    }

    pub fn remove_param(&mut self, index: usize) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.remove_param(index);
            tab.mark_dirty(true);
        }
    }

    pub fn toggle_param(&mut self, index: usize) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.toggle_param(index);
            tab.mark_dirty(true);
        }
    }

    pub fn update_param(&mut self, index: usize, key: String, value: String) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if let Some(p) = tab.request.params.get_mut(index) {
                p.key = key;
                p.value = value;
                tab.mark_dirty(true);
            }
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

    pub fn request_delete(&mut self, path: PathBuf) {
        self.pending_delete_path = Some(path);
    }

    pub fn cancel_delete(&mut self) {
        self.pending_delete_path = None;
    }

    pub async fn confirm_delete(&mut self) -> anyhow::Result<()> {
        if let Some(target_path) = self.pending_delete_path.take() {
            if let Some(root) = self.workspace_store.workspace_root.clone() {
                self.workspace_service
                    .delete_item(&root, &target_path)
                    .await?;
                self.workspace_store.apply_delete(&target_path);
            } else if target_path.exists() {
                if target_path.is_dir() {
                    tokio::fs::remove_dir_all(&target_path).await?;
                } else {
                    tokio::fs::remove_file(&target_path).await?;
                }
                self.workspace_store.apply_delete(&target_path);
            }

            // Close matching or descendant open tabs
            let mut ids_to_close = Vec::new();
            for tab in &self.tabs {
                if let Some(ref fp) = tab.file_path {
                    if fp == &target_path || fp.starts_with(&target_path) {
                        ids_to_close.push(tab.id.clone());
                    }
                }
            }
            for id in ids_to_close {
                self.close_tab(&id);
            }
        }
        Ok(())
    }

    pub fn start_inline_rename(&mut self, path: PathBuf, initial_name: impl Into<String>) {
        let name = initial_name.into();
        self.inline_rename = Some(InlineRenameState {
            path: path.clone(),
            current_value: name,
        });
    }

    pub fn cancel_inline_rename(&mut self) {
        self.inline_rename = None;
    }

    /// Requests opening the rename modal dialog for a file or folder.
    pub fn request_rename(&mut self, path: PathBuf, is_folder: bool) {
        let original_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let current_name = if !is_folder && original_name.ends_with(".aura") {
            original_name.trim_end_matches(".aura").to_string()
        } else {
            original_name.clone()
        };
        self.rename_dialog = Some(RenameDialogState {
            path,
            original_name,
            current_name,
            is_folder,
        });
    }

    /// Cancels active rename modal dialog.
    pub fn cancel_rename(&mut self) {
        self.rename_dialog = None;
    }

    /// Updates the text value in active rename dialog.
    pub fn update_rename_text(&mut self, text: String) {
        if let Some(ref mut d) = self.rename_dialog {
            d.current_name = text;
        }
    }

    /// Appends a character to the text in active rename dialog.
    pub fn append_rename_char(&mut self, c: char) {
        if let Some(ref mut d) = self.rename_dialog {
            d.current_name.push(c);
        }
    }

    /// Removes the last character from the text in active rename dialog.
    pub fn backspace_rename(&mut self) {
        if let Some(ref mut d) = self.rename_dialog {
            d.current_name.pop();
        }
    }

    /// Updates tab titles and paths when a file or directory is renamed.
    pub fn handle_item_renamed(&mut self, old_path: &Path, new_path: &Path, new_name: &str) {
        for tab in &mut self.tabs {
            if let Some(ref fp) = tab.file_path {
                if fp == old_path {
                    tab.file_path = Some(new_path.to_path_buf());
                    tab.title = new_name.to_string();
                } else if fp.starts_with(old_path) {
                    if let Ok(rel) = fp.strip_prefix(old_path) {
                        tab.file_path = Some(new_path.join(rel));
                    }
                }
            }
        }
    }

    /// Confirms and commits the rename operation asynchronously to disk and stores.
    pub async fn confirm_rename(&mut self) -> anyhow::Result<Option<PathBuf>> {
        if let Some(rename_state) = self.rename_dialog.take() {
            let trimmed = rename_state.current_name.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            let old_path = rename_state.path;
            if let Some(root) = self.workspace_store.workspace_root.clone() {
                let new_path = self
                    .workspace_service
                    .rename_item(&root, &old_path, trimmed)
                    .await?;
                let final_name = new_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(trimmed);
                self.workspace_store
                    .apply_rename(&old_path, &new_path, final_name);
                self.handle_item_renamed(&old_path, &new_path, final_name);
                return Ok(Some(new_path));
            } else if old_path.exists() {
                let parent = old_path.parent().unwrap_or(&old_path);
                let final_name = if old_path.is_file() && !trimmed.ends_with(".aura") {
                    format!("{}.aura", trimmed)
                } else {
                    trimmed.to_string()
                };
                let new_path = parent.join(&final_name);
                tokio::fs::rename(&old_path, &new_path).await?;
                self.workspace_store
                    .apply_rename(&old_path, &new_path, &final_name);
                self.handle_item_renamed(&old_path, &new_path, &final_name);
                return Ok(Some(new_path));
            }
        }
        Ok(None)
    }

    /// Toggles folder expanded/collapsed state in the workspace store.
    pub fn toggle_folder(&mut self, path: &Path) {
        self.workspace_store.toggle_folder_expanded(path);
    }

    /// Opens a workspace directory, saving it to preferences and initializing the store tree.
    pub fn open_workspace_sync(&mut self, root: &Path) -> anyhow::Result<()> {
        let mut prefs = crate::preferences::AuraPreferences::load();
        prefs.last_workspace_path = Some(root.to_path_buf());
        let _ = prefs.save();

        let existing = aura_sync::workspace_meta::WorkspaceMetadata::load_from_disk(root).ok();
        let metadata =
            aura_sync::workspace_meta::WorkspaceMetadata::reconcile_with_disk(root, existing)?;
        let _ = metadata.save_to_disk(root);
        let tree = build_tree_from_metadata(root, &metadata.order)?;

        self.workspace_store
            .set_workspace(root.to_path_buf(), metadata, tree);
        Ok(())
    }

    /// Asynchronously opens a workspace directory.
    pub async fn open_workspace_directory(&mut self, root: &Path) -> anyhow::Result<()> {
        self.open_workspace_sync(root)
    }

    /// Opens a file in a tab, parsing request or replacing an unmodified untitled tab.
    pub fn open_file_in_tab(&mut self, path: &Path) -> anyhow::Result<()> {
        if let Some(pos) = self
            .tabs
            .iter()
            .position(|t| t.file_path.as_deref() == Some(path))
        {
            self.select_tab(pos);
            return Ok(());
        }

        let content = std::fs::read_to_string(path).unwrap_or_default();
        let file_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled");
        let req = if content.trim().is_empty() {
            AuraRequest::new(file_stem, HttpMethod::GET, "https://postman-echo.com/get")
        } else if let Ok(r) = aura_parser::parse_request(&content) {
            r
        } else {
            match aura_sync::toml_to_request(&content, None) {
                Ok(r) => r,
                Err(_) => {
                    AuraRequest::new(file_stem, HttpMethod::GET, "https://postman-echo.com/get")
                }
            }
        };

        for tab in &mut self.tabs {
            tab.is_active = false;
        }

        let mut new_tab = WorkspaceTab::new(req);
        if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
            new_tab.title = fname.to_string();
        }
        new_tab.file_path = Some(path.to_path_buf());

        if self.tabs.len() == 1
            && self.tabs[0].file_path.is_none()
            && !self.tabs[0].is_dirty
            && self.tabs[0].title == "Untitled Request"
        {
            self.tabs[0] = new_tab;
            self.active_tab_index = 0;
        } else {
            self.tabs.push(new_tab);
            self.active_tab_index = self.tabs.len() - 1;
        }

        Ok(())
    }

    /// Synchronously reconciles workspace store with disk changes and prunes deleted open tabs.
    pub fn reconcile_workspace_sync(&mut self) -> anyhow::Result<()> {
        if let Some(root) = self.workspace_store.workspace_root.clone() {
            let existing = aura_sync::workspace_meta::WorkspaceMetadata::load_from_disk(&root).ok();
            let metadata =
                aura_sync::workspace_meta::WorkspaceMetadata::reconcile_with_disk(&root, existing)?;
            let _ = metadata.save_to_disk(&root);
            let tree = build_tree_from_metadata(&root, &metadata.order)?;
            self.workspace_store.apply_external_sync(metadata, tree);

            let mut ids_to_close = Vec::new();
            for tab in &self.tabs {
                if let Some(ref fp) = tab.file_path {
                    if !fp.exists() {
                        ids_to_close.push(tab.id.clone());
                    }
                }
            }
            for id in ids_to_close {
                self.close_tab(&id);
            }
        }
        Ok(())
    }

    /// Reconciles workspace store with disk changes and prunes deleted open tabs.
    pub async fn reconcile_workspace_with_disk(&mut self) -> anyhow::Result<()> {
        self.reconcile_workspace_sync()
    }

    /// Discover and load collections from workspace directory
    pub fn load_workspace_collections(&mut self, dir: &Path) -> anyhow::Result<usize> {
        let mut loaded = 0;
        if !dir.exists() {
            return Ok(0);
        }
        let mut paths_to_check = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    paths_to_check.push(p);
                } else if p.is_dir() {
                    if let Ok(sub_entries) = std::fs::read_dir(&p) {
                        for sub in sub_entries.flatten() {
                            let sub_p = sub.path();
                            if sub_p.is_file() {
                                paths_to_check.push(sub_p);
                            }
                        }
                    }
                }
            }
        }

        for p in paths_to_check {
            let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if fname.ends_with(".collection.json")
                || fname.ends_with(".collection.toml")
                || fname.ends_with(".aura.json")
                || fname.ends_with(".aura.toml")
            {
                if let Ok(content) = std::fs::read_to_string(&p) {
                    if fname.ends_with(".json") {
                        if let Ok(col) = serde_json::from_str::<Collection>(&content) {
                            if !self.collections.iter().any(|c| c.id == col.id) {
                                self.collections.push(col);
                                loaded += 1;
                            }
                        }
                    } else if fname.ends_with(".toml") {
                        if let Ok(col) = toml::from_str::<Collection>(&content) {
                            if !self.collections.iter().any(|c| c.id == col.id) {
                                self.collections.push(col);
                                loaded += 1;
                            }
                        }
                    }
                }
            }
        }
        Ok(loaded)
    }

    /// Open request by collection and request IDs
    pub fn open_collection_request(&mut self, collection_id: &str, request_id: &str) -> bool {
        for col in &self.collections {
            if col.id == collection_id {
                if let Some(req) = col.requests.iter().find(|r| r.id == request_id) {
                    self.open_request(req.clone());
                    return true;
                }
                fn find_in_folders<'a>(
                    folders: &'a [Folder],
                    req_id: &str,
                ) -> Option<&'a AuraRequest> {
                    for f in folders {
                        if let Some(req) = f.requests.iter().find(|r| r.id == req_id) {
                            return Some(req);
                        }
                        if let Some(req) = find_in_folders(&f.subfolders, req_id) {
                            return Some(req);
                        }
                    }
                    None
                }
                if let Some(req) = find_in_folders(&col.folders, request_id) {
                    self.open_request(req.clone());
                    return true;
                }
            }
        }
        false
    }

    /// Open or focus a tab for a request
    pub fn open_request(&mut self, req: AuraRequest) {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == req.id) {
            self.select_tab(pos);
        } else {
            for t in &mut self.tabs {
                t.is_active = false;
            }
            let new_tab = WorkspaceTab::new(req);
            self.tabs.push(new_tab);
            self.active_tab_index = self.tabs.len() - 1;
        }
    }

    /// Persistent `+` action: Append new untitled request
    pub fn new_request(&mut self) {
        let req = AuraRequest::new(
            "Untitled Request",
            HttpMethod::GET,
            "https://postman-echo.com/get",
        );
        self.open_request(req);
    }

    /// Switch active tab by index
    pub fn select_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            for t in &mut self.tabs {
                t.is_active = false;
            }
            self.tabs[index].is_active = true;
            self.active_tab_index = index;
        }
    }

    /// Navigate to the next tab in circular order
    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            let next_idx = (self.active_tab_index + 1) % self.tabs.len();
            self.select_tab(next_idx);
        }
    }

    /// Navigate to the previous tab in circular order
    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            let prev_idx = if self.active_tab_index == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab_index - 1
            };
            self.select_tab(prev_idx);
        }
    }

    /// Close tab by ID
    pub fn close_tab(&mut self, id: &str) {
        self.execution_service.cancel();
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            self.tabs.remove(pos);
            if self.tabs.is_empty() {
                self.new_request();
            } else if self.active_tab_index >= self.tabs.len() {
                self.active_tab_index = self.tabs.len() - 1;
                self.tabs[self.active_tab_index].is_active = true;
            } else {
                self.tabs[self.active_tab_index].is_active = true;
            }
        }
    }

    /// Close the active tab
    pub fn close_active_tab(&mut self) {
        if !self.tabs.is_empty() && self.active_tab_index < self.tabs.len() {
            let id = self.tabs[self.active_tab_index].id.clone();
            self.close_tab(&id);
        }
    }

    /// Mark active tab as dirty or clean
    pub fn mark_active_tab_dirty(&mut self, is_dirty: bool) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.mark_dirty(is_dirty);
        }
    }

    /// Mark specific tab by ID as dirty or clean
    pub fn mark_tab_dirty(&mut self, id: &str, is_dirty: bool) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.mark_dirty(is_dirty);
        }
    }

    /// Save the active request and clear dirty indicator
    pub fn save_active_tab(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.mark_dirty(false);
            self.status_message = Some(format!("Saved: {}", tab.title));
        }
    }

    /// Adjust pane divider percentage (clamped between 0.1 and 0.9)
    pub fn set_split_ratio(&mut self, ratio: f32) {
        self.split_ratio = ratio.clamp(0.1, 0.9);
    }

    /// Start dragging divider
    pub fn start_dragging_divider(&mut self) {
        self.is_dragging_divider = true;
    }

    /// Stop dragging divider
    pub fn stop_dragging_divider(&mut self) {
        self.is_dragging_divider = false;
    }

    /// Adjust pane divider by delta ratio (clamped between 0.15 and 0.85)
    pub fn drag_divider(&mut self, delta_ratio: f32) {
        self.split_ratio = (self.split_ratio + delta_ratio).clamp(0.15, 0.85);
    }

    /// Reset pane divider to 50/50 split
    pub fn reset_divider(&mut self) {
        self.split_ratio = 0.50;
    }

    /// Toggle layout between Horizontal and Vertical
    pub fn toggle_orientation(&mut self) {
        self.orientation = match self.orientation {
            LayoutOrientation::Horizontal => LayoutOrientation::Vertical,
            LayoutOrientation::Vertical => LayoutOrientation::Horizontal,
        };
    }

    /// Open command palette modal
    pub fn open_command_palette(&mut self) {
        self.is_command_palette_open = true;
        self.selected_palette_index = 0;
    }

    /// Close command palette modal
    pub fn close_command_palette(&mut self) {
        self.is_command_palette_open = false;
        self.command_palette_search.clear();
        self.selected_palette_index = 0;
    }

    /// Toggle command palette modal
    pub fn toggle_command_palette(&mut self) {
        if self.is_command_palette_open {
            self.close_command_palette();
        } else {
            self.open_command_palette();
        }
    }

    /// Toggle log viewer modal/view
    pub fn toggle_log_viewer(&mut self) {
        self.is_log_viewer_open = !self.is_log_viewer_open;
        if self.is_log_viewer_open {
            tracing::info!(target: "aura_ui", "System log viewer opened");
        }
    }

    /// Set the minimum log level filter for the log viewer
    pub fn set_log_filter_level(&mut self, level: Option<aura_engine::LogLevel>) {
        self.log_filter_level = level;
    }

    /// Set the subsystem filter for the log viewer
    pub fn set_log_filter_subsystem(&mut self, subsystem: Option<String>) {
        self.log_filter_subsystem = subsystem;
    }

    /// Set the search query for the log viewer
    pub fn set_log_search(&mut self, query: impl Into<String>) {
        self.log_search = query.into();
    }

    /// Retrieve filtered log entries from the global broadcaster
    pub fn get_filtered_logs(&self) -> Vec<aura_engine::LogEntry> {
        aura_engine::LogBroadcaster::global().filter_entries(
            self.log_filter_level,
            self.log_filter_subsystem.as_deref(),
            if self.log_search.is_empty() {
                None
            } else {
                Some(&self.log_search)
            },
        )
    }

    /// Clear all logs in the ring buffer
    pub fn clear_logs(&mut self) {
        aura_engine::LogBroadcaster::global().clear();
        tracing::info!(target: "aura_ui", "System logs cleared from ring buffer");
    }

    /// Set active keybinding preset
    pub fn set_keybinding_preset(&mut self, preset: KeybindingPreset) {
        self.keybinding_preset = preset;
    }

    /// Filter items for command palette
    pub fn filtered_palette_items(&self) -> Vec<CommandPaletteItem> {
        let q = self.command_palette_search.to_lowercase();
        self.tabs
            .iter()
            .filter(|t| {
                q.is_empty()
                    || t.title.to_lowercase().contains(&q)
                    || t.request.url.to_lowercase().contains(&q)
            })
            .map(|t| CommandPaletteItem {
                id: t.id.clone(),
                title: t.title.clone(),
                subtitle: t.request.url.clone(),
                method: Some(t.method),
            })
            .collect()
    }

    /// Move down in command palette list
    pub fn palette_select_next(&mut self) {
        let items = self.filtered_palette_items();
        if !items.is_empty() {
            self.selected_palette_index = (self.selected_palette_index + 1) % items.len();
        }
    }

    /// Move up in command palette list
    pub fn palette_select_prev(&mut self) {
        let items = self.filtered_palette_items();
        if !items.is_empty() {
            self.selected_palette_index = if self.selected_palette_index == 0 {
                items.len() - 1
            } else {
                self.selected_palette_index - 1
            };
        }
    }

    /// Execute selected item in command palette
    pub fn execute_selected_palette_item(&mut self) -> Option<String> {
        let items = self.filtered_palette_items();
        if let Some(item) = items.get(self.selected_palette_index) {
            let id = item.id.clone();
            if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
                self.select_tab(pos);
            }
            self.close_command_palette();
            Some(id)
        } else {
            None
        }
    }

    /// Handle keyboard action directly
    pub fn handle_key_action(&mut self, action: Action) -> bool {
        match action {
            Action::SendRequest => {
                self.status_message = Some("Dispatching request...".to_string());
                true
            }
            Action::OpenCommandPalette => {
                self.open_command_palette();
                true
            }
            Action::CloseCommandPalette => {
                if self.is_command_palette_open {
                    self.close_command_palette();
                    true
                } else {
                    false
                }
            }
            Action::ToggleLogViewer => {
                self.toggle_log_viewer();
                true
            }
            Action::NewRequest => {
                self.new_request();
                true
            }
            Action::CloseActiveTab => {
                self.close_active_tab();
                true
            }
            Action::ToggleLayout => {
                self.toggle_orientation();
                true
            }
            Action::ResetDivider => {
                self.reset_divider();
                true
            }
            Action::SaveRequest => {
                self.save_active_tab();
                true
            }
            Action::NextTab => {
                self.next_tab();
                true
            }
            Action::PrevTab => {
                self.prev_tab();
                true
            }
            Action::TreeUp => {
                self.workspace_store.select_prev_node();
                true
            }
            Action::TreeDown => {
                self.workspace_store.select_next_node();
                true
            }
            Action::TreeExpand => {
                self.workspace_store.expand_selected();
                true
            }
            Action::TreeCollapse => {
                self.workspace_store.collapse_selected();
                true
            }
            Action::TreeOpen => {
                if let Some(selected) = self.workspace_store.selected_path.clone() {
                    if selected.is_file() {
                        let _ = self.open_file_in_tab(&selected);
                        return true;
                    }
                }
                false
            }
            Action::DuplicateItem => {
                if let Some(selected) = self.workspace_store.selected_path.clone() {
                    if let Some(root) = self.workspace_store.workspace_root.clone() {
                        let srv = self.workspace_service.clone();
                        let rt_handle = tokio::runtime::Handle::try_current();
                        if let Ok(handle) = rt_handle {
                            handle.spawn(async move {
                                let _ = srv.duplicate_item(&root, &selected).await;
                            });
                        }
                    }
                    return true;
                }
                false
            }
            Action::CopyItem => {
                self.workspace_store.copy_selected();
                true
            }
            Action::PasteItem => {
                if let Some(payload) = self.workspace_store.clipboard_item.clone() {
                    if let Some(root) = self.workspace_store.workspace_root.clone() {
                        let target = self
                            .workspace_store
                            .selected_path
                            .clone()
                            .unwrap_or_else(|| root.clone());
                        let target_dir = if target.is_dir() {
                            target
                        } else {
                            target.parent().unwrap_or(&target).to_path_buf()
                        };
                        let srv = self.workspace_service.clone();
                        let rt_handle = tokio::runtime::Handle::try_current();
                        if let Ok(handle) = rt_handle {
                            handle.spawn(async move {
                                let _ = srv.paste_item(&root, &payload, &target_dir).await;
                            });
                        }
                    }
                    return true;
                }
                false
            }
            Action::RenameItem => {
                if let Some(selected) = self.workspace_store.selected_path.clone() {
                    let name = selected
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();
                    let is_folder = selected.is_dir();
                    self.request_rename(selected.clone(), is_folder);
                    self.start_inline_rename(selected, name);
                    true
                } else {
                    false
                }
            }
            Action::DeleteItem => {
                if let Some(selected) = self.workspace_store.selected_path.clone() {
                    self.request_delete(selected);
                    true
                } else {
                    false
                }
            }
        }
    }
}

fn build_tree_from_metadata(
    dir: &Path,
    order: &[aura_sync::workspace_meta::WorkspaceOrderItem],
) -> anyhow::Result<Vec<crate::stores::workspace_store::WorkspaceTreeNode>> {
    use crate::stores::workspace_store::WorkspaceTreeNode;
    use aura_sync::workspace_meta::WorkspaceOrderItem;

    let mut tree = Vec::new();
    for item in order {
        match item {
            WorkspaceOrderItem::Folder { name, order } => {
                let folder_path = dir.join(name);
                let children = build_tree_from_metadata(&folder_path, order)?;
                tree.push(WorkspaceTreeNode::Folder {
                    name: name.clone(),
                    path: folder_path,
                    is_expanded: true,
                    children,
                });
            }
            WorkspaceOrderItem::Request { name } => {
                let req_path = dir.join(name);
                let method = if let Ok(content) = std::fs::read_to_string(&req_path) {
                    if let Ok(r) = aura_parser::parse_request(&content) {
                        r.method
                    } else if let Ok(req) = aura_sync::toml_to_request(&content, None) {
                        req.method
                    } else {
                        HttpMethod::GET
                    }
                } else {
                    HttpMethod::GET
                };
                tree.push(WorkspaceTreeNode::Request {
                    name: name.clone(),
                    path: req_path,
                    method,
                });
            }
        }
    }
    Ok(tree)
}
