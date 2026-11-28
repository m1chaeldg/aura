//! Main application state for UI coordinating Domain Stores and Services.

use crate::keybindings::KeybindingPreset;
use crate::preferences::UpdateCheckFrequency;
use crate::services::ReleaseInfo;
use crate::stores::{
    EditorTab, LayoutOrientation, ResponseBodyMode, ResponseTab, WorkspaceStore, WorkspaceTab,
};
use aura_core::model::{AuraRequest, Collection, HttpMethod};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod ai_actions;
mod editor_dialog;
mod inline_input_edit;
mod inline_input_nav;
mod item_dialogs;
mod layout_and_palette;
mod load_config;
mod network_actions;
mod protocols;
mod request_editing;
mod security_actions;
mod types;
mod workspace_tabs;

pub use protocols::WsdlImportDialogState;
pub use types::*;

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
    pub draft_vault_profile_name: String,
    #[serde(skip)]
    pub draft_aws_profile_name: String,
    #[serde(skip)]
    pub draft_keepass_name: String,
    #[serde(skip)]
    pub draft_keepass_db_path: String,
    #[serde(skip)]
    pub draft_keepass_keyfile_path: String,
    #[serde(skip)]
    pub draft_keepass_password: String,
    #[serde(skip)]
    pub revealed_env_secret_rows: std::collections::HashSet<usize>,
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
    #[serde(skip)]
    pub load_modal_tab: LoadModalTab,
    #[serde(skip)]
    pub load_target_scope: aura_sync::LoadTargetScope,
    #[serde(skip)]
    pub load_env_override: Option<String>,
    #[serde(skip)]
    pub load_profile_name_str: String,
    #[serde(skip)]
    pub load_saved_profiles: Vec<aura_sync::WorkspaceLoadProfile>,
    #[serde(skip)]
    pub load_discovered_requests: Vec<LoadRequestEntry>,
    #[serde(skip)]
    pub load_request_overrides: std::collections::HashMap<String, aura_sync::LoadProfileRequestOverride>,
    #[serde(skip)]
    pub load_scope_dropdown_open: bool,
    #[serde(skip)]
    pub load_env_dropdown_open: bool,
    #[serde(skip)]
    pub load_profile_dropdown_open: bool,
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
            draft_vault_profile_name: String::new(),
            draft_aws_profile_name: String::new(),
            draft_keepass_name: String::new(),
            draft_keepass_db_path: String::new(),
            draft_keepass_keyfile_path: String::new(),
            draft_keepass_password: String::new(),
            revealed_env_secret_rows: std::collections::HashSet::new(),
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
            load_modal_tab: LoadModalTab::SetupAndScope,
            load_target_scope: aura_sync::LoadTargetScope::ActiveRequest,
            load_env_override: None,
            load_profile_name_str: "Default Profile".to_string(),
            load_saved_profiles: Vec::new(),
            load_discovered_requests: Vec::new(),
            load_request_overrides: std::collections::HashMap::new(),
            load_scope_dropdown_open: false,
            load_env_dropdown_open: false,
            load_profile_dropdown_open: false,
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

    /// Returns `(relative_path, display_label)` pairs for all folders in the active workspace tree.
    pub fn workspace_folder_options(&self) -> Vec<(String, String)> {
        let Some(root) = &self.workspace_store.workspace_root else {
            return Vec::new();
        };
        self.workspace_store
            .all_folders()
            .into_iter()
            .map(|abs| {
                let rel = aura_sync::to_workspace_relative_string(root, &abs);
                (rel.clone(), rel)
            })
            .collect()
    }

    /// Human-readable label for the currently selected load test target scope.
    pub fn load_target_scope_label(&self) -> String {
        match &self.load_target_scope {
            aura_sync::LoadTargetScope::ActiveRequest => "Active Request".to_string(),
            aura_sync::LoadTargetScope::Workspace => {
                let count = self
                    .load_discovered_requests
                    .iter()
                    .filter(|r| r.included)
                    .count();
                format!("Entire Workspace ({count})")
            }
            aura_sync::LoadTargetScope::Folder { relative_path } => {
                let count = self
                    .load_discovered_requests
                    .iter()
                    .filter(|r| r.included)
                    .count();
                format!("Folder: {relative_path} ({count})")
            }
        }
    }

    /// Builds a `TemplateContext` for load testing, respecting `load_env_override` if set.
    pub fn template_context_for_load(&self) -> aura_core::templating::TemplateContext {
        let mut ctx = self.active_template_context();
        if let Some(ref env_name) = self.load_env_override {
            if let Some(profile) = self
                .environments
                .iter()
                .find(|e| e.1.name.eq_ignore_ascii_case(env_name))
            {
                ctx.environment_vars.clear();
                for var in &profile.1.variables {
                    if var.enabled && !var.key.trim().is_empty() {
                        ctx.environment_vars
                            .insert(var.key.trim().to_string(), var.value.clone());
                    }
                }
            }
        }
        ctx
    }

    /// Applies `load_request_overrides` onto `load_discovered_requests`.
    pub fn apply_overrides_to_discovered_requests(&mut self) {
        for entry in &mut self.load_discovered_requests {
            if let Some(ov) = self.load_request_overrides.get(&entry.relative_path) {
                entry.included = ov.included;
                let flow = entry
                    .request
                    .flow
                    .get_or_insert_with(Default::default);
                if let Some(w) = ov.weight_percent {
                    flow.weight_percent = Some(w);
                }
                if let Some(o) = ov.once_per_run {
                    flow.once_per_run = o;
                }
                if let Some(l) = ov.loop_count {
                    flow.loop_count = Some(l);
                }
                if let Some(ref cond) = ov.condition_if {
                    flow.condition_if = if cond.trim().is_empty() {
                        None
                    } else {
                        Some(cond.clone())
                    };
                }
                if let Some(ref w_cond) = ov.loop_while {
                    flow.loop_while = if w_cond.trim().is_empty() {
                        None
                    } else {
                        Some(w_cond.clone())
                    };
                }
            }
        }
    }

    /// Records an override entry for a discovered request so saving profiles preserves inline edits.
    pub fn sync_override_for_request_idx(&mut self, idx: usize) {
        let Some(entry) = self.load_discovered_requests.get(idx) else {
            return;
        };
        let flow = entry.request.flow.as_ref();
        self.load_request_overrides.insert(
            entry.relative_path.clone(),
            aura_sync::LoadProfileRequestOverride {
                relative_path: entry.relative_path.clone(),
                included: entry.included,
                weight_percent: flow.and_then(|f| f.weight_percent),
                once_per_run: flow.map(|f| f.once_per_run),
                loop_count: flow.and_then(|f| f.loop_count),
                condition_if: flow.and_then(|f| f.condition_if.clone()),
                loop_while: flow.and_then(|f| f.loop_while.clone()),
            },
        );
    }

    /// Assembles the effective list of `AuraRequest`s to execute for the current target scope.
    pub fn effective_load_requests(&self) -> Vec<AuraRequest> {
        match &self.load_target_scope {
            aura_sync::LoadTargetScope::ActiveRequest => self
                .active_tab()
                .map(|t| vec![t.request.clone()])
                .unwrap_or_default(),
            aura_sync::LoadTargetScope::Workspace
            | aura_sync::LoadTargetScope::Folder { .. } => {
                if !self.load_discovered_requests.is_empty() {
                    self.load_discovered_requests
                        .iter()
                        .filter(|e| e.included)
                        .map(|e| e.request.clone())
                        .collect()
                } else {
                    self.collections
                        .iter()
                        .flat_map(|c| c.requests.iter().cloned())
                        .collect()
                }
            }
        }
    }

    /// Serializes current Load Tester state into a `WorkspaceLoadProfile`.
    pub fn to_workspace_load_profile(&self) -> aura_sync::WorkspaceLoadProfile {
        let data_feeder_path = if self.load_data_file_path.trim().is_empty() {
            None
        } else if let Some(ref root) = self.workspace_store.workspace_root {
            let p = PathBuf::from(self.load_data_file_path.trim());
            if p.starts_with(root) {
                Some(aura_sync::to_workspace_relative_string(root, &p))
            } else {
                Some(self.load_data_file_path.trim().to_string())
            }
        } else {
            Some(self.load_data_file_path.trim().to_string())
        };

        let profile_mode = match self.load_profile_mode {
            aura_engine::LoadProfileMode::FixedIterations => "FixedIterations",
            aura_engine::LoadProfileMode::Duration => "Duration",
            aura_engine::LoadProfileMode::RampUpDown => "RampUpDown",
            aura_engine::LoadProfileMode::SteppedStages => "SteppedStages",
        }
        .to_string();

        let think_mode = match self.load_think_mode {
            LoadThinkMode::None => "None",
            LoadThinkMode::Uniform => "Uniform",
            LoadThinkMode::Gaussian => "Gaussian",
        }
        .to_string();

        let data_sharing = match self.load_data_sharing {
            aura_engine::DataSharingMode::AllVUs => "AllVUs",
            aura_engine::DataSharingMode::PerVU => "PerVU",
        }
        .to_string();

        let data_eof = match self.load_data_eof {
            aura_engine::DataEofBehavior::Recycle => "Recycle",
            aura_engine::DataEofBehavior::StopVuOnEof => "StopVuOnEof",
        }
        .to_string();

        let stages = self
            .load_stages
            .iter()
            .map(|(v, s)| {
                (
                    v.trim().parse::<usize>().unwrap_or(5).max(1),
                    s.trim().parse::<f64>().unwrap_or(5.0).max(0.1),
                )
            })
            .collect();

        let mut overrides: Vec<aura_sync::LoadProfileRequestOverride> =
            self.load_request_overrides.values().cloned().collect();
        overrides.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

        aura_sync::WorkspaceLoadProfile {
            name: if self.load_profile_name_str.trim().is_empty() {
                "Default Profile".to_string()
            } else {
                self.load_profile_name_str.trim().to_string()
            },
            target_scope: self.load_target_scope.clone(),
            environment_name: self.load_env_override.clone(),
            profile_mode,
            total_iterations: self.load_iterations_str.trim().parse().unwrap_or(20).max(1),
            concurrency: self.load_concurrency_str.trim().parse().unwrap_or(4).max(1),
            delay_ms: self.load_delay_ms_str.trim().parse().unwrap_or(0),
            ramp_up_secs: self.load_ramp_up_str.trim().parse().unwrap_or(0.0),
            ramp_down_secs: self.load_ramp_down_str.trim().parse().unwrap_or(0.0),
            duration_secs: self.load_duration_str.trim().parse::<f64>().ok().filter(|v| *v > 0.0),
            stages,
            target_rps: if self.load_target_rps_enabled {
                self.load_target_rps_str.trim().parse::<f64>().ok().filter(|v| *v > 0.0)
            } else {
                None
            },
            think_mode,
            think_a: self.load_think_a_str.trim().parse().unwrap_or(0.0),
            think_b: self.load_think_b_str.trim().parse().unwrap_or(0.0),
            sync_barrier_enabled: self.load_barrier_enabled,
            sync_barrier_group_size: self.load_barrier_size_str.trim().parse().unwrap_or(4).max(2),
            sync_barrier_timeout_ms: self.load_barrier_timeout_str.trim().parse().unwrap_or(5000).max(50),
            weighted_routing: self.load_weighted_routing,
            data_feeder_enabled: self.load_data_enabled,
            data_feeder_path,
            data_sharing,
            data_eof,
            max_duration_ms: self.load_max_duration_ms_str.trim().parse().ok().filter(|&v| v > 0),
            min_size_bytes: self.load_min_size_bytes_str.trim().parse().ok().filter(|&v| v > 0),
            max_size_bytes: self.load_max_size_bytes_str.trim().parse().ok().filter(|&v| v > 0),
            sla_p95_ms: self.load_sla_p95_str.trim().parse().ok().filter(|&v| v > 0),
            sla_p99_ms: self.load_sla_p99_str.trim().parse().ok().filter(|&v| v > 0),
            sla_error_rate_percent: self.load_sla_err_str.trim().parse().ok().filter(|&v| v >= 0.0),
            request_overrides: overrides,
        }
    }

    /// Applies a `WorkspaceLoadProfile` to Load Tester UI state.
    pub fn apply_workspace_load_profile(&mut self, profile: &aura_sync::WorkspaceLoadProfile) {
        self.load_profile_name_str = profile.name.clone();
        self.load_target_scope = profile.target_scope.clone();
        self.load_use_collection = !matches!(profile.target_scope, aura_sync::LoadTargetScope::ActiveRequest);
        self.load_env_override = profile.environment_name.clone();
        self.load_profile_mode = match profile.profile_mode.as_str() {
            "Duration" => aura_engine::LoadProfileMode::Duration,
            "RampUpDown" => aura_engine::LoadProfileMode::RampUpDown,
            "SteppedStages" => aura_engine::LoadProfileMode::SteppedStages,
            _ => aura_engine::LoadProfileMode::FixedIterations,
        };
        self.load_iterations_str = profile.total_iterations.to_string();
        self.load_concurrency_str = profile.concurrency.to_string();
        self.load_delay_ms_str = profile.delay_ms.to_string();
        self.load_ramp_up_str = profile.ramp_up_secs.to_string();
        self.load_ramp_down_str = profile.ramp_down_secs.to_string();
        self.load_duration_str = profile.duration_secs.unwrap_or(20.0).to_string();
        if !profile.stages.is_empty() {
            self.load_stages = profile
                .stages
                .iter()
                .map(|(v, s)| (v.to_string(), s.to_string()))
                .collect();
        }
        self.load_target_rps_enabled = profile.target_rps.is_some();
        self.load_target_rps_str = profile.target_rps.unwrap_or(50.0).to_string();
        self.load_think_mode = match profile.think_mode.as_str() {
            "Uniform" => LoadThinkMode::Uniform,
            "Gaussian" => LoadThinkMode::Gaussian,
            _ => LoadThinkMode::None,
        };
        self.load_think_a_str = profile.think_a.to_string();
        self.load_think_b_str = profile.think_b.to_string();
        self.load_barrier_enabled = profile.sync_barrier_enabled;
        self.load_barrier_size_str = profile.sync_barrier_group_size.to_string();
        self.load_barrier_timeout_str = profile.sync_barrier_timeout_ms.to_string();
        self.load_weighted_routing = profile.weighted_routing;
        self.load_data_enabled = profile.data_feeder_enabled;
        self.load_data_file_path = profile.data_feeder_path.clone().unwrap_or_default();
        self.load_data_sharing = if profile.data_sharing == "PerVU" {
            aura_engine::DataSharingMode::PerVU
        } else {
            aura_engine::DataSharingMode::AllVUs
        };
        self.load_data_eof = if profile.data_eof == "StopVuOnEof" {
            aura_engine::DataEofBehavior::StopVuOnEof
        } else {
            aura_engine::DataEofBehavior::Recycle
        };
        self.load_max_duration_ms_str = profile.max_duration_ms.map(|v| v.to_string()).unwrap_or_default();
        self.load_min_size_bytes_str = profile.min_size_bytes.map(|v| v.to_string()).unwrap_or_default();
        self.load_max_size_bytes_str = profile.max_size_bytes.map(|v| v.to_string()).unwrap_or_default();
        self.load_sla_p95_str = profile.sla_p95_ms.map(|v| v.to_string()).unwrap_or_default();
        self.load_sla_p99_str = profile.sla_p99_ms.map(|v| v.to_string()).unwrap_or_default();
        self.load_sla_err_str = profile.sla_error_rate_percent.map(|v| v.to_string()).unwrap_or_default();
        self.load_request_overrides = profile
            .request_overrides
            .iter()
            .map(|o| (o.relative_path.clone(), o.clone()))
            .collect();
        self.apply_overrides_to_discovered_requests();
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

    fn new_environment_draft() -> EnvironmentDraft {
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
        EnvironmentDraft {
            original_path: None,
            profile,
            mode: EnvironmentEditorMode::Ui,
            code_buffer,
            code_error: None,
            is_dirty: true,
        }
    }

    fn build_env_drafts(&self) -> Vec<EnvironmentDraft> {
        self.environments
            .iter()
            .map(|(path, env)| EnvironmentDraft {
                original_path: Some(path.clone()),
                profile: env.clone(),
                mode: EnvironmentEditorMode::Ui,
                code_buffer: aura_sync::environment_to_toml(env).unwrap_or_default(),
                code_error: None,
                is_dirty: false,
            })
            .collect()
    }

    pub fn open_environment_manager(&mut self, select_id: Option<&str>) {
        self.is_env_dropdown_open = false;
        self.revealed_env_secret_rows.clear();
        let mut drafts = self.build_env_drafts();
        if drafts.is_empty() {
            drafts.push(Self::new_environment_draft());
        }
        let selected_id = select_id
            .and_then(|id| drafts.iter().find(|d| d.profile.id == id).map(|d| d.profile.id.clone()))
            .or_else(|| {
                self.active_environment_id.as_ref().and_then(|aid| {
                    drafts.iter().find(|d| &d.profile.id == aid).map(|d| d.profile.id.clone())
                })
            })
            .unwrap_or_else(|| drafts[0].profile.id.clone());

        self.environment_editor = Some(EnvironmentEditorState {
            drafts,
            selected_id,
            confirm_close: false,
            last_saved_message: None,
        });
    }

    pub fn open_new_environment_editor(&mut self) {
        self.is_env_dropdown_open = false;
        self.revealed_env_secret_rows.clear();
        let new_draft = Self::new_environment_draft();
        let new_id = new_draft.profile.id.clone();

        if let Some(ref mut ed) = self.environment_editor {
            ed.drafts.push(new_draft);
            ed.selected_id = new_id;
            ed.confirm_close = false;
            ed.last_saved_message = None;
        } else {
            let mut drafts = self.build_env_drafts();
            drafts.push(new_draft);
            self.environment_editor = Some(EnvironmentEditorState {
                drafts,
                selected_id: new_id,
                confirm_close: false,
                last_saved_message: None,
            });
        }
    }

    pub fn open_environment_editor(&mut self, id: &str) {
        self.open_environment_manager(Some(id));
    }

    pub fn select_environment_draft(&mut self, id: &str) {
        let Some(ref mut ed) = self.environment_editor else {
            return;
        };
        if ed.selected_id != id && ed.drafts.iter().any(|d| d.profile.id == id) {
            ed.selected_id = id.to_string();
            ed.confirm_close = false;
            ed.last_saved_message = None;
            self.revealed_env_secret_rows.clear();
        }
    }

    pub fn set_environment_editor_mode(&mut self, mode: EnvironmentEditorMode) {
        let Some(ref mut ed) = self.environment_editor else {
            return;
        };
        let Some(draft) = ed.current_mut() else {
            return;
        };
        if draft.mode == mode {
            return;
        }
        match mode {
            EnvironmentEditorMode::Code => {
                draft.code_buffer =
                    aura_sync::environment_to_toml(&draft.profile).unwrap_or_default();
                draft.code_error = None;
                draft.mode = EnvironmentEditorMode::Code;
            }
            EnvironmentEditorMode::Ui => {
                if draft.code_error.is_none() {
                    draft.mode = EnvironmentEditorMode::Ui;
                }
            }
        }
    }

    pub fn update_environment_code_buffer(&mut self, text: String) {
        let Some(ref mut ed) = self.environment_editor else {
            return;
        };
        let Some(draft) = ed.current_mut() else {
            return;
        };
        if draft.code_buffer == text {
            return;
        }
        draft.code_buffer = text.clone();
        draft.is_dirty = true;
        match aura_sync::toml_to_environment(&text, Some(&draft.profile.id)) {
            Ok(parsed) => {
                draft.profile = parsed;
                draft.code_error = None;
            }
            Err(err) => {
                draft.code_error = Some(err.to_string());
            }
        }
        ed.last_saved_message = None;
    }

    pub fn format_environment_code_buffer(&mut self) {
        let Some(ref mut ed) = self.environment_editor else {
            return;
        };
        let Some(draft) = ed.current_mut() else {
            return;
        };
        if draft.code_error.is_none() {
            if let Ok(formatted) = aura_sync::environment_to_toml(&draft.profile) {
                if draft.code_buffer != formatted {
                    draft.code_buffer = formatted;
                    draft.is_dirty = true;
                    ed.last_saved_message = None;
                }
            }
        }
    }

    pub fn request_close_environment_editor(&mut self) {
        if let Some(ref mut ed) = self.environment_editor {
            if ed.has_dirty() && !ed.confirm_close {
                ed.confirm_close = true;
                return;
            }
        }
        self.close_environment_editor();
    }

    pub fn cancel_close_environment_editor(&mut self) {
        if let Some(ref mut ed) = self.environment_editor {
            ed.confirm_close = false;
        }
    }

    pub fn close_environment_editor(&mut self) {
        self.environment_editor = None;
        self.revealed_env_secret_rows.clear();
    }

    fn persist_env_draft(
        root: &std::path::Path,
        draft: &mut EnvironmentDraft,
    ) -> anyhow::Result<PathBuf> {
        if draft.mode == EnvironmentEditorMode::Code {
            if let Some(ref err) = draft.code_error {
                anyhow::bail!("Fix TOML syntax errors before saving: {err}");
            }
        }
        if draft.profile.name.trim().is_empty() {
            draft.profile.name = "environment".to_string();
        }
        draft
            .profile
            .variables
            .retain(|v| !v.key.trim().is_empty() || !v.value.trim().is_empty());
        // Persist any newly entered plaintext secret variables into the OS Keychain
        // and replace their in-memory value with a `keyring://` reference before saving TOML.
        let env_svc = aura_core::model::VariableItem::env_keychain_service(&draft.profile.name);
        for v in &mut draft.profile.variables {
            let key = v.key.trim().to_string();
            if v.is_secret
                && !key.is_empty()
                && !aura_core::model::VariableItem::is_secret_uri(&v.value)
            {
                if let Err(e) = aura_engine::keychain_set(&env_svc, &key, &v.value) {
                    tracing::warn!(error = %e, key = %key, "Failed to store secret in OS Keychain");
                } else {
                    v.value =
                        aura_core::model::VariableItem::env_keychain_uri(&draft.profile.name, &key);
                }
            }
        }
        let saved_path = aura_sync::save_environment_to_dir(
            root,
            &draft.profile,
            draft.original_path.as_deref(),
        )?;
        draft.original_path = Some(saved_path.clone());
        draft.code_buffer = aura_sync::environment_to_toml(&draft.profile).unwrap_or_default();
        draft.code_error = None;
        draft.is_dirty = false;
        Ok(saved_path)
    }

    pub fn save_environment_editor(&mut self) -> anyhow::Result<()> {
        let Some(ref mut ed) = self.environment_editor else {
            return Ok(());
        };
        let Some(draft) = ed.current() else {
            return Ok(());
        };
        if draft.mode == EnvironmentEditorMode::Code {
            if let Some(ref err) = draft.code_error {
                anyhow::bail!("Fix TOML syntax errors before saving: {err}");
            }
        }
        let Some(root) = self.workspace_store.workspace_root.clone() else {
            self.status_message =
                Some("Open a workspace folder first to save environments".to_string());
            return Ok(());
        };
        let selected_id = ed.selected_id.clone();
        let was_active = self.active_environment_id.as_deref() == Some(selected_id.as_str());
        let saved_path = {
            let draft = ed.current_mut().expect("checked above");
            Self::persist_env_draft(&root, draft)?
        };
        self.revealed_env_secret_rows.clear();
        self.reload_environments_from_workspace();

        let mut saved_name = String::new();
        if let Some(ref mut ed) = self.environment_editor {
            if let Some((_, matched)) = self.environments.iter().find(|(p, _)| p == &saved_path) {
                saved_name = matched.name.clone();
                if let Some(draft) = ed.current_mut() {
                    draft.profile.id = matched.id.clone();
                }
                ed.selected_id = matched.id.clone();
                if was_active || self.active_environment_id.is_none() {
                    self.active_environment_id = Some(matched.id.clone());
                }
            } else if let Some(draft) = ed.current() {
                saved_name = draft.profile.name.clone();
            }
            ed.confirm_close = false;
            ed.last_saved_message = Some(format!("Saved '{saved_name}'"));
        }
        self.status_message = Some(format!("Saved environment '{saved_name}'"));
        Ok(())
    }

    pub fn save_all_environment_drafts(&mut self) -> anyhow::Result<()> {
        let Some(ref ed) = self.environment_editor else {
            return Ok(());
        };
        for draft in &ed.drafts {
            if draft.is_dirty && draft.mode == EnvironmentEditorMode::Code {
                if let Some(ref err) = draft.code_error {
                    anyhow::bail!(
                        "Fix TOML syntax errors in '{}' before saving: {err}",
                        draft.profile.name
                    );
                }
            }
        }
        let Some(root) = self.workspace_store.workspace_root.clone() else {
            self.status_message =
                Some("Open a workspace folder first to save environments".to_string());
            return Ok(());
        };
        let mut saved_count = 0usize;
        let mut saved_paths: Vec<(String, PathBuf)> = Vec::new();
        if let Some(ref mut ed) = self.environment_editor {
            for draft in &mut ed.drafts {
                if draft.is_dirty {
                    let old_id = draft.profile.id.clone();
                    let path = Self::persist_env_draft(&root, draft)?;
                    saved_paths.push((old_id, path));
                    saved_count += 1;
                }
            }
        }
        if saved_count == 0 {
            return self.save_environment_editor();
        }
        self.revealed_env_secret_rows.clear();
        self.reload_environments_from_workspace();

        if let Some(ref mut ed) = self.environment_editor {
            for (old_id, path) in saved_paths {
                if let Some((_, matched)) = self.environments.iter().find(|(p, _)| p == &path) {
                    if let Some(draft) = ed.drafts.iter_mut().find(|d| d.profile.id == old_id) {
                        draft.profile.id = matched.id.clone();
                    }
                    if ed.selected_id == old_id {
                        ed.selected_id = matched.id.clone();
                    }
                    if self.active_environment_id.as_deref() == Some(old_id.as_str())
                        || self.active_environment_id.is_none()
                    {
                        self.active_environment_id = Some(matched.id.clone());
                    }
                }
            }
            ed.confirm_close = false;
            ed.last_saved_message = Some(format!("Saved {saved_count} environment(s)"));
        }
        self.status_message = Some(format!("Saved {saved_count} environment(s)"));
        Ok(())
    }

    pub fn save_all_and_close_environment_editor(&mut self) -> anyhow::Result<()> {
        self.save_all_environment_drafts()?;
        self.close_environment_editor();
        Ok(())
    }

    pub fn duplicate_environment_in_editor(&mut self, id: &str) -> anyhow::Result<()> {
        let Some(ref mut ed) = self.environment_editor else {
            return Ok(());
        };
        let Some(src) = ed.drafts.iter().find(|d| d.profile.id == id).cloned() else {
            return Ok(());
        };
        let mut dup_profile = src.profile;
        dup_profile.id = uuid::Uuid::new_v4().to_string();
        dup_profile.name = format!("{} Copy", dup_profile.name);
        let code_buffer = aura_sync::environment_to_toml(&dup_profile).unwrap_or_default();
        let new_id = dup_profile.id.clone();
        let mut new_draft = EnvironmentDraft {
            original_path: None,
            profile: dup_profile,
            mode: EnvironmentEditorMode::Ui,
            code_buffer,
            code_error: None,
            is_dirty: true,
        };
        if let Some(root) = self.workspace_store.workspace_root.clone() {
            if let Ok(saved_path) = Self::persist_env_draft(&root, &mut new_draft) {
                self.reload_environments_from_workspace();
                if let Some((_, matched)) = self.environments.iter().find(|(p, _)| p == &saved_path) {
                    new_draft.profile.id = matched.id.clone();
                }
            }
        }
        let final_id = new_draft.profile.id.clone();
        if let Some(ref mut ed) = self.environment_editor {
            ed.drafts.push(new_draft);
            ed.selected_id = final_id.clone();
            ed.confirm_close = false;
            ed.last_saved_message = None;
        } else {
            let _ = new_id;
        }
        self.revealed_env_secret_rows.clear();
        Ok(())
    }

    pub fn delete_environment_in_editor(&mut self, id: &str) -> anyhow::Result<()> {
        let has_path = self
            .environment_editor
            .as_ref()
            .and_then(|ed| ed.drafts.iter().find(|d| d.profile.id == id))
            .is_some_and(|d| d.original_path.is_some());
        if has_path {
            self.delete_environment(id)?;
        }
        if let Some(ref mut ed) = self.environment_editor {
            ed.drafts.retain(|d| d.profile.id != id);
            if ed.drafts.is_empty() {
                let blank = Self::new_environment_draft();
                ed.selected_id = blank.profile.id.clone();
                ed.drafts.push(blank);
            } else if ed.selected_id == id {
                ed.selected_id = ed.drafts[0].profile.id.clone();
            }
            ed.confirm_close = false;
            ed.last_saved_message = None;
        }
        self.revealed_env_secret_rows.clear();
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
            let draft = state.environment_editor.as_mut().unwrap().current_mut().unwrap();
            assert_eq!(draft.mode, EnvironmentEditorMode::Ui);
            draft.profile.name = "Staging".to_string();
            draft.profile.variables = vec![
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
            let draft = state.environment_editor.as_ref().unwrap().current().unwrap();
            assert_eq!(draft.mode, EnvironmentEditorMode::Code);
            assert!(draft.code_error.is_none());
            assert!(draft.code_buffer.contains("name = \"Staging\""));
            assert!(draft.code_buffer.contains("API_URL = \"https://staging.example.com\""));
            assert!(draft.code_buffer.contains("\"DEBUG~\" = \"1\""));
        }

        // Edit in Code mode with valid TOML -> updates profile
        state.update_environment_code_buffer(
            "name = \"Production\"\n\n[variables]\nAPI_URL = \"https://api.example.com\"\n"
                .to_string(),
        );
        {
            let draft = state.environment_editor.as_ref().unwrap().current().unwrap();
            assert!(draft.code_error.is_none());
            assert_eq!(draft.profile.name, "Production");
            assert_eq!(draft.profile.variables.len(), 1);
            assert_eq!(draft.profile.variables[0].key, "API_URL");
            assert_eq!(draft.profile.variables[0].value, "https://api.example.com");
            assert!(draft.is_dirty);
        }

        // Edit with invalid TOML -> records error and blocks switching back or saving
        state.update_environment_code_buffer("name = [invalid".to_string());
        assert!(state
            .environment_editor
            .as_ref()
            .unwrap()
            .current()
            .unwrap()
            .code_error
            .is_some());
        state.set_environment_editor_mode(EnvironmentEditorMode::Ui);
        assert_eq!(
            state.environment_editor.as_ref().unwrap().current().unwrap().mode,
            EnvironmentEditorMode::Code
        );
        assert!(state.save_environment_editor().is_err());
        assert!(state.environment_editor.is_some());

        // Fix TOML and switch back to UI
        state.update_environment_code_buffer(
            "name = \"Production\"\n\n[variables]\nTOKEN = \"xyz\"\n".to_string(),
        );
        state.set_environment_editor_mode(EnvironmentEditorMode::Ui);
        let draft = state.environment_editor.as_ref().unwrap().current().unwrap();
        assert_eq!(draft.mode, EnvironmentEditorMode::Ui);
        assert!(draft.code_error.is_none());
        assert_eq!(draft.profile.variables[0].key, "TOKEN");
    }

    #[test]
    fn test_environment_manager_switch_save_without_close_and_confirm() {
        let dir = std::env::temp_dir().join(format!("aura_env_mgr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let mut state = AuraAppState::new();
        state.workspace_store.workspace_root = Some(dir.clone());

        // Create two environments
        state.open_new_environment_editor();
        let id1 = state.environment_editor.as_ref().unwrap().selected_id.clone();
        state.environment_editor.as_mut().unwrap().current_mut().unwrap().profile.name =
            "Dev".to_string();
        state.save_environment_editor().unwrap();

        // Modal must remain open after Save!
        assert!(state.environment_editor.is_some());
        let saved_id1 = state.environment_editor.as_ref().unwrap().selected_id.clone();
        assert!(!state.environment_editor.as_ref().unwrap().current().unwrap().is_dirty);

        // Add second profile while modal is open
        state.open_new_environment_editor();
        let id2 = state.environment_editor.as_ref().unwrap().selected_id.clone();
        assert_ne!(id1, id2);
        state.environment_editor.as_mut().unwrap().current_mut().unwrap().profile.name =
            "Prod".to_string();

        // Switch back to Dev without losing Prod's dirty draft
        state.select_environment_draft(&saved_id1);
        assert_eq!(state.environment_editor.as_ref().unwrap().selected_id, saved_id1);
        assert_eq!(state.environment_editor.as_ref().unwrap().dirty_count(), 1);

        // Request close with dirty draft -> triggers confirm_close, keeps modal open
        state.request_close_environment_editor();
        assert!(state.environment_editor.as_ref().unwrap().confirm_close);

        // Save all & close -> persists Prod and closes modal
        state.save_all_and_close_environment_editor().unwrap();
        assert!(state.environment_editor.is_none());
        assert_eq!(state.environments.len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
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

    #[test]
    fn test_workspace_load_profile_roundtrip_and_effective_requests() {
        let mut state = AuraAppState::new();
        state.load_profile_name_str = "API Spike".to_string();
        state.load_target_scope = aura_sync::LoadTargetScope::Folder {
            relative_path: "users".to_string(),
        };
        state.load_env_override = Some("Staging".to_string());
        state.load_concurrency_str = "16".to_string();
        state.load_profile_mode = aura_engine::LoadProfileMode::Duration;
        state.load_duration_str = "45".to_string();
        state.environments.push((
            PathBuf::from("environment/staging.toml"),
            aura_core::model::EnvironmentProfile {
                id: "env-1".to_string(),
                name: "Staging".to_string(),
                variables: vec![aura_core::model::VariableItem {
                    key: "BASE_URL".to_string(),
                    value: "https://staging.example.com".to_string(),
                    enabled: true,
                    is_secret: false,
                }],
            },
        ));

        let req_a = AuraRequest::new("Login", HttpMethod::POST, "https://example.com/login");
        let req_b = AuraRequest::new("GetUser", HttpMethod::GET, "https://example.com/user");
        state.load_discovered_requests = vec![
            LoadRequestEntry {
                relative_path: "users/login.aura".to_string(),
                abs_path: PathBuf::from("users/login.aura"),
                name: "Login".to_string(),
                method: HttpMethod::POST,
                url: "https://example.com/login".to_string(),
                included: true,
                is_dirty_tab: false,
                request: req_a,
            },
            LoadRequestEntry {
                relative_path: "users/get.aura".to_string(),
                abs_path: PathBuf::from("users/get.aura"),
                name: "GetUser".to_string(),
                method: HttpMethod::GET,
                url: "https://example.com/user".to_string(),
                included: false,
                is_dirty_tab: true,
                request: req_b,
            },
        ];
        state.sync_override_for_request_idx(0);
        state.sync_override_for_request_idx(1);

        let effective = state.effective_load_requests();
        assert_eq!(effective.len(), 1);
        assert_eq!(effective[0].name, "Login");

        let ctx = state.template_context_for_load();
        assert_eq!(
            ctx.environment_vars.get("BASE_URL").map(String::as_str),
            Some("https://staging.example.com")
        );

        let saved = state.to_workspace_load_profile();
        let mut restored = AuraAppState::new();
        restored.apply_workspace_load_profile(&saved);
        assert_eq!(restored.load_profile_name_str, "API Spike");
        assert_eq!(restored.load_concurrency_str, "16");
        assert_eq!(restored.load_env_override.as_deref(), Some("Staging"));
        assert_eq!(
            restored.load_target_scope,
            aura_sync::LoadTargetScope::Folder {
                relative_path: "users".to_string()
            }
        );
    }
}
