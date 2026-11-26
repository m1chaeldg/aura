use super::AuraWorkspaceView;
use crate::views::RequestEditorAction;
use gpui::*;

impl AuraWorkspaceView {
    pub fn handle_request_editor_action(
        &mut self,
        action: RequestEditorAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            RequestEditorAction::SelectTab(tab) => {
                self.state.set_editor_tab(tab);
            }
            RequestEditorAction::ToggleHeader(idx) => {
                self.state.toggle_header(idx);
            }
            RequestEditorAction::RemoveHeader(idx) => {
                self.state.remove_header(idx);
            }
            RequestEditorAction::OpenAddHeader => {
                self.state.add_new_header_and_focus();
            }
            RequestEditorAction::AddHeaderDirect => {
                self.state.add_new_header_and_focus();
            }
            RequestEditorAction::FocusHeaderKey(idx) => {
                self.state
                    .set_active_input(Some(crate::app_state::ActiveInputTarget::HeaderKey(idx)));
            }
            RequestEditorAction::FocusHeaderVal(idx) => {
                self.state
                    .set_active_input(Some(crate::app_state::ActiveInputTarget::HeaderValue(idx)));
            }
            RequestEditorAction::OpenEditHeader(idx) => {
                self.state
                    .set_active_input(Some(crate::app_state::ActiveInputTarget::HeaderKey(idx)));
            }
            RequestEditorAction::AddPresetHeader(k, v) => {
                self.state.add_header(k, v);
            }
            RequestEditorAction::ToggleParam(idx) => {
                self.state.toggle_param(idx);
            }
            RequestEditorAction::RemoveParam(idx) => {
                self.state.remove_param(idx);
            }
            RequestEditorAction::OpenAddParam => {
                self.state.add_new_param_and_focus();
            }
            RequestEditorAction::AddParamDirect => {
                self.state.add_new_param_and_focus();
            }
            RequestEditorAction::FocusParamKey(idx) => {
                self.state
                    .set_active_input(Some(crate::app_state::ActiveInputTarget::ParamKey(idx)));
            }
            RequestEditorAction::FocusParamVal(idx) => {
                self.state
                    .set_active_input(Some(crate::app_state::ActiveInputTarget::ParamValue(idx)));
            }
            RequestEditorAction::OpenEditParam(idx) => {
                self.state
                    .set_active_input(Some(crate::app_state::ActiveInputTarget::ParamKey(idx)));
            }
            RequestEditorAction::SetBodyType(name) => {
                self.state.set_body_type(name);
            }
            RequestEditorAction::OpenEditBody => {
                self.state
                    .set_active_input(Some(crate::app_state::ActiveInputTarget::BodyText));
            }
            RequestEditorAction::FocusBody => {
                self.state
                    .set_active_input(Some(crate::app_state::ActiveInputTarget::BodyText));
            }
            RequestEditorAction::FormatJsonBody => {
                if let Err(e) = self.state.format_active_json_body() {
                    self.state.status_message = Some(e);
                } else {
                    self.state.status_message = Some("JSON formatted".to_string());
                }
            }
            RequestEditorAction::ClearBody => {
                self.state.set_body_content(String::new());
            }
            RequestEditorAction::OpenAddFormParam => {
                self.state.add_new_form_param_and_focus();
            }
            RequestEditorAction::AddFormParamDirect => {
                self.state.add_new_form_param_and_focus();
            }
            RequestEditorAction::FocusFormParamKey(idx) => {
                self.state
                    .set_active_input(Some(crate::app_state::ActiveInputTarget::FormParamKey(idx)));
            }
            RequestEditorAction::FocusFormParamVal(idx) => {
                self.state.set_active_input(Some(
                    crate::app_state::ActiveInputTarget::FormParamValue(idx),
                ));
            }
            RequestEditorAction::ToggleFormParam(idx) => {
                self.state.toggle_form_urlencoded_param(idx);
            }
            RequestEditorAction::RemoveFormParam(idx) => {
                self.state.remove_form_urlencoded_param(idx);
            }
            RequestEditorAction::OpenAddMultipart => {
                self.state.add_multipart_field(
                    "field".to_string(),
                    Some("value".to_string()),
                    None,
                    None,
                );
            }
            RequestEditorAction::AddMultipartDirect => {
                self.state.add_multipart_field(
                    "field".to_string(),
                    Some("value".to_string()),
                    None,
                    None,
                );
            }
            RequestEditorAction::RemoveMultipart(idx) => {
                self.state.remove_multipart_field(idx);
            }
            RequestEditorAction::SetAuthType(name) => {
                self.state.set_auth_type(name);
            }
            RequestEditorAction::OpenEditAuth => {
                self.state
                    .set_active_input(Some(crate::app_state::ActiveInputTarget::AuthBearerToken));
            }
            RequestEditorAction::FocusAuthField(target) => {
                self.state.set_active_input(Some(target));
            }
            RequestEditorAction::SelectScriptSubTab(sub) => {
                self.state.set_script_sub_tab(sub);
            }
            RequestEditorAction::ToggleScriptReference => {
                self.state.toggle_script_reference();
            }
            RequestEditorAction::SearchInEditor(key) => {
                self.inputs.focus_editor(&key, window, cx);
                window.dispatch_action(Box::new(gpui_kit::component::input::Search), cx);
            }
            RequestEditorAction::TriggerAutocomplete(key) => {
                self.inputs.focus_editor(&key, window, cx);
                window.dispatch_action(Box::new(gpui_kit::component::input::ShowCompletions), cx);
            }
            RequestEditorAction::InsertDynamicSnippet(key, snippet) => {
                match key {
                    crate::input_registry::InputKey::PreRequestScript => {
                        self.state.append_script_snippet(true, &snippet);
                    }
                    crate::input_registry::InputKey::PostResponseScript => {
                        self.state.append_script_snippet(false, &snippet);
                    }
                    crate::input_registry::InputKey::TestScript => {
                        self.state.append_test_snippet(&snippet);
                    }
                    crate::input_registry::InputKey::SpecText => {
                        self.state.append_spec_snippet(&snippet);
                    }
                    _ => {}
                }
                self.schedule_auto_save(cx);
            }
            RequestEditorAction::FocusScript(sub) => match sub {
                crate::app_state::ScriptSubTab::PreRequest => {
                    self.state.set_active_input(Some(
                        crate::app_state::ActiveInputTarget::PreRequestScript,
                    ));
                }
                crate::app_state::ScriptSubTab::PostResponse => {
                    self.state.set_active_input(Some(
                        crate::app_state::ActiveInputTarget::PostResponseScript,
                    ));
                }
            },
            RequestEditorAction::InsertScriptSnippet(is_pre, snippet) => {
                self.state.append_script_snippet(is_pre, snippet);
            }
            RequestEditorAction::FocusTests => {
                self.state
                    .set_active_input(Some(crate::app_state::ActiveInputTarget::TestScript));
            }
            RequestEditorAction::InsertTestSnippet(snippet) => {
                self.state.append_test_snippet(snippet);
            }
            RequestEditorAction::RunTests => match self.state.run_active_tests() {
                Ok(n) => {
                    self.state.status_message = Some(format!("Ran tests: {n} assertions evaluated"))
                }
                Err(e) => self.state.status_message = Some(e),
            },
            RequestEditorAction::ToggleApiKeyPlacement(in_header) => {
                if let Some(tab) = self.state.tabs.get_mut(self.state.active_tab_index) {
                    if let aura_core::model::AuthConfig::ApiKey {
                        in_header: ref mut h,
                        ..
                    } = tab.request.auth
                    {
                        *h = in_header;
                        tab.mark_dirty(true);
                    }
                }
            }
            RequestEditorAction::FocusSpec => {
                self.state
                    .set_active_input(Some(crate::app_state::ActiveInputTarget::SpecText));
            }
            RequestEditorAction::FormatSpec => {
                if let Some(tab) = self.state.tabs.get_mut(self.state.active_tab_index) {
                    tab.ensure_spec_buffer();
                    tab.spec_buffer = Some(aura_parser::request_to_spec(&tab.request));
                    tab.spec_error = None;
                    tab.mark_dirty(true);
                }
            }
            RequestEditorAction::CopySpec => {
                if let Some(tab) = self.state.tabs.get_mut(self.state.active_tab_index) {
                    tab.ensure_spec_buffer();
                    if let Some(content) = &tab.spec_buffer {
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(content.clone()));
                        self.state.status_message = Some("Copied Aura spec to clipboard".to_string());
                    }
                }
            }
            RequestEditorAction::PasteSpec => {
                if let Some(item) = cx.read_from_clipboard() {
                    if let Some(text) = item.text() {
                        if let Some(tab) = self.state.tabs.get_mut(self.state.active_tab_index) {
                            tab.update_spec_buffer(text);
                            self.state.status_message = Some("Pasted spec from clipboard".to_string());
                        }
                    }
                }
            }
            RequestEditorAction::RevertSpec => {
                if let Some(tab) = self.state.tabs.get_mut(self.state.active_tab_index) {
                    tab.revert_spec_buffer();
                }
            }
            RequestEditorAction::AddVariableDirect => {
                self.state.add_variable_row();
                let new_idx = self
                    .state
                    .tabs
                    .get(self.state.active_tab_index)
                    .map(|t| t.variable_rows.len().saturating_sub(1))
                    .unwrap_or(0);
                self.state.set_active_input(Some(
                    crate::app_state::ActiveInputTarget::VariableKey(new_idx),
                ));
            }
            RequestEditorAction::ToggleVariable(idx) => {
                self.state.toggle_variable_row(idx);
            }
            RequestEditorAction::RemoveVariable(idx) => {
                self.state.remove_variable_row(idx);
            }
            RequestEditorAction::ToggleSettingFollowRedirects => {
                self.state.toggle_setting_follow_redirects();
            }
            RequestEditorAction::ToggleSettingVerifySsl => {
                self.state.toggle_setting_verify_ssl();
            }
            RequestEditorAction::ToggleSettingInheritAuth => {
                self.state.toggle_setting_inherit_auth();
            }
        }
        if self.state.is_active_tab_dirty() {
            self.schedule_auto_save(cx);
        }
        cx.notify();
    }

}
