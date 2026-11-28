use super::AuraWorkspaceView;
use crate::app_state::EditorDialogState;
use crate::input_registry::InputKey;
use gpui::*;
use gpui_kit::component::input::{EditorState, InputEvent, InputState, TextareaState};

impl AuraWorkspaceView {
    pub(super) fn handle_input_event(
        &mut self,
        key: &InputKey,
        state: &Entity<InputState>,
        event: &InputEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                let val = state.read(cx).value().to_string();
                if let Some(target) = key.to_active_target() {
                    self.state.set_input_text_for(target, val);
                    self.schedule_auto_save(cx);
                } else {
                    match key {
                        InputKey::CommandPaletteSearch => {
                            self.state.command_palette_search = val;
                            self.state.selected_palette_index = 0;
                        }
                        InputKey::RenameDialog => {
                            self.state.update_rename_text(val);
                        }
                        InputKey::MoveToSearch => {
                            self.state.update_move_to_search(val);
                        }
                        InputKey::InlineRename => {
                            if let Some(ref mut ir) = self.state.inline_rename {
                                ir.current_value = val;
                            }
                        }
                        InputKey::EditorDialogField(idx) => {
                            Self::set_editor_dialog_field(&mut self.state.editor_dialog, *idx, val);
                        }
                        InputKey::ResponseTableSearch => {
                            self.state.set_response_table_search(val);
                        }
                        InputKey::EnvName => {
                            if let Some(ref mut env_ed) = self.state.environment_editor {
                                if let Some(draft) = env_ed.current_mut() {
                                    if draft.profile.name != val {
                                        draft.profile.name = val;
                                        draft.is_dirty = true;
                                        env_ed.last_saved_message = None;
                                    }
                                }
                            }
                        }
                        InputKey::EnvVarKey(idx) => {
                            if let Some(ref mut env_ed) = self.state.environment_editor {
                                if let Some(draft) = env_ed.current_mut() {
                                    if let Some(v) = draft.profile.variables.get_mut(*idx) {
                                        if v.key != val {
                                            v.key = val;
                                            draft.is_dirty = true;
                                            env_ed.last_saved_message = None;
                                        }
                                    }
                                }
                            }
                        }
                        InputKey::EnvVarValue(idx) => {
                            if let Some(ref mut env_ed) = self.state.environment_editor {
                                if let Some(draft) = env_ed.current_mut() {
                                    if let Some(v) = draft.profile.variables.get_mut(*idx) {
                                        if v.value != val {
                                            v.value = val;
                                            draft.is_dirty = true;
                                            env_ed.last_saved_message = None;
                                        }
                                    }
                                }
                            }
                        }
                        InputKey::WsTemplateName => {
                            self.state.ws_template_name = val;
                        }
                        InputKey::GrpcService => {
                            if let Some(tab) = self.state.tabs.get_mut(self.state.active_tab_index)
                            {
                                let cfg = tab
                                    .request
                                    .grpc
                                    .get_or_insert_with(aura_core::model::GrpcConfig::default);
                                cfg.service = val;
                                tab.mark_dirty(true);
                                tab.sync_spec_from_request();
                            }
                        }
                        InputKey::GrpcMethod => {
                            if let Some(tab) = self.state.tabs.get_mut(self.state.active_tab_index)
                            {
                                let cfg = tab
                                    .request
                                    .grpc
                                    .get_or_insert_with(aura_core::model::GrpcConfig::default);
                                cfg.method = val;
                                tab.mark_dirty(true);
                                tab.sync_spec_from_request();
                            }
                        }
                        InputKey::SoapOperation => {
                            self.state.soap_operation = val;
                        }
                        InputKey::SoapNamespace => {
                            self.state.soap_namespace = val;
                        }
                        InputKey::SoapAction => {
                            self.state.soap_action = val;
                        }
                        InputKey::VaultAddr => {
                            self.state.vault_config.vault_addr = val;
                        }
                        InputKey::VaultToken => {
                            self.state.vault_config.vault_token = val;
                        }
                        InputKey::VaultNamespace => {
                            self.state.vault_config.vault_namespace =
                                if val.trim().is_empty() { None } else { Some(val) };
                        }
                        InputKey::VaultAwsRegion => {
                            self.state.vault_config.aws_region = val;
                        }
                        InputKey::VaultAwsProfile => {
                            self.state.vault_config.aws_profile =
                                if val.trim().is_empty() { None } else { Some(val) };
                        }
                        InputKey::VaultProfileName => {
                            self.state.draft_vault_profile_name = val;
                        }
                        InputKey::AwsProfileName => {
                            self.state.draft_aws_profile_name = val;
                        }
                        InputKey::KeePassProfileName => {
                            self.state.draft_keepass_name = val;
                        }
                        InputKey::KeePassDbPath => {
                            self.state.draft_keepass_db_path = val;
                        }
                        InputKey::KeePassKeyfilePath => {
                            self.state.draft_keepass_keyfile_path = val;
                        }
                        InputKey::KeePassPassword => {
                            self.state.draft_keepass_password = val;
                        }
                        InputKey::KeychainService => {
                            self.state.keychain_service = val;
                        }
                        InputKey::KeychainKey => {
                            self.state.keychain_key = val;
                        }
                        InputKey::KeychainValue => {
                            self.state.keychain_value = val;
                        }
                        InputKey::DomainCertDomain => {
                            self.state.draft_cert_domain = val;
                        }
                        InputKey::DomainCertPath => {
                            self.state.draft_cert_path = val;
                        }
                        InputKey::DomainCertKeyPath => {
                            self.state.draft_cert_key_path = val;
                        }
                        InputKey::DomainCertPassphrase => {
                            self.state.draft_cert_passphrase = val;
                        }
                        InputKey::GlobalProxyHost => {
                            self.state.global_proxy.host = val;
                        }
                        InputKey::GlobalProxyPort => {
                            if let Ok(p) = val.trim().parse::<u16>() {
                                self.state.global_proxy.port = p;
                            }
                        }
                        InputKey::GlobalProxyUsername => {
                            self.state.global_proxy.username = val;
                        }
                        InputKey::GlobalProxyPassword => {
                            self.state.global_proxy.password = val;
                        }
                        InputKey::GlobalProxyBypass => {
                            self.state.set_global_proxy_bypass_csv(&val);
                        }
                        InputKey::CookieFilter => {
                            self.state.cookie_domain_filter = val;
                        }
                        InputKey::CookieDraftDomain => {
                            self.state.draft_cookie_domain = val;
                        }
                        InputKey::CookieDraftPath => {
                            self.state.draft_cookie_path = val;
                        }
                        InputKey::CookieDraftName => {
                            self.state.draft_cookie_name = val;
                        }
                        InputKey::CookieDraftValue => {
                            self.state.draft_cookie_value = val;
                        }
                        InputKey::InterceptorBindAddr => {
                            self.state.interceptor_bind_addr = val;
                        }
                        InputKey::InterceptorRuleMatch => {
                            self.state.draft_rule_match = val;
                        }
                        InputKey::InterceptorRuleTarget => {
                            self.state.draft_rule_target = val;
                        }
                        InputKey::MockHostPort => {
                            self.state.mock_host_port = val;
                        }
                        InputKey::MockDraftMethod => {
                            self.state.mock_draft_method = val;
                        }
                        InputKey::MockDraftPath => {
                            self.state.mock_draft_path = val;
                        }
                        InputKey::MockDraftStatus => {
                            self.state.mock_draft_status = val;
                        }
                        InputKey::MockDraftBody => {
                            self.state.mock_draft_body = val;
                        }
                        InputKey::MockDraftHeaderRule => {
                            self.state.mock_draft_header_rule = val;
                        }
                        InputKey::MockDraftQueryRule => {
                            self.state.mock_draft_query_rule = val;
                        }
                        InputKey::MockDraftBodyRule => {
                            self.state.mock_draft_body_rule = val;
                        }
                        InputKey::MockDraftDatasetPath => {
                            self.state.mock_draft_dataset_path = val;
                        }
                        InputKey::GitCommitMessage => {
                            self.state.git_commit_message = val;
                        }
                        InputKey::GitNewBranchName => {
                            self.state.git_new_branch_name = val;
                        }
                        InputKey::LoadIterations => {
                            self.state.load_iterations_str = val;
                        }
                        InputKey::LoadConcurrency => {
                            self.state.load_concurrency_str = val;
                        }
                        InputKey::LoadDelayMs => {
                            self.state.load_delay_ms_str = val;
                        }
                        InputKey::LoadRampUpSecs => {
                            self.state.load_ramp_up_str = val;
                        }
                        InputKey::LoadRampDownSecs => {
                            self.state.load_ramp_down_str = val;
                        }
                        InputKey::LoadDurationSecs => {
                            self.state.load_duration_str = val;
                        }
                        InputKey::LoadTargetRps => {
                            self.state.load_target_rps_str = val;
                        }
                        InputKey::LoadStageVus(idx) => {
                            if let Some(st) = self.state.load_stages.get_mut(*idx) {
                                st.0 = val;
                            }
                        }
                        InputKey::LoadStageDurationSecs(idx) => {
                            if let Some(st) = self.state.load_stages.get_mut(*idx) {
                                st.1 = val;
                            }
                        }
                        InputKey::LoadThinkA => {
                            self.state.load_think_a_str = val;
                        }
                        InputKey::LoadThinkB => {
                            self.state.load_think_b_str = val;
                        }
                        InputKey::LoadBarrierGroupSize => {
                            self.state.load_barrier_size_str = val;
                        }
                        InputKey::LoadBarrierTimeoutMs => {
                            self.state.load_barrier_timeout_str = val;
                        }
                        InputKey::FlowWeightPercent => {
                            self.state.set_active_flow_weight_str(&val);
                            self.schedule_auto_save(cx);
                        }
                        InputKey::FlowLoopCount => {
                            self.state.set_active_flow_loop_count_str(&val);
                            self.schedule_auto_save(cx);
                        }
                        InputKey::FlowConditionIf => {
                            self.state.set_active_flow_condition_if(val);
                            self.schedule_auto_save(cx);
                        }
                        InputKey::FlowLoopWhile => {
                            self.state.set_active_flow_loop_while(val);
                            self.schedule_auto_save(cx);
                        }
                        InputKey::LoadDataFilePath => {
                            self.state.load_data_file_path = val;
                        }
                        InputKey::LoadMaxDurationMs => {
                            self.state.load_max_duration_ms_str = val;
                        }
                        InputKey::LoadMinSizeBytes => {
                            self.state.load_min_size_bytes_str = val;
                        }
                        InputKey::LoadMaxSizeBytes => {
                            self.state.load_max_size_bytes_str = val;
                        }
                        InputKey::LoadSlaP95Ms => {
                            self.state.load_sla_p95_str = val;
                        }
                        InputKey::LoadSlaP99Ms => {
                            self.state.load_sla_p99_str = val;
                        }
                        InputKey::LoadSlaErrorPct => {
                            self.state.load_sla_err_str = val;
                        }
                        InputKey::LoadProfileName => {
                            self.state.load_profile_name_str = val;
                        }
                        InputKey::LoadReqWeight(idx) => {
                            if let Some(entry) = self.state.load_discovered_requests.get_mut(*idx) {
                                let flow = entry.request.flow.get_or_insert_with(Default::default);
                                flow.weight_percent = val.trim().parse::<u8>().ok();
                                self.state.sync_override_for_request_idx(*idx);
                            }
                        }
                        InputKey::LoadReqLoop(idx) => {
                            if let Some(entry) = self.state.load_discovered_requests.get_mut(*idx) {
                                let flow = entry.request.flow.get_or_insert_with(Default::default);
                                flow.loop_count = val.trim().parse::<u32>().ok().filter(|v| *v >= 1);
                                self.state.sync_override_for_request_idx(*idx);
                            }
                        }
                        InputKey::LoadReqIf(idx) => {
                            if let Some(entry) = self.state.load_discovered_requests.get_mut(*idx) {
                                let flow = entry.request.flow.get_or_insert_with(Default::default);
                                flow.condition_if = if val.trim().is_empty() { None } else { Some(val) };
                                self.state.sync_override_for_request_idx(*idx);
                            }
                        }
                        InputKey::LoadReqWhile(idx) => {
                            if let Some(entry) = self.state.load_discovered_requests.get_mut(*idx) {
                                let flow = entry.request.flow.get_or_insert_with(Default::default);
                                flow.loop_while = if val.trim().is_empty() { None } else { Some(val) };
                                self.state.sync_override_for_request_idx(*idx);
                            }
                        }
                        InputKey::AiPromptInput => {
                            self.state.ai_prompt_draft = val;
                        }
                        InputKey::AiCustomCliName => {
                            self.state.ai_custom_cli_name = val;
                        }
                        InputKey::AiCustomCliCmd => {
                            self.state.ai_custom_cli_cmd = val;
                        }
                        InputKey::AiCustomCliArgs => {
                            self.state.ai_custom_cli_args = val;
                        }
                        InputKey::AiHttpName => {
                            self.state.ai_http_name = val;
                        }
                        InputKey::AiHttpUrl => {
                            self.state.ai_http_url = val;
                        }
                        InputKey::AiHttpModel => {
                            self.state.ai_http_model = val;
                        }
                        InputKey::AiHttpApiKey => {
                            self.state.ai_http_api_key = val;
                        }
                        _ => {}
                    }
                }
                self.inputs
                    .set_template_context(self.state.active_template_context());
                self.inputs.sync_all_template_highlights(cx);
                cx.notify();
            }
            InputEvent::PressEnter { .. } => match key {
                InputKey::Url => {
                    self.dispatch_send_request(cx);
                }
                InputKey::CommandPaletteSearch => {
                    self.handle_palette_execution(cx);
                    cx.notify();
                }
                InputKey::RenameDialog => {
                    self.confirm_rename_action(cx);
                }
                InputKey::MoveToSearch => {
                    self.confirm_move_to_action(cx);
                }
                InputKey::InlineRename => {
                    self.state.inline_rename = None;
                    cx.notify();
                }
                InputKey::EditorDialogField(_) => {
                    self.state.confirm_editor_dialog();
                    self.schedule_auto_save(cx);
                    cx.notify();
                }
                _ => {}
            },
            InputEvent::Focus => {
                if let Some(target) = key.to_active_target() {
                    self.state.set_active_input(Some(target));
                    cx.notify();
                }
            }
            InputEvent::Blur => {
                if let Some(target) = key.to_active_target() {
                    if self.state.active_input_target.as_ref() == Some(&target) {
                        self.state.blur_active_input();
                        cx.notify();
                    }
                }
            }
        }
    }

    pub(super) fn handle_textarea_event(
        &mut self,
        key: &InputKey,
        state: &Entity<TextareaState>,
        event: &InputEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                let val = state.read(cx).value().to_string();
                if let Some(target) = key.to_active_target() {
                    self.state.set_input_text_for(target, val);
                    self.schedule_auto_save(cx);
                    cx.notify();
                } else if matches!(key, InputKey::EnvCode) {
                    self.state.update_environment_code_buffer(val);
                    cx.notify();
                }
            }
            InputEvent::Focus => {
                if let Some(target) = key.to_active_target() {
                    self.state.set_active_input(Some(target));
                    cx.notify();
                }
            }
            InputEvent::Blur => {
                if let Some(target) = key.to_active_target() {
                    if self.state.active_input_target.as_ref() == Some(&target) {
                        self.state.blur_active_input();
                        cx.notify();
                    }
                }
            }
            InputEvent::PressEnter { .. } => {}
        }
    }

    pub(super) fn handle_editor_event(
        &mut self,
        key: &InputKey,
        state: &Entity<EditorState>,
        event: &InputEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                let val = state.read(cx).value().to_string();
                if let Some(target) = key.to_active_target() {
                    self.state.set_input_text_for(target, val);
                    self.schedule_auto_save(cx);
                    self.inputs
                        .set_template_context(self.state.active_template_context());
                    self.inputs.sync_all_template_highlights(cx);
                    cx.notify();
                } else if matches!(key, InputKey::EnvCode) {
                    self.state.update_environment_code_buffer(val);
                    self.inputs
                        .set_template_context(self.state.active_template_context());
                    self.inputs.sync_all_template_highlights(cx);
                    cx.notify();
                } else {
                    match key {
                        InputKey::GraphQLQuery => {
                            self.state.set_graphql_query(val);
                            self.schedule_auto_save(cx);
                        }
                        InputKey::GraphQLVariables => {
                            self.state.set_graphql_variables(val);
                            self.schedule_auto_save(cx);
                        }
                        InputKey::WsMessageInput => {
                            self.state.ws_composer_text = val;
                        }
                        InputKey::GrpcPayload => {
                            if let Some(tab) = self.state.tabs.get_mut(self.state.active_tab_index)
                            {
                                let cfg = tab
                                    .request
                                    .grpc
                                    .get_or_insert_with(aura_core::model::GrpcConfig::default);
                                cfg.payload = val;
                                tab.mark_dirty(true);
                                tab.sync_spec_from_request();
                            }
                            self.schedule_auto_save(cx);
                        }
                        InputKey::WsdlImportText => {
                            if let Some(ref mut dlg) = self.state.wsdl_import_dialog {
                                dlg.wsdl_text = val;
                                dlg.error = None;
                            }
                        }
                        _ => {}
                    }
                    self.inputs
                        .set_template_context(self.state.active_template_context());
                    self.inputs.sync_all_template_highlights(cx);
                    cx.notify();
                }
            }
            InputEvent::Focus => {
                if let Some(target) = key.to_active_target() {
                    self.state.set_active_input(Some(target));
                    cx.notify();
                }
            }
            InputEvent::Blur => {
                if let Some(target) = key.to_active_target() {
                    if self.state.active_input_target.as_ref() == Some(&target) {
                        self.state.blur_active_input();
                        cx.notify();
                    }
                }
            }
            InputEvent::PressEnter { .. } => {}
        }
    }

    fn set_editor_dialog_field(dialog: &mut Option<EditorDialogState>, idx: usize, val: String) {
        let Some(dlg) = dialog.as_mut() else {
            return;
        };
        match dlg {
            EditorDialogState::AddHeader { key, value, .. }
            | EditorDialogState::EditHeader { key, value, .. }
            | EditorDialogState::AddParam { key, value, .. }
            | EditorDialogState::EditParam { key, value, .. }
            | EditorDialogState::AddFormParam { key, value, .. } => {
                if idx == 0 {
                    *key = val;
                } else if idx == 1 {
                    *value = val;
                }
            }
            EditorDialogState::EditBody { content } => {
                if idx == 0 {
                    *content = val;
                }
            }
            EditorDialogState::EditBearerAuth { token } => {
                if idx == 0 {
                    *token = val;
                }
            }
            EditorDialogState::EditBasicAuth {
                username, password, ..
            } => {
                if idx == 0 {
                    *username = val;
                } else if idx == 1 {
                    *password = val;
                }
            }
            EditorDialogState::EditApiKeyAuth { key, value, .. } => {
                if idx == 0 {
                    *key = val;
                } else if idx == 1 {
                    *value = val;
                }
            }
            EditorDialogState::EditAwsSigV4 {
                access_key,
                secret_key,
                region,
                service,
                session_token,
                ..
            } => match idx {
                0 => *access_key = val,
                1 => *secret_key = val,
                2 => *region = val,
                3 => *service = val,
                4 => *session_token = val,
                _ => {}
            },
            EditorDialogState::EditOAuth2 {
                access_token,
                grant_type,
                token_url,
                client_id,
                client_secret,
                ..
            } => match idx {
                0 => *access_token = val,
                1 => *grant_type = val,
                2 => *token_url = val,
                3 => *client_id = val,
                4 => *client_secret = val,
                _ => {}
            },
            EditorDialogState::AddMultipartField {
                name,
                value,
                file_path,
                ..
            } => match idx {
                0 => *name = val,
                1 => *value = val,
                2 => *file_path = val,
                _ => {}
            },
        }
    }

}
