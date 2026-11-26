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
                self.state.close_body_format_dropdown();
            }
            RequestEditorAction::ToggleBodyFormatDropdown => {
                self.state.toggle_body_format_dropdown();
            }
            RequestEditorAction::DismissBodyFormatDropdown => {
                self.state.close_body_format_dropdown();
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
            RequestEditorAction::ToggleAwsUseProfile(use_profile) => {
                self.state.set_aws_use_profile(use_profile);
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
            RequestEditorAction::IntrospectGraphQLSchema => {
                let Some(tab) = self.state.active_tab().cloned() else {
                    return;
                };
                if self.state.is_graphql_introspecting {
                    return;
                }
                self.state.is_graphql_introspecting = true;
                self.state.status_message =
                    Some(format!("Introspecting GraphQL schema from {}...", tab.request.url));
                let tab_id = tab.id.clone();
                let req = tab.request.clone();
                let ctx = self.state.active_template_context();
                cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                    let mut cx = cx.clone();
                    async move {
                        let res = aura_engine::fetch_graphql_introspection(&req, &ctx).await;
                        let _ = this.update(&mut cx, |this, cx| {
                            this.state.is_graphql_introspecting = false;
                            match res {
                                Ok(schema) => {
                                    let count = schema.types.len();
                                    this.state.graphql_schemas.insert(tab_id.clone(), schema);
                                    this.state.show_graphql_schema = true;
                                    this.state.status_message =
                                        Some(format!("Loaded GraphQL schema ({count} types)"));
                                }
                                Err(e) => {
                                    this.state.status_message =
                                        Some(format!("GraphQL introspection failed: {e}"));
                                }
                            }
                            cx.notify();
                        });
                    }
                })
                .detach();
            }
            RequestEditorAction::ToggleGraphQLSchema => {
                self.state.show_graphql_schema = !self.state.show_graphql_schema;
            }
            RequestEditorAction::InsertGraphQLField(snippet) => {
                self.state.insert_graphql_field(&snippet);
            }
            RequestEditorAction::ToggleWsConnection => {
                if self.state.is_active_ws_connected() {
                    if let Some(tid) = self.state.active_tab().map(|t| t.id.clone()) {
                        self.state.disconnect_ws_tab(&tid);
                        self.state.status_message = Some("WebSocket disconnected".to_string());
                    }
                } else {
                    self.dispatch_connect_websocket(cx);
                }
            }
            RequestEditorAction::SendWsMessage => match self.state.send_active_ws_message() {
                Ok(()) => {
                    self.state.status_message = Some("Sent WebSocket message".to_string());
                }
                Err(e) => {
                    self.state.status_message = Some(e);
                }
            },
            RequestEditorAction::SaveWsTemplate => {
                self.state.save_active_ws_template();
                self.state.status_message = Some("Saved WebSocket message template".to_string());
            }
            RequestEditorAction::LoadWsTemplate(id) => {
                self.state.load_ws_template(&id);
            }
            RequestEditorAction::DeleteWsTemplate(id) => {
                self.state.delete_ws_template(&id);
            }
            RequestEditorAction::LoadProtoFile => {
                let Some(tab_id) = self.state.active_tab().map(|t| t.id.clone()) else {
                    return;
                };
                cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                    let mut cx = cx.clone();
                    async move {
                        let handle = rfd::AsyncFileDialog::new()
                            .set_title("Load .proto Schema")
                            .add_filter("Protocol Buffers", &["proto"])
                            .pick_file()
                            .await;
                        if let Some(file) = handle {
                            let path = file.path().to_path_buf();
                            if let Ok(content) = tokio::fs::read_to_string(&path).await {
                                let parsed = aura_core::grpc::parse_proto_file(&content);
                                let _ = this.update(&mut cx, |this, cx| {
                                    match parsed {
                                        Ok(schema) => {
                                            let svc_count = schema.services.len();
                                            this.state.apply_proto_schema(
                                                &tab_id,
                                                Some(path.to_string_lossy().to_string()),
                                                Some(content),
                                                schema,
                                            );
                                            this.state.status_message = Some(format!(
                                                "Loaded .proto schema ({svc_count} services)"
                                            ));
                                        }
                                        Err(e) => {
                                            this.state.status_message =
                                                Some(format!("Failed to parse .proto: {e}"));
                                        }
                                    }
                                    cx.notify();
                                });
                            }
                        }
                    }
                })
                .detach();
            }
            RequestEditorAction::ReflectGrpcServices => {
                let Some(tab) = self.state.active_tab().cloned() else {
                    return;
                };
                let tab_id = tab.id.clone();
                let ctx = self.state.active_template_context();
                let url = ctx.render(&tab.request.url);
                let metadata: Vec<(String, String)> = tab
                    .request
                    .headers
                    .iter()
                    .filter(|h| h.enabled)
                    .map(|h| (ctx.render(&h.key), ctx.render(&h.value)))
                    .collect();
                let verify_ssl = tab.request.settings.verify_ssl;
                self.state.status_message = Some(format!("Reflecting gRPC services on {url}..."));
                cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                    let mut cx = cx.clone();
                    async move {
                        let res =
                            aura_engine::reflect_grpc_services(&url, &metadata, verify_ssl).await;
                        let _ = this.update(&mut cx, |this, cx| {
                            match res {
                                Ok(schema) => {
                                    let svc_count = schema.services.len();
                                    this.state.apply_proto_schema(&tab_id, None, None, schema);
                                    this.state.status_message = Some(format!(
                                        "Reflected {svc_count} gRPC service(s) from server"
                                    ));
                                }
                                Err(e) => {
                                    this.state.status_message =
                                        Some(format!("gRPC reflection failed: {e}"));
                                }
                            }
                            cx.notify();
                        });
                    }
                })
                .detach();
            }
            RequestEditorAction::SelectGrpcService(svc) => {
                self.state.select_grpc_service(svc);
            }
            RequestEditorAction::SelectGrpcMethod(m) => {
                self.state.select_grpc_method(m);
            }
            RequestEditorAction::GenerateGrpcSample => {
                self.state.generate_grpc_sample_payload();
            }
            RequestEditorAction::WrapSoapEnvelope(ver) => {
                self.state.wrap_active_xml_as_soap(ver);
                self.state.status_message = Some("Wrapped body in SOAP envelope".to_string());
            }
            RequestEditorAction::OpenWsdlImport => {
                self.state.open_wsdl_import_dialog();
            }
        }
        if self.state.is_active_tab_dirty() {
            self.schedule_auto_save(cx);
        }
        cx.notify();
    }

    pub(super) fn spawn_pick_wsdl_file(&mut self, cx: &mut Context<Self>) {
        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let handle = rfd::AsyncFileDialog::new()
                    .set_title("Open WSDL Specification")
                    .add_filter("WSDL / XML", &["wsdl", "xml"])
                    .pick_file()
                    .await;
                if let Some(file) = handle {
                    if let Ok(xml) = tokio::fs::read_to_string(file.path()).await {
                        let _ = this.update(&mut cx, |this, cx| {
                            if let Some(ref mut dlg) = this.state.wsdl_import_dialog {
                                dlg.wsdl_text = xml;
                                dlg.error = None;
                            }
                            cx.notify();
                        });
                    }
                }
            }
        })
        .detach();
    }

}
