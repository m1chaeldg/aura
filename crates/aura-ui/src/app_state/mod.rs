//! Main application state for UI coordinating Domain Stores and Services.

use crate::keybindings::KeybindingPreset;
use crate::preferences::UpdateCheckFrequency;
use crate::services::ReleaseInfo;
use crate::stores::{
    EditorTab, LayoutOrientation, ResponseBodyMode, ResponseTab, WorkspaceStore, WorkspaceTab,
};
use aura_core::model::{AuraRequest, Collection, HttpMethod};
use gpui::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod ai_actions;
mod editor_dialog;
mod inline_input_edit;
mod inline_input_nav;
mod item_dialogs;
mod layout_and_palette;
mod network_actions;
mod protocols;
mod request_editing;
mod security_actions;
mod workspace_tabs;

pub use protocols::WsdlImportDialogState;

/// Sub-tabs within the Scripts panel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ScriptSubTab {
    #[default]
    PreRequest,
    PostResponse,
}

/// UI tab inside Load Tester results section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadResultsViewTab {
    #[default]
    TimeSeriesCharts,
    AggregateTable,
    RawStream,
}

impl LoadResultsViewTab {
    pub const ALL: [Self; 3] = [
        Self::TimeSeriesCharts,
        Self::AggregateTable,
        Self::RawStream,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::TimeSeriesCharts => "Live Charts",
            Self::AggregateTable => "Aggregate Report",
            Self::RawStream => "Raw Samples",
        }
    }
}

/// Time-series metric plotted in the Live Charts tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadChartMetric {
    #[default]
    Rps,
    ActiveVus,
    LatencyPercentiles,
    ErrorRate,
}

impl LoadChartMetric {
    pub const ALL: [Self; 4] = [
        Self::Rps,
        Self::ActiveVus,
        Self::LatencyPercentiles,
        Self::ErrorRate,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Rps => "RPS",
            Self::ActiveVus => "Active VUs",
            Self::LatencyPercentiles => "Latency (p50/p95/p99)",
            Self::ErrorRate => "Error Rate %",
        }
    }
}

/// UI selector for JMeter-style jittered Think-Time mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadThinkMode {
    #[default]
    None,
    Uniform,
    Gaussian,
}

impl LoadThinkMode {
    pub const ALL: [Self; 3] = [Self::None, Self::Uniform, Self::Gaussian];

    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "No Think-Time",
            Self::Uniform => "Uniform (min..max ms)",
            Self::Gaussian => "Gaussian (mean ± std)",
        }
    }
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
    AuthSchemeField(usize),
    SettingCertField(usize),
    SettingProxyField(usize),
    BodyText,
    PreRequestScript,
    PostResponseScript,
    TestScript,
    AssertExpr(usize),
    AssertExpected(usize),
    ExtractorVar(usize),
    ExtractorExpr(usize),
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

/// Top-bar dropdown menu categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopMenuKind {
    File,
    Workspace,
    View,
    Tools,
    Ai,
    Help,
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
    pub is_protocol_dropdown_open: bool,
    #[serde(skip)]
    pub is_body_format_dropdown_open: bool,
    #[serde(skip)]
    pub is_response_lang_dropdown_open: bool,
    #[serde(skip)]
    pub is_env_dropdown_open: bool,
    #[serde(skip)]
    pub graphql_schemas: std::collections::HashMap<String, aura_core::graphql::GraphQLSchema>,
    #[serde(skip)]
    pub is_graphql_introspecting: bool,
    #[serde(skip)]
    pub show_graphql_schema: bool,
    #[serde(skip)]
    pub grpc_schemas: std::collections::HashMap<String, aura_core::grpc::ProtoSchema>,
    #[serde(skip)]
    pub ws_senders:
        std::collections::HashMap<String, tokio::sync::mpsc::UnboundedSender<aura_engine::WsCommand>>,
    #[serde(skip)]
    pub ws_composer_text: String,
    #[serde(skip)]
    pub ws_template_name: String,
    #[serde(skip)]
    pub soap_operation: String,
    #[serde(skip)]
    pub soap_namespace: String,
    #[serde(skip)]
    pub soap_action: String,
    #[serde(skip)]
    pub wsdl_import_dialog: Option<WsdlImportDialogState>,
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
    pub reference_category: Option<String>,
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
    #[serde(default)]
    pub domain_client_certs: Vec<aura_core::model::ClientCertificateConfig>,
    #[serde(default)]
    pub vault_config: aura_engine::VaultConfig,
    #[serde(default)]
    pub global_proxy: aura_core::model::ProxyConfig,
    #[serde(default)]
    pub cookie_jar: aura_core::cookies::CookieJar,
    #[serde(skip)]
    pub interceptor_manager: aura_core::interceptor::InterceptorManager,
    #[serde(skip)]
    pub capture_proxy_handle: Option<aura_engine::CaptureProxyHandle>,
    #[serde(skip)]
    pub is_oauth2_in_flight: bool,
    #[serde(skip)]
    pub is_security_vault_modal_open: bool,
    #[serde(skip)]
    pub is_cookie_modal_open: bool,
    #[serde(skip)]
    pub open_top_menu: Option<TopMenuKind>,
    #[serde(skip)]
    pub is_settings_modal_open: bool,
    #[serde(skip)]
    pub is_about_modal_open: bool,
    #[serde(skip)]
    pub is_help_modal_open: bool,
    #[serde(skip)]
    pub is_update_modal_open: bool,
    #[serde(default)]
    pub update_check_frequency: UpdateCheckFrequency,
    #[serde(default)]
    pub last_update_check_epoch: Option<i64>,
    #[serde(skip)]
    pub update_checking: bool,
    #[serde(skip)]
    pub update_installing: bool,
    #[serde(skip)]
    pub update_available: Option<ReleaseInfo>,
    #[serde(skip)]
    pub update_status_message: Option<String>,
    #[serde(default)]
    pub ai_prefs: crate::preferences::AiPreferences,
    #[serde(skip)]
    pub is_ai_drawer_open: bool,
    #[serde(skip)]
    pub ai_chat_history: Vec<crate::services::AiChatMessage>,
    #[serde(skip)]
    pub ai_prompt_draft: String,
    #[serde(skip)]
    pub ai_is_busy: bool,
    #[serde(skip)]
    pub ai_status_message: Option<String>,
    #[serde(skip)]
    pub ai_discovery_in_flight: bool,
    #[serde(skip)]
    pub ai_cancel_token: Option<tokio_util::sync::CancellationToken>,
    #[serde(skip)]
    pub ai_custom_cli_name: String,
    #[serde(skip)]
    pub ai_custom_cli_cmd: String,
    #[serde(skip)]
    pub ai_custom_cli_args: String,
    #[serde(skip)]
    pub ai_http_name: String,
    #[serde(skip)]
    pub ai_http_url: String,
    #[serde(skip)]
    pub ai_http_model: String,
    #[serde(skip)]
    pub ai_http_api_key: String,
    #[serde(skip)]
    pub settings_tab_ai: bool,
    #[serde(skip)]
    pub cookie_domain_filter: String,
    #[serde(skip)]
    pub draft_cookie_domain: String,
    #[serde(skip)]
    pub draft_cookie_path: String,
    #[serde(skip)]
    pub draft_cookie_name: String,
    #[serde(skip)]
    pub draft_cookie_value: String,
    #[serde(skip)]
    pub draft_cookie_secure: bool,
    #[serde(skip)]
    pub is_proxy_modal_open: bool,
    #[serde(skip)]
    pub is_interceptor_modal_open: bool,
    #[serde(skip)]
    pub interceptor_bind_addr: String,
    #[serde(skip)]
    pub draft_rule_match: String,
    #[serde(skip)]
    pub draft_rule_target: String,
    #[serde(skip)]
    pub is_codegen_modal_open: bool,
    #[serde(skip, default = "default_codegen_language")]
    pub codegen_language: aura_core::codegen::CodegenLanguage,
    #[serde(skip)]
    pub codegen_resolve_vars: bool,
    #[serde(skip)]
    pub keychain_service: String,
    #[serde(skip)]
    pub keychain_key: String,
    #[serde(skip)]
    pub keychain_value: String,
    #[serde(skip)]
    pub draft_cert_domain: String,
    #[serde(skip)]
    pub draft_cert_path: String,
    #[serde(skip)]
    pub draft_cert_key_path: String,
    #[serde(skip)]
    pub draft_cert_passphrase: String,
    #[serde(skip)]
    pub is_git_modal_open: bool,
    #[serde(skip)]
    pub git_snapshot: Option<aura_sync::GitStatusSnapshot>,
    #[serde(skip)]
    pub git_selected_file: Option<(String, bool)>,
    #[serde(skip)]
    pub git_selected_diff: String,
    #[serde(skip)]
    pub git_commit_message: String,
    #[serde(skip)]
    pub git_new_branch_name: String,
    #[serde(skip)]
    pub git_is_busy: bool,
    #[serde(skip)]
    pub git_last_output: Option<String>,
    #[serde(skip)]
    pub git_error: Option<String>,
    #[serde(skip)]
    pub is_load_tester_modal_open: bool,
    #[serde(skip)]
    pub load_profile_mode: aura_engine::LoadProfileMode,
    #[serde(skip)]
    pub load_use_collection: bool,
    #[serde(skip)]
    pub load_iterations_str: String,
    #[serde(skip)]
    pub load_concurrency_str: String,
    #[serde(skip)]
    pub load_delay_ms_str: String,
    #[serde(skip)]
    pub load_ramp_up_str: String,
    #[serde(skip)]
    pub load_ramp_down_str: String,
    #[serde(skip)]
    pub load_duration_str: String,
    #[serde(skip)]
    pub load_target_rps_enabled: bool,
    #[serde(skip)]
    pub load_target_rps_str: String,
    #[serde(skip)]
    pub load_stages: Vec<(String, String)>,
    #[serde(skip)]
    pub load_is_running: bool,
    #[serde(skip)]
    pub load_cancel: Option<tokio_util::sync::CancellationToken>,
    #[serde(skip)]
    pub load_live_records: Vec<aura_engine::RunnerExecutionRecord>,
    #[serde(skip)]
    pub load_report: Option<aura_engine::RunnerReport>,
    #[serde(skip)]
    pub load_error: Option<String>,
    #[serde(skip)]
    pub load_think_mode: LoadThinkMode,
    #[serde(skip)]
    pub load_think_a_str: String,
    #[serde(skip)]
    pub load_think_b_str: String,
    #[serde(skip)]
    pub load_barrier_enabled: bool,
    #[serde(skip)]
    pub load_barrier_size_str: String,
    #[serde(skip)]
    pub load_barrier_timeout_str: String,
    #[serde(skip)]
    pub load_weighted_routing: bool,
    #[serde(skip)]
    pub load_data_enabled: bool,
    #[serde(skip)]
    pub load_data_file_path: String,
    #[serde(skip)]
    pub load_data_sharing: aura_engine::DataSharingMode,
    #[serde(skip)]
    pub load_data_eof: aura_engine::DataEofBehavior,
    #[serde(skip)]
    pub load_max_duration_ms_str: String,
    #[serde(skip)]
    pub load_min_size_bytes_str: String,
    #[serde(skip)]
    pub load_max_size_bytes_str: String,
    #[serde(skip)]
    pub load_sla_p95_str: String,
    #[serde(skip)]
    pub load_sla_p99_str: String,
    #[serde(skip)]
    pub load_sla_err_str: String,
    #[serde(skip)]
    pub load_results_tab: LoadResultsViewTab,
    #[serde(skip)]
    pub load_chart_metric: LoadChartMetric,
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

fn default_codegen_language() -> aura_core::codegen::CodegenLanguage {
    aura_core::codegen::CodegenLanguage::Curl
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
            is_protocol_dropdown_open: false,
            is_body_format_dropdown_open: false,
            is_response_lang_dropdown_open: false,
            is_env_dropdown_open: false,
            graphql_schemas: std::collections::HashMap::new(),
            is_graphql_introspecting: false,
            show_graphql_schema: true,
            grpc_schemas: std::collections::HashMap::new(),
            ws_senders: std::collections::HashMap::new(),
            ws_composer_text: String::new(),
            ws_template_name: String::new(),
            soap_operation: "Operation".to_string(),
            soap_namespace: "http://tempuri.org/".to_string(),
            soap_action: String::new(),
            wsdl_import_dialog: None,
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
            reference_category: None,
            setting_timeout_draft: None,
            setting_timeout_error: None,
            setting_max_redirects_draft: None,
            setting_max_redirects_error: None,
            auto_save_enabled: true,
            auto_save_delay_ms: 1000,
            domain_client_certs: Vec::new(),
            vault_config: aura_engine::VaultConfig::default(),
            global_proxy: aura_core::model::ProxyConfig::default(),
            cookie_jar: aura_core::cookies::CookieJar::default(),
            interceptor_manager: aura_core::interceptor::InterceptorManager::default(),
            capture_proxy_handle: None,
            is_oauth2_in_flight: false,
            is_security_vault_modal_open: false,
            is_cookie_modal_open: false,
            open_top_menu: None,
            is_settings_modal_open: false,
            is_about_modal_open: false,
            is_help_modal_open: false,
            is_update_modal_open: false,
            update_check_frequency: UpdateCheckFrequency::default(),
            last_update_check_epoch: None,
            update_checking: false,
            update_installing: false,
            update_available: None,
            update_status_message: None,
            ai_prefs: crate::preferences::AiPreferences::default(),
            is_ai_drawer_open: false,
            ai_chat_history: Vec::new(),
            ai_prompt_draft: String::new(),
            ai_is_busy: false,
            ai_status_message: None,
            ai_discovery_in_flight: false,
            ai_cancel_token: None,
            ai_custom_cli_name: String::new(),
            ai_custom_cli_cmd: String::new(),
            ai_custom_cli_args: "-p {prompt}".to_string(),
            ai_http_name: String::new(),
            ai_http_url: "https://api.openai.com/v1".to_string(),
            ai_http_model: "gpt-4o".to_string(),
            ai_http_api_key: "$OPENAI_API_KEY".to_string(),
            settings_tab_ai: false,
            cookie_domain_filter: String::new(),
            draft_cookie_domain: String::new(),
            draft_cookie_path: "/".to_string(),
            draft_cookie_name: String::new(),
            draft_cookie_value: String::new(),
            draft_cookie_secure: false,
            is_proxy_modal_open: false,
            is_interceptor_modal_open: false,
            interceptor_bind_addr: "127.0.0.1:8888".to_string(),
            draft_rule_match: String::new(),
            draft_rule_target: String::new(),
            is_codegen_modal_open: false,
            codegen_language: default_codegen_language(),
            codegen_resolve_vars: true,
            keychain_service: "aura".to_string(),
            keychain_key: String::new(),
            keychain_value: String::new(),
            draft_cert_domain: String::new(),
            draft_cert_path: String::new(),
            draft_cert_key_path: String::new(),
            draft_cert_passphrase: String::new(),
            is_git_modal_open: false,
            git_snapshot: None,
            git_selected_file: None,
            git_selected_diff: String::new(),
            git_commit_message: String::new(),
            git_new_branch_name: String::new(),
            git_is_busy: false,
            git_last_output: None,
            git_error: None,
            is_load_tester_modal_open: false,
            load_profile_mode: aura_engine::LoadProfileMode::FixedIterations,
            load_use_collection: false,
            load_iterations_str: "20".to_string(),
            load_concurrency_str: "4".to_string(),
            load_delay_ms_str: "0".to_string(),
            load_ramp_up_str: "5".to_string(),
            load_ramp_down_str: "5".to_string(),
            load_duration_str: "20".to_string(),
            load_target_rps_enabled: false,
            load_target_rps_str: "50".to_string(),
            load_stages: vec![
                ("5".to_string(), "5".to_string()),
                ("25".to_string(), "10".to_string()),
                ("5".to_string(), "5".to_string()),
            ],
            load_is_running: false,
            load_cancel: None,
            load_live_records: Vec::new(),
            load_report: None,
            load_error: None,
            load_think_mode: LoadThinkMode::None,
            load_think_a_str: "200".to_string(),
            load_think_b_str: "600".to_string(),
            load_barrier_enabled: false,
            load_barrier_size_str: "4".to_string(),
            load_barrier_timeout_str: "5000".to_string(),
            load_weighted_routing: false,
            load_data_enabled: false,
            load_data_file_path: String::new(),
            load_data_sharing: aura_engine::DataSharingMode::AllVUs,
            load_data_eof: aura_engine::DataEofBehavior::Recycle,
            load_max_duration_ms_str: String::new(),
            load_min_size_bytes_str: String::new(),
            load_max_size_bytes_str: String::new(),
            load_sla_p95_str: String::new(),
            load_sla_p99_str: String::new(),
            load_sla_err_str: String::new(),
            load_results_tab: LoadResultsViewTab::TimeSeriesCharts,
            load_chart_metric: LoadChartMetric::Rps,
            workspace_service: default_workspace_service(),
            execution_service: default_execution_service(),
        }
    }
}

impl AuraAppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn git_workspace_dir(&self) -> PathBuf {
        self.workspace_store
            .workspace_root
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    pub fn build_load_runner_config(&self) -> aura_engine::RunnerConfig {
        use aura_core::model::ThinkTimeConfig;
        use aura_engine::{LoadProfileMode, LoadStage, RunnerConfig, SynchronizingTimerConfig};

        let total_iterations = self
            .load_iterations_str
            .trim()
            .parse::<usize>()
            .unwrap_or(10)
            .max(1);
        let concurrency = self
            .load_concurrency_str
            .trim()
            .parse::<usize>()
            .unwrap_or(2)
            .max(1);
        let delay_ms = self.load_delay_ms_str.trim().parse::<u64>().unwrap_or(0);
        let target_rps = if self.load_target_rps_enabled {
            self.load_target_rps_str
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|v| *v > 0.0)
        } else {
            None
        };
        let think_time = match self.load_think_mode {
            LoadThinkMode::None => ThinkTimeConfig::None,
            LoadThinkMode::Uniform => ThinkTimeConfig::Uniform {
                min_ms: self.load_think_a_str.trim().parse::<u64>().unwrap_or(0),
                max_ms: self.load_think_b_str.trim().parse::<u64>().unwrap_or(0),
            },
            LoadThinkMode::Gaussian => ThinkTimeConfig::Gaussian {
                mean_ms: self.load_think_a_str.trim().parse::<f64>().unwrap_or(0.0).max(0.0),
                stddev_ms: self.load_think_b_str.trim().parse::<f64>().unwrap_or(0.0).max(0.0),
            },
        };
        let sync_barrier = if self.load_barrier_enabled {
            Some(SynchronizingTimerConfig {
                group_size: self
                    .load_barrier_size_str
                    .trim()
                    .parse::<usize>()
                    .unwrap_or(2)
                    .max(2),
                timeout_ms: self
                    .load_barrier_timeout_str
                    .trim()
                    .parse::<u64>()
                    .unwrap_or(5000)
                    .max(50),
            })
        } else {
            None
        };
        let weighted_routing = self.load_weighted_routing;
        let data_feeder = if self.load_data_enabled && !self.load_data_file_path.trim().is_empty() {
            Some(aura_engine::DataFeederConfig {
                enabled: true,
                file_path: self.load_data_file_path.trim().to_string(),
                sharing_mode: self.load_data_sharing,
                eof_behavior: self.load_data_eof,
                ..Default::default()
            })
        } else {
            None
        };
        let sample_assertions = aura_engine::PerformanceAssertionConfig {
            max_duration_ms: self.load_max_duration_ms_str.trim().parse::<u64>().ok().filter(|&v| v > 0),
            min_size_bytes: self.load_min_size_bytes_str.trim().parse::<usize>().ok().filter(|&v| v > 0),
            max_size_bytes: self.load_max_size_bytes_str.trim().parse::<usize>().ok().filter(|&v| v > 0),
        };
        let max_p95_ms = self.load_sla_p95_str.trim().parse::<u64>().ok().filter(|&v| v > 0);
        let max_p99_ms = self.load_sla_p99_str.trim().parse::<u64>().ok().filter(|&v| v > 0);
        let max_error_rate_percent = self.load_sla_err_str.trim().parse::<f64>().ok().filter(|&v| v >= 0.0);
        let sla_thresholds = if max_p95_ms.is_some() || max_p99_ms.is_some() || max_error_rate_percent.is_some() {
            Some(aura_engine::SlaThresholds {
                max_p95_ms,
                max_p99_ms,
                max_error_rate_percent,
            })
        } else {
            None
        };

        match self.load_profile_mode {
            LoadProfileMode::FixedIterations => RunnerConfig {
                total_iterations,
                concurrency,
                delay_ms,
                ramp_up_secs: self
                    .load_ramp_up_str
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(0.0)
                    .max(0.0),
                ramp_down_secs: 0.0,
                duration_secs: None,
                stages: Vec::new(),
                target_rps,
                think_time,
                sync_barrier,
                weighted_routing,
                data_feeder,
                sample_assertions,
                sla_thresholds,
            },
            LoadProfileMode::Duration => RunnerConfig {
                total_iterations,
                concurrency,
                delay_ms,
                ramp_up_secs: 0.0,
                ramp_down_secs: 0.0,
                duration_secs: Some(
                    self.load_duration_str
                        .trim()
                        .parse::<f64>()
                        .unwrap_or(15.0)
                        .max(0.5),
                ),
                stages: Vec::new(),
                target_rps,
                think_time,
                sync_barrier,
                weighted_routing,
                data_feeder,
                sample_assertions,
                sla_thresholds,
            },
            LoadProfileMode::RampUpDown => RunnerConfig {
                total_iterations,
                concurrency,
                delay_ms,
                ramp_up_secs: self
                    .load_ramp_up_str
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(5.0)
                    .max(0.0),
                ramp_down_secs: self
                    .load_ramp_down_str
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(5.0)
                    .max(0.0),
                duration_secs: Some(
                    self.load_duration_str
                        .trim()
                        .parse::<f64>()
                        .unwrap_or(20.0)
                        .max(1.0),
                ),
                stages: Vec::new(),
                target_rps,
                think_time,
                sync_barrier,
                weighted_routing,
                data_feeder,
                sample_assertions,
                sla_thresholds,
            },
            LoadProfileMode::SteppedStages => {
                let stages = self
                    .load_stages
                    .iter()
                    .map(|(vus, secs)| {
                        LoadStage::new(
                            vus.trim().parse::<usize>().unwrap_or(1).max(1),
                            secs.trim().parse::<f64>().unwrap_or(5.0).max(0.1),
                        )
                    })
                    .collect();
                RunnerConfig {
                    total_iterations,
                    concurrency,
                    delay_ms,
                    ramp_up_secs: 0.0,
                    ramp_down_secs: 0.0,
                    duration_secs: None,
                    stages,
                    target_rps,
                    think_time,
                    sync_barrier,
                    weighted_routing,
                    data_feeder,
                    sample_assertions,
                    sla_thresholds,
                }
            }
        }
    }

    pub fn toggle_active_flow_once_per_run(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let flow = tab
                .request
                .flow
                .get_or_insert_with(aura_core::model::FlowControl::default);
            flow.once_per_run = !flow.once_per_run;
            tab.mark_dirty(true);
            tab.sync_spec_from_request();
        }
    }

    pub fn set_active_flow_weight_str(&mut self, text: &str) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let val = text.trim().parse::<u8>().ok().map(|v| v.clamp(1, 100));
            let flow = tab
                .request
                .flow
                .get_or_insert_with(aura_core::model::FlowControl::default);
            flow.weight_percent = val;
            tab.mark_dirty(true);
            tab.sync_spec_from_request();
        }
    }

    pub fn set_active_flow_loop_count_str(&mut self, text: &str) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let val = text.trim().parse::<u32>().ok().filter(|&v| v > 1);
            let flow = tab
                .request
                .flow
                .get_or_insert_with(aura_core::model::FlowControl::default);
            flow.loop_count = val;
            tab.mark_dirty(true);
            tab.sync_spec_from_request();
        }
    }

    pub fn set_active_flow_condition_if(&mut self, text: String) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let flow = tab
                .request
                .flow
                .get_or_insert_with(aura_core::model::FlowControl::default);
            flow.condition_if = if text.trim().is_empty() {
                None
            } else {
                Some(text)
            };
            tab.mark_dirty(true);
            tab.sync_spec_from_request();
        }
    }

    pub fn set_active_flow_loop_while(&mut self, text: String) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let flow = tab
                .request
                .flow
                .get_or_insert_with(aura_core::model::FlowControl::default);
            flow.loop_while = if text.trim().is_empty() {
                None
            } else {
                Some(text)
            };
            tab.mark_dirty(true);
            tab.sync_spec_from_request();
        }
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
            sse_events: Vec::new(),
            stream_messages: Vec::new(),
            timing: None,
            captured_cookies: Vec::new(),
            next_request: None,
            stop_chain: false,
            exported_variables: HashMap::new(),
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

    #[test]
    fn test_load_runner_config_builder_modes() {
        let mut state = AuraAppState::new();
        state.load_concurrency_str = "8".into();
        state.load_iterations_str = "40".into();
        state.load_duration_str = "30".into();
        state.load_ramp_up_str = "5".into();
        state.load_ramp_down_str = "4".into();
        state.load_target_rps_enabled = true;
        state.load_target_rps_str = "250".into();

        state.load_profile_mode = aura_engine::LoadProfileMode::RampUpDown;
        let ramp_cfg = state.build_load_runner_config();
        assert_eq!(ramp_cfg.concurrency, 8);
        assert_eq!(ramp_cfg.duration_secs, Some(30.0));
        assert_eq!(ramp_cfg.ramp_up_secs, 5.0);
        assert_eq!(ramp_cfg.ramp_down_secs, 4.0);
        assert_eq!(ramp_cfg.target_rps, Some(250.0));

        state.load_profile_mode = aura_engine::LoadProfileMode::SteppedStages;
        state.load_stages = vec![("10".into(), "30".into()), ("100".into(), "60".into())];
        let stage_cfg = state.build_load_runner_config();
        assert_eq!(stage_cfg.stages.len(), 2);
        assert_eq!(stage_cfg.stages[0].target_vus, 10);
        assert_eq!(stage_cfg.stages[1].target_vus, 100);
        assert_eq!(stage_cfg.stages[1].duration_secs, 60.0);

        state.load_think_mode = LoadThinkMode::Gaussian;
        state.load_think_a_str = "300".into();
        state.load_think_b_str = "75".into();
        state.load_barrier_enabled = true;
        state.load_barrier_size_str = "6".into();
        state.load_barrier_timeout_str = "2500".into();
        state.load_weighted_routing = true;
        let adv_cfg = state.build_load_runner_config();
        assert_eq!(
            adv_cfg.think_time,
            aura_core::model::ThinkTimeConfig::Gaussian {
                mean_ms: 300.0,
                stddev_ms: 75.0
            }
        );
        assert_eq!(
            adv_cfg.sync_barrier,
            Some(aura_engine::SynchronizingTimerConfig {
                group_size: 6,
                timeout_ms: 2500
            })
        );
        assert!(adv_cfg.weighted_routing);

        state.load_data_enabled = true;
        state.load_data_file_path = "users.csv".into();
        state.load_data_sharing = aura_engine::DataSharingMode::PerVU;
        state.load_data_eof = aura_engine::DataEofBehavior::StopVuOnEof;
        state.load_max_duration_ms_str = "200".into();
        state.load_sla_p95_str = "150".into();
        state.load_sla_err_str = "1.5".into();
        let data_cfg = state.build_load_runner_config();
        let feeder = data_cfg.data_feeder.expect("data feeder set");
        assert_eq!(feeder.file_path, "users.csv");
        assert_eq!(feeder.sharing_mode, aura_engine::DataSharingMode::PerVU);
        assert_eq!(feeder.eof_behavior, aura_engine::DataEofBehavior::StopVuOnEof);
        assert_eq!(data_cfg.sample_assertions.max_duration_ms, Some(200));
        let sla = data_cfg.sla_thresholds.expect("sla gates set");
        assert_eq!(sla.max_p95_ms, Some(150));
        assert_eq!(sla.max_error_rate_percent, Some(1.5));
    }
}
