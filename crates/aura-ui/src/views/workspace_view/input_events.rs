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
                                env_ed.profile.name = val;
                            }
                        }
                        InputKey::EnvVarKey(idx) => {
                            if let Some(ref mut env_ed) = self.state.environment_editor {
                                if let Some(v) = env_ed.profile.variables.get_mut(*idx) {
                                    v.key = val;
                                }
                            }
                        }
                        InputKey::EnvVarValue(idx) => {
                            if let Some(ref mut env_ed) = self.state.environment_editor {
                                if let Some(v) = env_ed.profile.variables.get_mut(*idx) {
                                    v.value = val;
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
                    let _ = self.state.execute_selected_palette_item();
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
