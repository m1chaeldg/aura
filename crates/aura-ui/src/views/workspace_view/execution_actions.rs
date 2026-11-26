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
        let cancel_token = execution_service.begin_flight();
        let tab_id = active_tab.id.clone();
        let request = active_tab.request.clone();
        let mut context = aura_core::templating::TemplateContext::default();
        context.request_vars = request.variables.clone();

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
                    .dispatch_with_token(request, context, cancel_token)
                    .await;
                let _ = this.update(&mut cx, |this, cx| {
                    match result {
                        Ok(resp) => {
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
                            if let Some(tab) = this.state.tabs.iter_mut().find(|t| t.id == tab_id) {
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

}
