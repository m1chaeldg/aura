use super::AuraWorkspaceView;
use gpui::*;

impl AuraWorkspaceView {
    /// Dispatches active HTTP request via execution service off the main thread.
    pub fn dispatch_send_request(&mut self, cx: &mut Context<Self>) {
        if self.state.execution_service.is_in_flight() {
            tracing::warn!(
                target: "aura_ui::execution",
                "Send ignored because a request is already in flight"
            );
            return;
        }

        let active_tab = match self.state.tabs.get(self.state.active_tab_index) {
            Some(t) => t.clone(),
            None => return,
        };
        let execution_service = self.state.execution_service.clone();
        let tab_id = active_tab.id.clone();
        let request = active_tab.request.clone();
        let context = self.state.active_template_context();
        let options = aura_engine::ExecutionOptions {
            domain_certs: self.state.domain_client_certs.clone(),
            vault_config: self.state.vault_config.clone(),
            global_proxy: Some(self.state.global_proxy.clone()),
            cookie_jar: self.state.cookie_jar.clone(),
            interceptor: Some(self.state.interceptor_manager.clone()),
        };

        if request.protocol == aura_core::model::RequestProtocol::Websocket {
            if self.state.is_active_ws_connected() {
                self.state.disconnect_ws_tab(&tab_id);
                self.state.status_message = Some("WebSocket disconnected".to_string());
                cx.notify();
            } else {
                self.dispatch_connect_websocket(cx);
            }
            return;
        }

        if request.protocol == aura_core::model::RequestProtocol::Sse {
            self.dispatch_sse_stream(tab_id, request, context, cx);
            return;
        }

        if request.protocol == aura_core::model::RequestProtocol::Grpc {
            self.dispatch_grpc_request(tab_id, request, context, cx);
            return;
        }

        let cancel_token = execution_service.begin_flight();

        tracing::info!(
            target: "aura_ui::execution",
            tab_id = %tab_id,
            method = %request.method,
            url = %request.url,
            "UI dispatched request; Send button switched to Cancel"
        );
        self.state.status_message = Some(format!("Sending {} {}...", request.method, request.url));
        cx.notify();

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let result = execution_service
                    .dispatch_with_options(request, context, options, cancel_token)
                    .await;
                let _ = this.update(&mut cx, |this, cx| {
                    match result {
                        Ok((updated_req, resp)) => {
                            let test_summary = if resp.assertions.is_empty() {
                                String::new()
                            } else {
                                let passed = resp.assertions.iter().filter(|a| a.passed).count();
                                format!(" • {passed}/{} Tests Passed", resp.assertions.len())
                            };
                            this.state.status_message = if let Some(first_err) = resp.script_errors.first() {
                                Some(format!(
                                    "⚠ {} error: {}",
                                    first_err.stage, first_err.message
                                ))
                            } else {
                                Some(format!(
                                    "Response: {} {} ({} ms, {}){}",
                                    resp.status,
                                    resp.status_text,
                                    resp.duration_ms,
                                    resp.formatted_size(),
                                    test_summary
                                ))
                            };
                            if !resp.captured_cookies.is_empty() {
                                this.state.ingest_captured_cookies(resp.captured_cookies.clone());
                            }
                            if let Some(tab) = this.state.tabs.iter_mut().find(|t| t.id == tab_id) {
                                if updated_req.auth != tab.request.auth {
                                    tab.request.auth = updated_req.auth;
                                    tab.mark_dirty(true);
                                }
                                tab.response = Some(resp);
                            }
                        }
                Err(e) => {
                            tracing::warn!(
                                target: "aura_ui::execution",
                                tab_id = %tab_id,
                                error = %e,
                                "Request did not complete"
                            );
                            this.state.status_message = Some(format!("Request failed: {e}"));
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub(super) fn dispatch_connect_websocket(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.state.active_tab().cloned() else {
            return;
        };
        let tab_id = tab.id.clone();
        let ctx = self.state.active_template_context();
        let url = ctx.render(&tab.request.url);
        let mut headers: Vec<(String, String)> = tab
            .request
            .headers
            .iter()
            .filter(|h| h.enabled)
            .map(|h| (ctx.render(&h.key), ctx.render(&h.value)))
            .collect();
        let mut dummy_params = Vec::new();
        aura_core::security::apply_auth(&tab.request.auth, &mut headers, &mut dummy_params);
        let verify_ssl = tab.request.settings.verify_ssl;

        self.state.status_message = Some(format!("Connecting WebSocket {url}..."));
        self.state.active_response_tab = crate::stores::ResponseTab::Stream;
        cx.notify();

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                match aura_engine::connect_websocket(&url, headers, verify_ssl).await {
                    Ok((cmd_tx, mut ev_rx)) => {
                        let _ = this.update(&mut cx, |this, cx| {
                            this.state.register_ws_connection(&tab_id, cmd_tx);
                            this.state.status_message = Some(format!("WebSocket connected: {url}"));
                            cx.notify();
                        });
                        while let Some(ev) = ev_rx.recv().await {
                            let tab_id_inner = tab_id.clone();
                            let cont = this.update(&mut cx, |this, cx| {
                                match ev {
                                    aura_engine::WsEvent::Message { payload, .. } => {
                                        this.state.push_stream_message(&tab_id_inner, "in", payload);
                                    }
                                    aura_engine::WsEvent::Closed { reason, .. } => {
                                        this.state.ws_senders.remove(&tab_id_inner);
                                        this.state.push_stream_message(
                                            &tab_id_inner,
                                            "system",
                                            format!("Closed: {reason}"),
                                        );
                                        this.state.status_message =
                                            Some(format!("WebSocket closed: {reason}"));
                                    }
                                    aura_engine::WsEvent::Error(err) => {
                                        this.state.push_stream_message(
                                            &tab_id_inner,
                                            "system",
                                            format!("Error: {err}"),
                                        );
                                    }
                                    aura_engine::WsEvent::Connected { .. } => {}
                                }
                                cx.notify();
                            });
                            if cont.is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = this.update(&mut cx, |this, cx| {
                            this.state.status_message =
                                Some(format!("WebSocket connect failed: {e}"));
                            cx.notify();
                        });
                    }
                }
            }
        })
        .detach();
    }

    fn dispatch_sse_stream(
        &mut self,
        tab_id: String,
        request: aura_core::model::AuraRequest,
        context: aura_core::templating::TemplateContext,
        cx: &mut Context<Self>,
    ) {
        let cancel_token = self.state.execution_service.begin_flight();
        self.state.status_message = Some(format!("Streaming SSE {}...", request.url));
        self.state.active_response_tab = crate::stores::ResponseTab::Stream;
        cx.notify();

        let (ev_tx, mut ev_rx) = tokio::sync::mpsc::unbounded_channel();
        let tab_id_pump = tab_id.clone();
        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                while let Some(ev) = ev_rx.recv().await {
                    let tid = tab_id_pump.clone();
                    if this
                        .update(&mut cx, |this, cx| {
                            this.state.push_sse_event(&tid, ev);
                            cx.notify();
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            }
        })
        .detach();

        let exec_svc = self.state.execution_service.clone();
        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let res = tokio::select! {
                    r = aura_engine::execute_sse_request(&request, &context, Some(ev_tx)) => r,
                    _ = cancel_token.cancelled() => Err(anyhow::anyhow!("SSE stream cancelled")),
                };
                exec_svc.end_flight();
                let _ = this.update(&mut cx, |this, cx| {
                    match res {
                        Ok(resp) => {
                            let ev_count = resp.sse_events.len();
                            this.state.status_message = Some(format!(
                                "SSE completed: {} {} ({ev_count} events, {} ms)",
                                resp.status, resp.status_text, resp.duration_ms
                            ));
                            if let Some(tab) = this.state.tabs.iter_mut().find(|t| t.id == tab_id) {
                                tab.response = Some(resp);
                            }
                        }
                        Err(e) => {
                            this.state.status_message = Some(format!("SSE failed: {e}"));
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn dispatch_grpc_request(
        &mut self,
        tab_id: String,
        request: aura_core::model::AuraRequest,
        context: aura_core::templating::TemplateContext,
        cx: &mut Context<Self>,
    ) {
        let cancel_token = self.state.execution_service.begin_flight();
        let schema = self.state.grpc_schemas.get(&tab_id).cloned();
        self.state.status_message = Some(format!("Calling gRPC {}...", request.url));
        self.state.active_response_tab = crate::stores::ResponseTab::Stream;
        cx.notify();

        let exec_svc = self.state.execution_service.clone();
        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let res = tokio::select! {
                    r = aura_engine::execute_grpc_call(&request, schema.as_ref(), &context, None) => r,
                    _ = cancel_token.cancelled() => Err(anyhow::anyhow!("gRPC call cancelled")),
                };
                exec_svc.end_flight();
                let _ = this.update(&mut cx, |this, cx| {
                    match res {
                        Ok(resp) => {
                            this.state.status_message = Some(format!(
                                "gRPC: {} ({} ms, {})",
                                resp.status_text,
                                resp.duration_ms,
                                resp.formatted_size()
                            ));
                            if let Some(tab) = this.state.tabs.iter_mut().find(|t| t.id == tab_id) {
                                tab.response = Some(resp);
                            }
                        }
                        Err(e) => {
                            this.state.status_message = Some(format!("gRPC call failed: {e}"));
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Schedules a debounced auto-save for the active request tab.
    pub fn schedule_auto_save(&mut self, cx: &mut Context<Self>) {
        if !self.state.auto_save_enabled {
            return;
        }
        let active_tab_id = match self.state.tabs.get(self.state.active_tab_index) {
            Some(t) if t.is_dirty => t.id.clone(),
            _ => return,
        };

        self.auto_save_seq += 1;
        let current_seq = self.auto_save_seq;
        let delay_ms = self.state.auto_save_delay_ms;

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                let _ = this.update(&mut cx, |this, cx| {
                    if this.auto_save_seq == current_seq {
                        this.dispatch_auto_save_tab(&active_tab_id, cx);
                    }
                });
            }
        })
        .detach();
    }

    /// Dispatches a save for the active tab (manual or keyboard triggered) and persists off the main thread.
    pub fn dispatch_save_active_tab(&mut self, is_auto_save: bool, cx: &mut Context<Self>) {
        let active_tab = match self.state.tabs.get(self.state.active_tab_index) {
            Some(t) => t.clone(),
            None => return,
        };

        if !active_tab.is_dirty {
            return;
        }

        // Cancel any pending debounced auto-save timers
        self.auto_save_seq += 1;

        if is_auto_save {
            self.state.auto_save_active_tab();
        } else {
            self.state.save_active_tab();
        }
        cx.notify();

        let srv = self.state.workspace_service.clone();
        let request = active_tab.request.clone();
        let file_path = active_tab.file_path.clone();
        let root = self.state.workspace_store.workspace_root.clone();
        let tab_id = active_tab.id.clone();

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                if let Some(path) = file_path {
                    let _ = srv.save_request_file(&path, &request).await;
                } else if let Some(r) = root {
                    let sanitized = request
                        .name
                        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
                    let file_name = if sanitized.ends_with(".aura") {
                        sanitized
                    } else {
                        format!("{sanitized}.aura")
                    };
                    let path = r.join(file_name);
                    if srv.save_request_file(&path, &request).await.is_ok() {
                        let _ = this.update(&mut cx, |this, cx| {
                            if let Some(tab) =
                                this.state.tabs.iter_mut().find(|t| t.id == tab_id)
                            {
                                if tab.file_path.is_none() {
                                    tab.file_path = Some(path);
                                }
                            }
                            let _ = this.state.reconcile_workspace_sync();
                            cx.notify();
                        });
                    }
                }
            }
        })
        .detach();
    }

    /// Dispatches auto-save for a specific tab ID off the main thread.
    pub fn dispatch_auto_save_tab(&mut self, tab_id: &str, cx: &mut Context<Self>) {
        let tab = match self.state.tabs.iter_mut().find(|t| t.id == tab_id) {
            Some(t) if t.is_dirty => {
                t.mark_dirty(false);
                self.state.status_message = Some(format!("Auto-saved: {}", t.title));
                t.clone()
            }
            _ => return,
        };

        cx.notify();

        let srv = self.state.workspace_service.clone();
        let request = tab.request.clone();
        let file_path = tab.file_path.clone();
        let root = self.state.workspace_store.workspace_root.clone();
        let tab_id_clone = tab_id.to_string();

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                if let Some(path) = file_path {
                    let _ = srv.save_request_file(&path, &request).await;
                } else if let Some(r) = root {
                    let sanitized = request
                        .name
                        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
                    let file_name = if sanitized.ends_with(".aura") {
                        sanitized
                    } else {
                        format!("{sanitized}.aura")
                    };
                    let path = r.join(file_name);
                    if srv.save_request_file(&path, &request).await.is_ok() {
                        let _ = this.update(&mut cx, |this, cx| {
                            if let Some(tab) =
                                this.state.tabs.iter_mut().find(|t| t.id == tab_id_clone)
                            {
                                if tab.file_path.is_none() {
                                    tab.file_path = Some(path);
                                }
                            }
                            let _ = this.state.reconcile_workspace_sync();
                            cx.notify();
                        });
                    }
                }
            }
        })
        .detach();
    }

    /// Launches interactive or headless OAuth 2.0 token acquisition on a background task
    /// and writes the obtained access_token / refresh_token / expires_at back into the active tab.
    pub(super) fn dispatch_oauth2_flow(&mut self, cx: &mut Context<Self>) {
        if self.state.is_oauth2_in_flight {
            self.state.status_message =
                Some("OAuth 2.0 authorization flow is already running...".to_string());
            cx.notify();
            return;
        }
        let Some(tab) = self.state.active_tab().cloned() else {
            return;
        };
        let aura_core::model::AuthConfig::OAuth2 {
            grant_type,
            token_url,
            client_id,
            client_secret,
            auth_url,
            redirect_uri,
            scope,
            username,
            password,
            use_pkce,
            ..
        } = &tab.request.auth
        else {
            return;
        };

        let ctx = self.state.active_template_context();
        let tab_id = tab.id.clone();
        let grant_type = ctx.render(grant_type);
        let auth_url = auth_url.as_ref().map(|s| ctx.render(s));
        let token_url = token_url.as_ref().map(|s| ctx.render(s));
        let client_id = client_id.as_ref().map(|s| ctx.render(s));
        let client_secret = client_secret.as_ref().map(|s| ctx.render(s));
        let redirect_uri = redirect_uri.as_ref().map(|s| ctx.render(s));
        let scope = scope.as_ref().map(|s| ctx.render(s));
        let username = username.as_ref().map(|s| ctx.render(s));
        let password = password.as_ref().map(|s| ctx.render(s));
        let use_pkce = *use_pkce;
        let verify_ssl = tab.request.settings.verify_ssl;

        self.state.is_oauth2_in_flight = true;
        self.state.status_message = Some(format!(
            "Acquiring OAuth 2.0 token ({grant_type})..."
        ));
        cx.notify();

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let res = aura_engine::run_oauth2_token_acquisition(
                    &grant_type,
                    auth_url.as_deref(),
                    token_url.as_deref(),
                    client_id.as_deref(),
                    client_secret.as_deref(),
                    redirect_uri.as_deref(),
                    scope.as_deref(),
                    username.as_deref(),
                    password.as_deref(),
                    use_pkce,
                    verify_ssl,
                )
                .await;

                let _ = this.update(&mut cx, |this, cx| {
                    this.state.is_oauth2_in_flight = false;
                    match res {
                        Ok(token_res) => {
                            this.state.apply_oauth2_token_result(&tab_id, token_res);
                            this.state.status_message =
                                Some("OAuth 2.0 access token updated".to_string());
                        }
                        Err(e) => {
                            this.state.status_message =
                                Some(format!("OAuth 2.0 flow failed: {e}"));
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub(super) fn dispatch_toggle_capture_proxy(&mut self, cx: &mut Context<Self>) {
        if let Some(handle) = self.state.capture_proxy_handle.take() {
            handle.stop();
            self.state.status_message = Some("Capture proxy stopped".to_string());
            cx.notify();
            return;
        }

        let bind_addr = if self.state.interceptor_bind_addr.trim().is_empty() {
            "127.0.0.1:8888".to_string()
        } else {
            self.state.interceptor_bind_addr.trim().to_string()
        };
        let mgr = self.state.interceptor_manager.clone();
        self.state.status_message = Some(format!("Starting capture proxy on {bind_addr}..."));
        cx.notify();

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let res = aura_engine::start_capture_proxy(&bind_addr, mgr).await;
                let _ = this.update(&mut cx, |this, cx| {
                    match res {
                        Ok(handle) => {
                            let addr = handle.listen_addr.clone();
                            this.state.capture_proxy_handle = Some(handle);
                            this.state.status_message = Some(format!(
                                "MITM capture proxy listening on {addr}"
                            ));
                        }
                        Err(e) => {
                            this.state.status_message =
                                Some(format!("Failed to start capture proxy: {e}"));
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }
}
