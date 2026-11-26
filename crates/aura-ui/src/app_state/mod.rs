//! Main application state for UI coordinating Domain Stores and Services.

use crate::keybindings::KeybindingPreset;
use crate::stores::{
    EditorTab, LayoutOrientation, ResponseBodyMode, ResponseTab, WorkspaceStore, WorkspaceTab,
};
use aura_core::model::{AuraRequest, Collection, HttpMethod};
use gpui::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod editor_dialog;
mod inline_input_edit;
mod inline_input_nav;
mod item_dialogs;
mod layout_and_palette;
mod request_editing;
mod workspace_tabs;

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
    Url,
    ParamKey(usize),
    ParamValue(usize),
    HeaderKey(usize),
    HeaderValue(usize),
    VariableKey(usize),
    VariableValue(usize),
    SettingTimeout,
    SettingMaxRedirects,
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
    SpecText,
}

/// Drag and drop payload for sidebar items
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DragItemPayload {
    pub path: PathBuf,
    pub is_folder: bool,
}

impl gpui::Render for DragItemPayload {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex};
        let name = self
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Item");
        let icon_name = if self.is_folder {
            gpui_kit::assets::IconName::Folder
        } else {
            gpui_kit::assets::IconName::FileText
        };
        h_flex()
            .gap_1p5()
            .px_2()
            .py_1()
            .bg(cx.theme().popover)
            .border_1()
            .border_color(cx.theme().primary)
            .rounded(cx.theme().radius)
            .text_xs()
            .text_color(cx.theme().popover_foreground)
            .shadow_md()
            .child(crate::theme::icon(icon_name).xsmall())
            .child(name.to_string())
    }
}

/// Destination item for the Move To quick-pick dialog
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveToDestination {
    pub display_name: String,
    pub path: PathBuf,
    pub is_root: bool,
}

/// State for active Move To quick-pick dialog
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveToDialogState {
    pub source_path: PathBuf,
    pub search_query: String,
    pub selected_index: usize,
    pub destinations: Vec<MoveToDestination>,
}

impl MoveToDialogState {
    pub fn new(source_path: PathBuf, root: PathBuf, folders: Vec<PathBuf>) -> Self {
        let mut destinations = Vec::new();
        let root_name = root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Workspace Root");
        destinations.push(MoveToDestination {
            display_name: format!("/ (Root: {})", root_name),
            path: root.clone(),
            is_root: true,
        });
        for folder in folders {
            if folder == source_path || folder.starts_with(&source_path) {
                continue; // Cannot move into itself or its descendants
            }
            let rel = folder.strip_prefix(&root).unwrap_or(&folder);
            destinations.push(MoveToDestination {
                display_name: format!("/{}", rel.to_string_lossy().replace('\\', "/")),
                path: folder,
                is_root: false,
            });
        }
        Self {
            source_path,
            search_query: String::new(),
            selected_index: 0,
            destinations,
        }
    }

    pub fn filtered_destinations(&self) -> Vec<&MoveToDestination> {
        if self.search_query.trim().is_empty() {
            return self.destinations.iter().collect();
        }
        let q = self.search_query.to_lowercase();
        self.destinations
            .iter()
            .filter(|d| d.display_name.to_lowercase().contains(&q))
            .collect()
    }

    pub fn select_next(&mut self) {
        let total = self.filtered_destinations().len();
        if total > 0 {
            self.selected_index = (self.selected_index + 1) % total;
        }
    }

    pub fn select_prev(&mut self) {
        let total = self.filtered_destinations().len();
        if total > 0 {
            if self.selected_index == 0 {
                self.selected_index = total - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    pub fn selected_destination(&self) -> Option<&MoveToDestination> {
        let filtered = self.filtered_destinations();
        if filtered.is_empty() {
            None
        } else {
            let idx = self.selected_index.min(filtered.len() - 1);
            Some(filtered[idx])
        }
    }
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

/// Editing mode for the Environment Profile Editor modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnvironmentEditorMode {
    #[default]
    Ui,
    Code,
}

/// State for the modal Environment Profile Editor.
#[derive(Debug, Clone)]
pub struct EnvironmentEditorState {
    pub original_path: Option<PathBuf>,
    pub profile: aura_core::model::EnvironmentProfile,
    pub mode: EnvironmentEditorMode,
    pub code_buffer: String,
    pub code_error: Option<String>,
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
    #[serde(skip)]
    pub response_language_override: Option<crate::services::ResponseLanguage>,
    #[serde(default = "default_true")]
    pub response_soft_wrap: bool,
    #[serde(skip)]
    pub response_tree_collapsed: std::collections::HashSet<String>,
    #[serde(skip)]
    pub response_table_search: String,
    #[serde(skip)]
    pub response_table_sort_col: Option<usize>,
    #[serde(skip)]
    pub response_table_sort_asc: bool,
    #[serde(skip)]
    pub response_bi_chart_type: crate::services::ChartType,
    #[serde(skip)]
    pub response_bi_agg_op: crate::services::AggregationOp,
    #[serde(skip)]
    pub response_bi_x_col: Option<String>,
    #[serde(skip)]
    pub response_bi_y_col: Option<String>,
    #[serde(skip)]
    pub response_bi_pivot_col: Option<String>,
    pub is_code_mode: bool, // Dual-mode: Visual vs Code Editor (TOML)
    pub is_command_palette_open: bool,
    #[serde(skip)]
    pub is_method_dropdown_open: bool,
    #[serde(skip)]
    pub is_body_format_dropdown_open: bool,
    #[serde(skip)]
    pub is_response_lang_dropdown_open: bool,
    #[serde(skip)]
    pub is_env_dropdown_open: bool,
    pub is_log_viewer_open: bool,
    #[serde(default)]
    pub is_log_viewer_text_mode: bool,
    #[serde(default)]
    pub is_sidebar_collapsed: bool,
    pub log_filter_level: Option<aura_engine::LogLevel>,
    pub log_filter_subsystem: Option<String>,
    pub log_search: String,
    pub command_palette_search: String,
    pub selected_palette_index: usize,
    pub collections: Vec<Collection>,
    #[serde(skip)]
    pub environments: Vec<(PathBuf, aura_core::model::EnvironmentProfile)>,
    #[serde(default)]
    pub active_environment_id: Option<String>,
    #[serde(skip)]
    pub clipboard_environment: Option<aura_core::model::EnvironmentProfile>,
    #[serde(skip)]
    pub environment_editor: Option<EnvironmentEditorState>,
    pub is_dragging_divider: bool,
    pub keybinding_preset: KeybindingPreset,
    pub status_message: Option<String>,
    pub workspace_store: WorkspaceStore,
    pub inline_rename: Option<InlineRenameState>,
    pub rename_dialog: Option<RenameDialogState>,
    pub move_to_dialog: Option<MoveToDialogState>,
    pub drag_target: Option<PathBuf>,
    pub pending_delete_path: Option<PathBuf>,
    pub editor_dialog: Option<EditorDialogState>,
    pub active_input_target: Option<ActiveInputTarget>,
    pub active_script_sub_tab: ScriptSubTab,
    #[serde(skip)]
    pub show_script_reference: bool,
    #[serde(skip)]
    pub setting_timeout_draft: Option<String>,
    #[serde(skip)]
    pub setting_timeout_error: Option<String>,
    #[serde(skip)]
    pub setting_max_redirects_draft: Option<String>,
    #[serde(skip)]
    pub setting_max_redirects_error: Option<String>,
    #[serde(default = "default_auto_save_enabled")]
    pub auto_save_enabled: bool,
    #[serde(default = "default_auto_save_delay_ms")]
    pub auto_save_delay_ms: u64,
    #[serde(skip, default = "default_workspace_service")]
    pub workspace_service: std::sync::Arc<crate::services::WorkspaceService>,
    #[serde(skip, default = "default_execution_service")]
    pub execution_service: std::sync::Arc<crate::services::ExecutionService>,
}

fn default_true() -> bool {
    true
}

fn default_auto_save_enabled() -> bool {
    true
}

fn default_auto_save_delay_ms() -> u64 {
    1000
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
            response_language_override: None,
            response_soft_wrap: true,
            response_tree_collapsed: std::collections::HashSet::new(),
            response_table_search: String::new(),
            response_table_sort_col: None,
            response_table_sort_asc: true,
            response_bi_chart_type: crate::services::ChartType::Bar,
            response_bi_agg_op: crate::services::AggregationOp::Count,
            response_bi_x_col: None,
            response_bi_y_col: None,
            response_bi_pivot_col: None,
            is_code_mode: false,
            is_command_palette_open: false,
            is_method_dropdown_open: false,
            is_body_format_dropdown_open: false,
            is_response_lang_dropdown_open: false,
            is_env_dropdown_open: false,
            is_log_viewer_open: false,
            is_log_viewer_text_mode: false,
            is_sidebar_collapsed: false,
            log_filter_level: None,
            log_filter_subsystem: None,
            log_search: String::new(),
            command_palette_search: String::new(),
            selected_palette_index: 0,
            collections: Vec::new(),
            environments: Vec::new(),
            active_environment_id: None,
            clipboard_environment: None,
            environment_editor: None,
            is_dragging_divider: false,
            keybinding_preset: KeybindingPreset::VsCode,
            status_message: None,
            workspace_store: WorkspaceStore::new(),
            inline_rename: None,
            rename_dialog: None,
            move_to_dialog: None,
            drag_target: None,
            pending_delete_path: None,
            editor_dialog: None,
            active_input_target: None,
            active_script_sub_tab: ScriptSubTab::PreRequest,
            show_script_reference: false,
            setting_timeout_draft: None,
            setting_timeout_error: None,
            setting_max_redirects_draft: None,
            setting_max_redirects_error: None,
            auto_save_enabled: true,
            auto_save_delay_ms: 1000,
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
        if tab == EditorTab::Spec {
            if let Some(active_tab) = self.tabs.get_mut(self.active_tab_index) {
                active_tab.sync_spec_from_request();
            }
        }
    }

    pub fn active_tab(&self) -> Option<&WorkspaceTab> {
        self.tabs.get(self.active_tab_index)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut WorkspaceTab> {
        self.tabs.get_mut(self.active_tab_index)
    }

    pub fn active_request(&self) -> Option<&AuraRequest> {
        self.active_tab().map(|t| &t.request)
    }

    pub fn active_request_mut(&mut self) -> Option<&mut AuraRequest> {
        self.active_tab_mut().map(|t| &mut t.request)
    }

    pub fn set_active_url(&mut self, url: String) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.url = url;
            tab.mark_dirty(true);
            tab.sync_spec_from_request();
        }
    }

    pub fn set_active_editor_tab(&mut self, tab: EditorTab) {
        self.set_editor_tab(tab);
    }

    pub fn set_response_tab(&mut self, tab: ResponseTab) {
        self.active_response_tab = tab;
    }

    pub fn set_response_body_mode(&mut self, mode: ResponseBodyMode) {
        self.active_body_mode = mode;
    }

    pub fn toggle_response_body_mode(&mut self) {
        self.active_body_mode = match self.active_body_mode {
            ResponseBodyMode::Pretty => ResponseBodyMode::Compress,
            ResponseBodyMode::Compress => ResponseBodyMode::Raw,
            ResponseBodyMode::Raw => ResponseBodyMode::Pretty,
        };
    }

    pub fn cycle_response_language(&mut self, current_effective: crate::services::ResponseLanguage) {
        self.response_language_override = Some(current_effective.next());
    }

    pub fn toggle_response_soft_wrap(&mut self) {
        self.response_soft_wrap = !self.response_soft_wrap;
    }

    pub fn toggle_response_tree_node(&mut self, id: &str) {
        if !self.response_tree_collapsed.remove(id) {
            self.response_tree_collapsed.insert(id.to_string());
        }
    }

    pub fn expand_all_response_tree(&mut self) {
        self.response_tree_collapsed.clear();
    }

    pub fn set_response_table_search(&mut self, query: String) {
        self.response_table_search = query;
    }

    pub fn toggle_response_table_sort(&mut self, col: usize) {
        if self.response_table_sort_col == Some(col) {
            self.response_table_sort_asc = !self.response_table_sort_asc;
        } else {
            self.response_table_sort_col = Some(col);
            self.response_table_sort_asc = true;
        }
    }

    pub fn set_response_bi_chart_type(&mut self, chart_type: crate::services::ChartType) {
        self.response_bi_chart_type = chart_type;
    }

    pub fn cycle_response_bi_agg_op(&mut self) {
        self.response_bi_agg_op = self.response_bi_agg_op.next();
    }

    pub fn cycle_response_bi_x_col(&mut self, columns: &[String]) {
        if columns.is_empty() {
            return;
        }
        let next = match &self.response_bi_x_col {
            Some(cur) => {
                let idx = columns.iter().position(|c| c == cur).unwrap_or(0);
                columns[(idx + 1) % columns.len()].clone()
            }
            None => columns[0].clone(),
        };
        self.response_bi_x_col = Some(next);
    }

    pub fn cycle_response_bi_y_col(&mut self, numeric_columns: &[String]) {
        if numeric_columns.is_empty() {
            self.response_bi_y_col = None;
            return;
        }
        let next = match &self.response_bi_y_col {
            Some(cur) => {
                let idx = numeric_columns.iter().position(|c| c == cur).unwrap_or(0);
                numeric_columns[(idx + 1) % numeric_columns.len()].clone()
            }
            None => numeric_columns[0].clone(),
        };
        self.response_bi_y_col = Some(next);
    }

    pub fn cycle_response_bi_pivot_col(&mut self, columns: &[String]) {
        if columns.is_empty() {
            return;
        }
        let next = match &self.response_bi_pivot_col {
            Some(cur) => {
                let idx = columns.iter().position(|c| c == cur).unwrap_or(0);
                columns[(idx + 1) % columns.len()].clone()
            }
            None => columns.get(1).or_else(|| columns.first()).cloned().unwrap_or_default(),
        };
        self.response_bi_pivot_col = Some(next);
    }

    pub fn cancel_active_request(&self) {
        self.execution_service.cancel();
    }

    pub fn cycle_method(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.method = match tab.method {
                HttpMethod::GET => HttpMethod::POST,
                HttpMethod::POST => HttpMethod::PUT,
                HttpMethod::PUT => HttpMethod::PATCH,
                HttpMethod::PATCH => HttpMethod::DELETE,
                HttpMethod::DELETE => HttpMethod::GET,
                _ => HttpMethod::GET,
            };
            tab.request.method = tab.method;
            tab.mark_dirty(true);
        }
    }

    pub fn set_active_method(&mut self, method: HttpMethod) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if tab.method != method {
                tab.method = method;
                tab.request.method = method;
                tab.mark_dirty(true);
            }
        }
        self.is_method_dropdown_open = false;
    }

    pub fn toggle_method_dropdown(&mut self) {
        self.is_method_dropdown_open = !self.is_method_dropdown_open;
    }

    pub fn close_method_dropdown(&mut self) {
        self.is_method_dropdown_open = false;
    }

    pub fn toggle_body_format_dropdown(&mut self) {
        self.is_body_format_dropdown_open = !self.is_body_format_dropdown_open;
    }

    pub fn close_body_format_dropdown(&mut self) {
        self.is_body_format_dropdown_open = false;
    }

    pub fn toggle_response_lang_dropdown(&mut self) {
        self.is_response_lang_dropdown_open = !self.is_response_lang_dropdown_open;
    }

    pub fn close_response_lang_dropdown(&mut self) {
        self.is_response_lang_dropdown_open = false;
    }

    pub fn set_response_language(&mut self, lang: crate::services::ResponseLanguage) {
        self.response_language_override = Some(lang);
        self.is_response_lang_dropdown_open = false;
    }

    pub fn toggle_env_dropdown(&mut self) {
        self.is_env_dropdown_open = !self.is_env_dropdown_open;
    }

    pub fn close_env_dropdown(&mut self) {
        self.is_env_dropdown_open = false;
    }

    pub fn reload_environments_from_workspace(&mut self) {
        if let Some(root) = self.workspace_store.workspace_root.as_deref() {
            self.environments = aura_sync::load_environments_from_dir(root);
            if let Some(active_id) = &self.active_environment_id {
                if !self.environments.iter().any(|(_, e)| &e.id == active_id) {
                    self.active_environment_id = None;
                }
            }
        } else {
            self.environments.clear();
            self.active_environment_id = None;
        }
    }

    pub fn active_environment(&self) -> Option<&aura_core::model::EnvironmentProfile> {
        let id = self.active_environment_id.as_deref()?;
        self.environments
            .iter()
            .find(|(_, e)| e.id == id)
            .map(|(_, e)| e)
    }

    pub fn active_environment_vars(&self) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        if let Some(env) = self.active_environment() {
            for v in &env.variables {
                if v.enabled && !v.key.trim().is_empty() {
                    map.insert(v.key.clone(), v.value.clone());
                }
            }
        }
        map
    }

    /// Builds the unified template resolution context for UI previews, inputs,
    /// code editors, and request execution.
    pub fn active_template_context(&self) -> aura_core::templating::TemplateContext {
        let mut context = aura_core::templating::TemplateContext::default();
        if let Some(tab) = self.active_tab() {
            context.request_vars = tab.request.variables.clone();
            for (raw_key, value, enabled) in &tab.variable_rows {
                let key = raw_key.trim();
                if key.is_empty() {
                    continue;
                }
                if *enabled {
                    context
                        .request_vars
                        .insert(key.to_string(), value.clone());
                } else {
                    context.request_vars.remove(key);
                }
            }
        }
        context.environment_vars = self.active_environment_vars();
        for collection in &self.collections {
            for (k, v) in &collection.variables {
                context
                    .collection_vars
                    .entry(k.clone())
                    .or_insert_with(|| v.clone());
            }
        }
        context
    }

    pub fn set_active_environment(&mut self, id: Option<String>) {
        self.active_environment_id = id;
        self.is_env_dropdown_open = false;
    }

    pub fn open_new_environment_editor(&mut self) {
        self.is_env_dropdown_open = false;
        let profile = aura_core::model::EnvironmentProfile {
            id: uuid::Uuid::new_v4().to_string(),
            name: "New Environment".to_string(),
            variables: vec![aura_core::model::VariableItem {
                key: String::new(),
                value: String::new(),
                enabled: true,
                is_secret: false,
            }],
        };
        let code_buffer = aura_sync::environment_to_toml(&profile).unwrap_or_default();
        self.environment_editor = Some(EnvironmentEditorState {
            original_path: None,
            profile,
            mode: EnvironmentEditorMode::Ui,
            code_buffer,
            code_error: None,
        });
    }

    pub fn open_environment_editor(&mut self, id: &str) {
        self.is_env_dropdown_open = false;
        if let Some((path, env)) = self.environments.iter().find(|(_, e)| e.id == id) {
            let code_buffer = aura_sync::environment_to_toml(env).unwrap_or_default();
            self.environment_editor = Some(EnvironmentEditorState {
                original_path: Some(path.clone()),
                profile: env.clone(),
                mode: EnvironmentEditorMode::Ui,
                code_buffer,
                code_error: None,
            });
        }
    }

    pub fn set_environment_editor_mode(&mut self, mode: EnvironmentEditorMode) {
        let Some(ref mut ed) = self.environment_editor else {
            return;
        };
        if ed.mode == mode {
            return;
        }
        match mode {
            EnvironmentEditorMode::Code => {
                ed.code_buffer = aura_sync::environment_to_toml(&ed.profile).unwrap_or_default();
                ed.code_error = None;
                ed.mode = EnvironmentEditorMode::Code;
            }
            EnvironmentEditorMode::Ui => {
                if ed.code_error.is_none() {
                    ed.mode = EnvironmentEditorMode::Ui;
                }
            }
        }
    }

    pub fn update_environment_code_buffer(&mut self, text: String) {
        let Some(ref mut ed) = self.environment_editor else {
            return;
        };
        ed.code_buffer = text.clone();
        match aura_sync::toml_to_environment(&text, Some(&ed.profile.id)) {
            Ok(parsed) => {
                ed.profile = parsed;
                ed.code_error = None;
            }
            Err(err) => {
                ed.code_error = Some(err.to_string());
            }
        }
    }

    pub fn format_environment_code_buffer(&mut self) {
        let Some(ref mut ed) = self.environment_editor else {
            return;
        };
        if ed.code_error.is_none() {
            if let Ok(formatted) = aura_sync::environment_to_toml(&ed.profile) {
                ed.code_buffer = formatted;
            }
        }
    }

    pub fn close_environment_editor(&mut self) {
        self.environment_editor = None;
    }

    pub fn save_environment_editor(&mut self) -> anyhow::Result<()> {
        if let Some(ref ed) = self.environment_editor {
            if ed.mode == EnvironmentEditorMode::Code {
                if let Some(ref err) = ed.code_error {
                    anyhow::bail!("Fix TOML syntax errors before saving: {err}");
                }
            }
        }
        let Some(editor) = self.environment_editor.take() else {
            return Ok(());
        };
        let Some(root) = self.workspace_store.workspace_root.clone() else {
            self.status_message = Some("Open a workspace folder first to save environments".to_string());
            return Ok(());
        };
        let mut profile = editor.profile;
        if profile.name.trim().is_empty() {
            profile.name = "environment".to_string();
        }
        profile
            .variables
            .retain(|v| !v.key.trim().is_empty() || !v.value.trim().is_empty());
        let saved_path = aura_sync::save_environment_to_dir(
            &root,
            &profile,
            editor.original_path.as_deref(),
        )?;
        self.reload_environments_from_workspace();
        // Keep active_environment_id pointing to the saved profile if it was active or newly created
        if let Some((_, matched)) = self.environments.iter().find(|(p, _)| p == &saved_path) {
            if self.active_environment_id.as_deref() == Some(&profile.id)
                || self.active_environment_id.is_none()
            {
                self.active_environment_id = Some(matched.id.clone());
            }
        }
        self.status_message = Some(format!("Saved environment '{}'", profile.name));
        Ok(())
    }

    pub fn duplicate_environment(&mut self, id: &str) -> anyhow::Result<()> {
        let Some(root) = self.workspace_store.workspace_root.clone() else {
            return Ok(());
        };
        let Some((_, src)) = self.environments.iter().find(|(_, e)| e.id == id).cloned() else {
            return Ok(());
        };
        let mut dup = src.clone();
        dup.id = uuid::Uuid::new_v4().to_string();
        dup.name = format!("{} Copy", src.name);
        aura_sync::save_environment_to_dir(&root, &dup, None)?;
        self.reload_environments_from_workspace();
        self.status_message = Some(format!("Duplicated environment '{}'", dup.name));
        Ok(())
    }

    pub fn copy_environment(&mut self, id: &str) {
        if let Some((_, src)) = self.environments.iter().find(|(_, e)| e.id == id) {
            self.clipboard_environment = Some(src.clone());
            self.status_message = Some(format!("Copied environment '{}'", src.name));
        }
    }

    pub fn paste_environment(&mut self) -> anyhow::Result<()> {
        let Some(root) = self.workspace_store.workspace_root.clone() else {
            return Ok(());
        };
        let Some(src) = self.clipboard_environment.clone() else {
            return Ok(());
        };
        let mut pasted = src;
        pasted.id = uuid::Uuid::new_v4().to_string();
        if self
            .environments
            .iter()
            .any(|(_, e)| e.name.eq_ignore_ascii_case(&pasted.name))
        {
            pasted.name = format!("{} Copy", pasted.name);
        }
        aura_sync::save_environment_to_dir(&root, &pasted, None)?;
        self.reload_environments_from_workspace();
        self.status_message = Some(format!("Pasted environment '{}'", pasted.name));
        Ok(())
    }

    pub fn delete_environment(&mut self, id: &str) -> anyhow::Result<()> {
        if let Some((path, env)) = self.environments.iter().find(|(_, e)| e.id == id).cloned() {
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            if self.active_environment_id.as_deref() == Some(id) {
                self.active_environment_id = None;
            }
            self.reload_environments_from_workspace();
            self.status_message = Some(format!("Deleted environment '{}'", env.name));
        }
        Ok(())
    }

    // =========================================================================
    // Active Inline Input & Navigation Operations
    // =========================================================================

    pub fn set_script_sub_tab(&mut self, tab: ScriptSubTab) {
        self.active_script_sub_tab = tab;
    }

    pub fn toggle_script_reference(&mut self) {
        self.show_script_reference = !self.show_script_reference;
    }

    pub fn set_active_input(&mut self, target: Option<ActiveInputTarget>) {
        self.active_input_target = target;
    }

    pub fn blur_active_input(&mut self) {
        self.active_input_target = None;
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use aura_core::model::AuraResponse;
    use std::collections::HashMap;

    #[test]
    fn test_method_dropdown_and_selection() {
        let mut state = AuraAppState::new();
        state.new_request();
        assert!(!state.is_method_dropdown_open);
        state.toggle_method_dropdown();
        assert!(state.is_method_dropdown_open);
        state.set_active_method(HttpMethod::POST);
        assert!(!state.is_method_dropdown_open);
        assert_eq!(state.tabs[0].method, HttpMethod::POST);
        assert_eq!(state.tabs[0].request.method, HttpMethod::POST);
        assert!(state.tabs[0].is_dirty);
    }

    #[test]
    fn test_settings_validation_rejects_invalid_and_accepts_valid() {
        let mut state = AuraAppState::new();
        state.new_request();
        assert_eq!(state.tabs[0].request.settings.timeout_ms, 0);

        assert!(!state.update_setting_timeout("abc"));
        assert!(state.setting_timeout_error.is_some());
        assert_eq!(state.tabs[0].request.settings.timeout_ms, 0);

        assert!(!state.update_setting_timeout("50"));
        assert!(state.setting_timeout_error.is_some());
        assert_eq!(state.tabs[0].request.settings.timeout_ms, 0);

        assert!(state.update_setting_timeout("5000"));
        assert!(state.setting_timeout_error.is_none());
        assert_eq!(state.tabs[0].request.settings.timeout_ms, 5000);

        assert!(state.update_setting_timeout("0"));
        assert!(state.setting_timeout_error.is_none());
        assert_eq!(state.tabs[0].request.settings.timeout_ms, 0);

        assert!(!state.update_setting_max_redirects("99"));
        assert!(state.setting_max_redirects_error.is_some());
        assert_eq!(state.tabs[0].request.settings.max_redirects, 10);

        assert!(state.update_setting_max_redirects("5"));
        assert!(state.setting_max_redirects_error.is_none());
        assert_eq!(state.tabs[0].request.settings.max_redirects, 5);
    }

    #[test]
    fn test_variables_editing_syncs_to_request_map() {
        let mut state = AuraAppState::new();
        state.new_request();
        state.add_variable_row();
        assert_eq!(state.tabs[0].variable_rows.len(), 1);

        state.set_input_text_for(ActiveInputTarget::VariableKey(0), "base_url".to_string());
        state.set_input_text_for(
            ActiveInputTarget::VariableValue(0),
            "https://example.com".to_string(),
        );
        assert_eq!(
            state.tabs[0].request.variables.get("base_url").map(String::as_str),
            Some("https://example.com")
        );

        state.remove_variable_row(0);
        assert!(state.tabs[0].variable_rows.is_empty());
        assert!(state.tabs[0].request.variables.is_empty());
    }

    #[test]
    fn test_toggle_variable_row_moves_between_enabled_and_disabled() {
        let mut state = AuraAppState::new();
        state.new_request();
        state.add_variable_row();
        state.set_input_text_for(ActiveInputTarget::VariableKey(0), "api_key".to_string());
        state.set_input_text_for(ActiveInputTarget::VariableValue(0), "secret".to_string());

        assert_eq!(
            state.tabs[0].request.variables.get("api_key").map(String::as_str),
            Some("secret")
        );
        assert!(state.tabs[0].request.disabled_variables.is_empty());

        state.toggle_variable_row(0);
        assert!(!state.tabs[0].variable_rows[0].2);
        assert!(state.tabs[0].request.variables.is_empty());
        assert_eq!(
            state.tabs[0]
                .request
                .disabled_variables
                .get("api_key")
                .map(String::as_str),
            Some("secret")
        );

        let spec = aura_parser::request_to_spec(&state.tabs[0].request);
        assert!(spec.contains("api_key~ = \"secret\""));

        state.toggle_variable_row(0);
        assert!(state.tabs[0].variable_rows[0].2);
        assert_eq!(
            state.tabs[0].request.variables.get("api_key").map(String::as_str),
            Some("secret")
        );
        assert!(state.tabs[0].request.disabled_variables.is_empty());
    }

    #[test]
    fn test_run_active_tests_uses_duration_and_variables() {
        let mut state = AuraAppState::new();
        state.new_request();
        state.tabs[0]
            .request
            .variables
            .insert("expected_status".to_string(), "200".to_string());
        state.tabs[0].request.scripts.tests = r#"
            describe('Response', () => {
                it('status matches variable', () => {
                    expect(response.status).toBe(Number(aura.variables.get('expected_status')));
                });
                it('duration_ms is available', () => {
                    expect(response.duration_ms).toBeLessThan(500);
                });
            });
        "#
        .to_string();
        state.tabs[0].response = Some(AuraResponse {
            status: 200,
            status_text: "OK".to_string(),
            duration_ms: 42,
            size_bytes: 2,
            headers: HashMap::new(),
            body: "{}".to_string(),
            timestamp: chrono::Utc::now(),
            assertions: Vec::new(),
            console_logs: Vec::new(),
            script_errors: Vec::new(),
        });

        let count = state.run_active_tests().expect("tests should run");
        assert_eq!(count, 2);
        let assertions = &state.tabs[0].response.as_ref().unwrap().assertions;
        assert!(assertions.iter().all(|a| a.passed));
        assert_eq!(state.active_response_tab, crate::stores::ResponseTab::Tests);
    }

    #[test]
    fn test_environment_editor_ui_and_code_mode_sync() {
        let mut state = AuraAppState::new();
        state.open_new_environment_editor();

        {
            let ed = state.environment_editor.as_mut().unwrap();
            assert_eq!(ed.mode, EnvironmentEditorMode::Ui);
            ed.profile.name = "Staging".to_string();
            ed.profile.variables = vec![
                aura_core::model::VariableItem {
                    key: "API_URL".to_string(),
                    value: "https://staging.example.com".to_string(),
                    enabled: true,
                    is_secret: false,
                },
                aura_core::model::VariableItem {
                    key: "DEBUG".to_string(),
                    value: "1".to_string(),
                    enabled: false,
                    is_secret: false,
                },
            ];
        }

        // Switch UI -> Code: serializes current profile to TOML
        state.set_environment_editor_mode(EnvironmentEditorMode::Code);
        {
            let ed = state.environment_editor.as_ref().unwrap();
            assert_eq!(ed.mode, EnvironmentEditorMode::Code);
            assert!(ed.code_error.is_none());
            assert!(ed.code_buffer.contains("name = \"Staging\""));
            assert!(ed.code_buffer.contains("API_URL = \"https://staging.example.com\""));
            assert!(ed.code_buffer.contains("\"DEBUG~\" = \"1\""));
        }

        // Edit in Code mode with valid TOML -> updates profile
        state.update_environment_code_buffer(
            "name = \"Production\"\n\n[variables]\nAPI_URL = \"https://api.example.com\"\n"
                .to_string(),
        );
        {
            let ed = state.environment_editor.as_ref().unwrap();
            assert!(ed.code_error.is_none());
            assert_eq!(ed.profile.name, "Production");
            assert_eq!(ed.profile.variables.len(), 1);
            assert_eq!(ed.profile.variables[0].key, "API_URL");
            assert_eq!(ed.profile.variables[0].value, "https://api.example.com");
        }

        // Edit with invalid TOML -> records error and blocks switching back or saving
        state.update_environment_code_buffer("name = [invalid".to_string());
        assert!(state
            .environment_editor
            .as_ref()
            .unwrap()
            .code_error
            .is_some());
        state.set_environment_editor_mode(EnvironmentEditorMode::Ui);
        assert_eq!(
            state.environment_editor.as_ref().unwrap().mode,
            EnvironmentEditorMode::Code
        );
        assert!(state.save_environment_editor().is_err());
        assert!(state.environment_editor.is_some());

        // Fix TOML and switch back to UI
        state.update_environment_code_buffer(
            "name = \"Production\"\n\n[variables]\nTOKEN = \"xyz\"\n".to_string(),
        );
        state.set_environment_editor_mode(EnvironmentEditorMode::Ui);
        let ed = state.environment_editor.as_ref().unwrap();
        assert_eq!(ed.mode, EnvironmentEditorMode::Ui);
        assert!(ed.code_error.is_none());
        assert_eq!(ed.profile.variables[0].key, "TOKEN");
    }
}
