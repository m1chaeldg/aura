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
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let name = self
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Item");
        let icon = if self.is_folder { "📁" } else { "📄" };
        gpui::div()
            .px_2()
            .py_1()
            .bg(gpui::rgb(0x27374d))
            .border_1()
            .border_color(gpui::rgb(0x0066cc))
            .rounded_sm()
            .text_xs()
            .text_color(gpui::rgb(0xffffff))
            .shadow_md()
            .flex()
            .items_center()
            .gap_1()
            .child(icon)
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
    pub is_code_mode: bool, // Dual-mode: Visual vs Code Editor (TOML)
    pub is_command_palette_open: bool,
    #[serde(skip)]
    pub is_method_dropdown_open: bool,
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
            is_code_mode: false,
            is_command_palette_open: false,
            is_method_dropdown_open: false,
            is_log_viewer_open: false,
            is_log_viewer_text_mode: false,
            is_sidebar_collapsed: false,
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
}
