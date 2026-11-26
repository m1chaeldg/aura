//! Modular GPUI Desktop Workspace View coordinating single-responsibility sub-views.

use crate::app_state::{AuraAppState, EditorDialogState};
use crate::input_registry::{InputKey, InputRegistry};
use crate::stores::LayoutOrientation;
use crate::views::{
    CommandPaletteView, ContextMenuView, LogViewerView, RequestEditorAction, RequestEditorView,
    RequestHeaderView, ResponsePaneView, SidebarMenuAction, SidebarView, SplitResizerView,
    TabBarView,
};

use aura_core::model::{AuthConfig, BodyType, HttpMethod};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::input::{Input, InputEvent, InputState, TextareaState};

pub struct AuraWorkspaceView {
    pub state: AuraAppState,
    pub inputs: InputRegistry,
    pub focus_handle: Option<FocusHandle>,
    #[allow(dead_code)]
    watcher_cancel: Option<tokio_util::sync::CancellationToken>,
    pub auto_save_seq: u64,
    pub divider_drag_start: Option<(f32, f32, f32)>,
}

impl AuraWorkspaceView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut view = Self {
            state: AuraAppState::new(),
            inputs: InputRegistry::new(),
            focus_handle: Some(cx.focus_handle()),
            watcher_cancel: None,
            auto_save_seq: 0,
            divider_drag_start: None,
        };
        let prefs = crate::preferences::AuraPreferences::load();
        view.state.auto_save_enabled = prefs.auto_save_enabled;
        view.state.auto_save_delay_ms = prefs.auto_save_delay_ms;
        view.state.set_split_ratio(prefs.split_ratio);
        if prefs.layout_orientation.eq_ignore_ascii_case("vertical") {
            view.state.orientation = LayoutOrientation::Vertical;
        } else {
            view.state.orientation = LayoutOrientation::Horizontal;
        }
        if let Some(ref path) = prefs.last_workspace_path {
            if path.exists() && path.is_dir() && view.state.open_workspace_sync(path).is_ok() {
                view.start_workspace_watcher(path.clone(), cx);
            }
        }

        if view.state.workspace_store.workspace_root.is_none() {
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            let _ = view.state.load_workspace_collections(&cwd);
            let aura_dir = cwd.join(".aura");
            if aura_dir.exists() {
                let _ = view.state.load_workspace_collections(&aura_dir);
            }
            let collections_dir = cwd.join("collections");
            if collections_dir.exists() {
                let _ = view.state.load_workspace_collections(&collections_dir);
            }
        }

        if view.state.tabs.is_empty() {
            view.state.new_request();
        }
        view
    }

    /// Spawns background filesystem watcher and wires debounced sync updates to GPUI context.
    pub fn start_workspace_watcher(&mut self, root: std::path::PathBuf, cx: &mut Context<Self>) {
        if let Some(cancel) = self.watcher_cancel.take() {
            cancel.cancel();
        }

        let watcher_res = self.state.workspace_service.start_watcher(&root);
        if let Ok((cancel_token, mut rx)) = watcher_res {
            self.watcher_cancel = Some(cancel_token);

            cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    while let Some(_event) = rx.recv().await {
                        let update_res = this.update(&mut cx, |this, cx| {
                            if let Err(e) = this.state.reconcile_workspace_sync() {
                                tracing::error!(target: "aura_ui::workspace", "Failed to reconcile workspace: {e}");
                            }
                            cx.notify();
                        });

                        if update_res.is_err() {
                            break;
                        }
                    }
                }
            })
            .detach();
        }
    }

    /// Persists layout split ratio, orientation, auto-save settings, and last workspace to preferences file.
    pub fn save_preferences_sync(&self) {
        let mut prefs = crate::preferences::AuraPreferences::load();
        prefs.split_ratio = self.state.split_ratio;
        prefs.layout_orientation = match self.state.orientation {
            LayoutOrientation::Horizontal => "horizontal".to_string(),
            LayoutOrientation::Vertical => "vertical".to_string(),
        };
        prefs.auto_save_enabled = self.state.auto_save_enabled;
        prefs.auto_save_delay_ms = self.state.auto_save_delay_ms;
        if let Some(ref root) = self.state.workspace_store.workspace_root {
            prefs.last_workspace_path = Some(root.clone());
        }
        let _ = prefs.save();
    }

    /// Dispatches active HTTP request via execution service off the main thread.
    pub fn dispatch_send_request(&mut self, cx: &mut Context<Self>) {
        let active_tab = match self.state.tabs.get(self.state.active_tab_index) {
            Some(t) => t.clone(),
            None => return,
        };
        let execution_service = self.state.execution_service.clone();
        let tab_id = active_tab.id.clone();
        let request = active_tab.request.clone();
        let context = aura_core::templating::TemplateContext::default();

        self.state.status_message = Some(format!("Sending {} {}...", request.method, request.url));
        cx.notify();

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let result = execution_service.dispatch(request, context).await;
                let _ = this.update(&mut cx, |this, cx| {
                    match result {
                        Ok(resp) => {
                            this.state.status_message = Some(format!(
                                "Response: {} {} ({} ms, {})",
                                resp.status,
                                resp.status_text,
                                resp.duration_ms,
                                resp.formatted_size()
                            ));
                            if let Some(tab) = this.state.tabs.iter_mut().find(|t| t.id == tab_id) {
                                tab.response = Some(resp);
                            }
                        }
                        Err(e) => {
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

    /// Confirms pending item deletion off the main thread and syncs workspace.
    pub fn confirm_delete_action(&mut self, cx: &mut Context<Self>) {
        let mut state_clone = self.state.clone();
        self.state.pending_delete_path = None;
        cx.notify();

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                if let Err(e) = state_clone.confirm_delete().await {
                    tracing::error!(target: "aura_ui::workspace", "Failed to confirm delete: {e}");
                }
                let _ = this.update(&mut cx, |this, cx| {
                    if let Err(e) = this.state.reconcile_workspace_sync() {
                        tracing::error!(target: "aura_ui::workspace", "Failed to reconcile workspace: {e}");
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Confirms pending item renaming off the main thread and syncs workspace.
    pub fn confirm_rename_action(&mut self, cx: &mut Context<Self>) {
        let mut state_clone = self.state.clone();
        self.state.rename_dialog = None;
        cx.notify();

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                match state_clone.confirm_rename().await {
                    Ok(Some(new_path)) => {
                        let _ = this.update(&mut cx, |this, cx| {
                            if let Err(e) = this.state.reconcile_workspace_sync() {
                                tracing::error!(target: "aura_ui::workspace", "Failed to reconcile workspace: {e}");
                            }
                            this.state.workspace_store.select_node(new_path);
                            cx.notify();
                        });
                    }
                    Ok(None) => {}
                    Err(e) => {
                        tracing::error!(target: "aura_ui::workspace", "Failed to confirm rename: {e}");
                    }
                }
            }
        })
        .detach();
    }

    /// Confirms pending Move To dialog action and relocates item.
    pub fn confirm_move_to_action(&mut self, cx: &mut Context<Self>) {
        if let Some(dialog) = self.state.move_to_dialog.take() {
            if let Some(dest) = dialog.selected_destination() {
                let source = dialog.source_path.clone();
                let target_parent = dest.path.clone();
                self.handle_drop_item(
                    source,
                    target_parent,
                    crate::stores::workspace_store::DragPlacement::Inside,
                    cx,
                );
            }
        }
    }

    /// Handles actions dispatched from the sidebar context menu.
    pub fn handle_sidebar_menu_action(
        &mut self,
        action: SidebarMenuAction,
        cx: &mut Context<Self>,
    ) {
        let menu = match self.state.workspace_store.active_context_menu.take() {
            Some(m) => m,
            None => return,
        };
        let target = menu.target_path;
        let is_folder = menu.is_folder;

        match action {
            SidebarMenuAction::NewRequest => {
                let parent = if is_folder {
                    target
                } else {
                    target.parent().unwrap_or(&target).to_path_buf()
                };
                let root = self.state.workspace_store.workspace_root.clone();
                let srv = self.state.workspace_service.clone();
                cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                    let mut cx = cx.clone();
                    async move {
                        if let Some(r) = root {
                            if let Ok(p) = srv
                                .create_request(&r, &parent, "New Request", HttpMethod::GET)
                                .await
                            {
                                let _ = this.update(&mut cx, |this, cx| {
                                    let _ = this.state.open_file_in_tab(&p);
                                    let _ = this.state.reconcile_workspace_sync();
                                    cx.notify();
                                });
                            }
                        }
                    }
                })
                .detach();
            }
            SidebarMenuAction::NewFolder => {
                let parent = if is_folder {
                    target
                } else {
                    target.parent().unwrap_or(&target).to_path_buf()
                };
                let root = self.state.workspace_store.workspace_root.clone();
                let srv = self.state.workspace_service.clone();
                cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                    let mut cx = cx.clone();
                    async move {
                        if let Some(r) = root {
                            let _ = srv.create_folder(&r, &parent, "New Folder").await;
                            let _ = this.update(&mut cx, |this, cx| {
                                let _ = this.state.reconcile_workspace_sync();
                                cx.notify();
                            });
                        }
                    }
                })
                .detach();
            }
            SidebarMenuAction::Duplicate => {
                let root = self.state.workspace_store.workspace_root.clone();
                let srv = self.state.workspace_service.clone();
                cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                    let mut cx = cx.clone();
                    async move {
                        if let Some(r) = root {
                            if let Ok(p) = srv.duplicate_item(&r, &target).await {
                                let _ = this.update(&mut cx, |this, cx| {
                                    if p.is_file() {
                                        let _ = this.state.open_file_in_tab(&p);
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
            SidebarMenuAction::Copy => {
                self.state.workspace_store.select_node(target);
                self.state.workspace_store.copy_selected();
                cx.notify();
            }
            SidebarMenuAction::Paste => {
                if let Some(payload) = self.state.workspace_store.clipboard_item.clone() {
                    let root = self.state.workspace_store.workspace_root.clone();
                    let srv = self.state.workspace_service.clone();
                    let target_parent = if is_folder {
                        target
                    } else {
                        target.parent().unwrap_or(&target).to_path_buf()
                    };
                    cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                        let mut cx = cx.clone();
                        async move {
                            if let Some(r) = root {
                                let _ = srv.paste_item(&r, &payload, &target_parent).await;
                                let _ = this.update(&mut cx, |this, cx| {
                                    let _ = this.state.reconcile_workspace_sync();
                                    cx.notify();
                                });
                            }
                        }
                    })
                    .detach();
                }
            }
            SidebarMenuAction::Rename => {
                let name = target
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                self.state.request_rename(target.clone(), is_folder);
                self.state.start_inline_rename(target, name);
                cx.notify();
            }

            SidebarMenuAction::MoveTo => {
                self.state.open_move_to_dialog(target);
                cx.notify();
            }

            SidebarMenuAction::Delete => {
                self.state.request_delete(target);
                cx.notify();
            }
        }
    }

    pub fn handle_drop_item(
        &mut self,
        dragging: std::path::PathBuf,
        target: std::path::PathBuf,
        placement: crate::stores::workspace_store::DragPlacement,
        cx: &mut Context<Self>,
    ) {
        if let Some(root) = self.state.workspace_store.workspace_root.clone() {
            let srv = self.state.workspace_service.clone();
            let target_parent = match placement {
                crate::stores::workspace_store::DragPlacement::Inside => target,
                _ => target.parent().unwrap_or(&root).to_path_buf(),
            };
            self.state.workspace_store.set_drag_state(None);
            cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    if let Ok(()) = srv.reorder_item(&root, &dragging, &target_parent, 0).await {
                        let _ = this.update(&mut cx, |this, cx| {
                            let _ = this.state.reconcile_workspace_sync();
                            cx.notify();
                        });
                    }
                }
            })
            .detach();
        }
    }

    pub fn handle_request_editor_action(
        &mut self,
        action: RequestEditorAction,
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
        }
        if self.state.is_active_tab_dirty() {
            self.schedule_auto_save(cx);
        }
        cx.notify();
    }

    fn render_editor_dialog(
        dialog: &EditorDialogState,
        inputs: &InputRegistry,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (title, subtitle, width, body_el) = match dialog {
            EditorDialogState::AddHeader {
                key,
                value,
                is_value_focused,
            } => {
                let is_val = *is_value_focused;
                (
                    "Add Request Header",
                    "Enter header key and value. Press Tab to switch fields, Enter to confirm.",
                    460.0,
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(Self::render_dialog_input_row(
                            "Key / Header Name",
                            inputs.get_input(&InputKey::EditorDialogField(0)),
                            key,
                            !is_val,
                            "e.g. Content-Type",
                        ))
                        .child(Self::render_dialog_input_row(
                            "Header Value",
                            inputs.get_input(&InputKey::EditorDialogField(1)),
                            value,
                            is_val,
                            "e.g. application/json",
                        )),
                )
            }
            EditorDialogState::EditHeader {
                key,
                value,
                is_value_focused,
                ..
            } => {
                let is_val = *is_value_focused;
                (
                    "Edit Request Header",
                    "Modify header key and value. Press Tab to switch fields, Enter to confirm.",
                    460.0,
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(Self::render_dialog_input_row(
                            "Key / Header Name",
                            inputs.get_input(&InputKey::EditorDialogField(0)),
                            key,
                            !is_val,
                            "Header Name",
                        ))
                        .child(Self::render_dialog_input_row(
                            "Header Value",
                            inputs.get_input(&InputKey::EditorDialogField(1)),
                            value,
                            is_val,
                            "Header Value",
                        )),
                )
            }
            EditorDialogState::AddParam {
                key,
                value,
                is_value_focused,
            } => {
                let is_val = *is_value_focused;
                (
                    "Add Query Parameter",
                    "Enter parameter key and value to append to URL.",
                    460.0,
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(Self::render_dialog_input_row(
                            "Parameter Key",
                            inputs.get_input(&InputKey::EditorDialogField(0)),
                            key,
                            !is_val,
                            "e.g. page",
                        ))
                        .child(Self::render_dialog_input_row(
                            "Parameter Value",
                            inputs.get_input(&InputKey::EditorDialogField(1)),
                            value,
                            is_val,
                            "e.g. 1",
                        )),
                )
            }
            EditorDialogState::EditParam {
                key,
                value,
                is_value_focused,
                ..
            } => {
                let is_val = *is_value_focused;
                (
                    "Edit Query Parameter",
                    "Modify parameter key and value.",
                    460.0,
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(Self::render_dialog_input_row(
                            "Parameter Key",
                            inputs.get_input(&InputKey::EditorDialogField(0)),
                            key,
                            !is_val,
                            "Key",
                        ))
                        .child(Self::render_dialog_input_row(
                            "Parameter Value",
                            inputs.get_input(&InputKey::EditorDialogField(1)),
                            value,
                            is_val,
                            "Value",
                        )),
                )
            }
            EditorDialogState::EditBody { content } => (
                "Edit Request Body",
                "Type or paste your JSON, XML, or raw body payload. Enter confirms.",
                520.0,
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Self::render_dialog_input_row(
                        "Body Content",
                        inputs.get_input(&InputKey::EditorDialogField(0)),
                        content,
                        true,
                        "{\n  \"key\": \"value\"\n}",
                    )),
            ),
            EditorDialogState::EditBearerAuth { token } => (
                "Edit Bearer Token",
                "Enter access token or JWT. Format: Bearer <token>",
                480.0,
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Self::render_dialog_input_row(
                        "Token",
                        inputs.get_input(&InputKey::EditorDialogField(0)),
                        token,
                        true,
                        "eyJhbGciOi...",
                    )),
            ),
            EditorDialogState::EditBasicAuth {
                username,
                password,
                is_password_focused,
            } => {
                let is_pw = *is_password_focused;
                (
                    "Edit Basic Auth Credentials",
                    "Enter username and password for HTTP Basic Authentication.",
                    460.0,
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(Self::render_dialog_input_row(
                            "Username",
                            inputs.get_input(&InputKey::EditorDialogField(0)),
                            username,
                            !is_pw,
                            "Username",
                        ))
                        .child(Self::render_dialog_input_row(
                            "Password",
                            inputs.get_input(&InputKey::EditorDialogField(1)),
                            password,
                            is_pw,
                            "Password",
                        )),
                )
            }
            EditorDialogState::EditApiKeyAuth {
                key,
                value,
                in_header,
                is_value_focused,
            } => {
                let is_val = *is_value_focused;
                let in_hdr = *in_header;
                (
                    "Edit API Key Authentication",
                    "Enter the API key header/query name and secret value.",
                    480.0,
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(Self::render_dialog_input_row(
                            "Key Name",
                            inputs.get_input(&InputKey::EditorDialogField(0)),
                            key,
                            !is_val,
                            "e.g. X-API-Key",
                        ))
                        .child(Self::render_dialog_input_row(
                            "Key Value",
                            inputs.get_input(&InputKey::EditorDialogField(1)),
                            value,
                            is_val,
                            "secret_key_123",
                        ))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0xaaaaaa))
                                        .child("Placement:"),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_0p5()
                                        .rounded_sm()
                                        .bg(if in_hdr { rgb(0x0066cc) } else { rgb(0x333333) })
                                        .text_xs()
                                        .text_color(rgb(0xffffff))
                                        .child("In Header"),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_0p5()
                                        .rounded_sm()
                                        .bg(if !in_hdr {
                                            rgb(0x0066cc)
                                        } else {
                                            rgb(0x333333)
                                        })
                                        .text_xs()
                                        .text_color(rgb(0xffffff))
                                        .child("In Query Params"),
                                ),
                        ),
                )
            }
            EditorDialogState::EditAwsSigV4 {
                access_key,
                secret_key,
                region,
                service,
                session_token,
                active_field,
            } => {
                let af = *active_field;
                (
                    "Edit AWS Signature V4 Auth",
                    "Enter AWS IAM credentials and region. Tab navigates fields.",
                    500.0,
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(Self::render_dialog_input_row(
                            "Access Key",
                            inputs.get_input(&InputKey::EditorDialogField(0)),
                            access_key,
                            af == 0,
                            "AKIA...",
                        ))
                        .child(Self::render_dialog_input_row(
                            "Secret Key",
                            inputs.get_input(&InputKey::EditorDialogField(1)),
                            secret_key,
                            af == 1,
                            "wJalrXUtnFEMI...",
                        ))
                        .child(Self::render_dialog_input_row(
                            "Region",
                            inputs.get_input(&InputKey::EditorDialogField(2)),
                            region,
                            af == 2,
                            "us-east-1",
                        ))
                        .child(Self::render_dialog_input_row(
                            "Service",
                            inputs.get_input(&InputKey::EditorDialogField(3)),
                            service,
                            af == 3,
                            "execute-api",
                        ))
                        .child(Self::render_dialog_input_row(
                            "Session Token (optional)",
                            inputs.get_input(&InputKey::EditorDialogField(4)),
                            session_token,
                            af == 4,
                            "AQoDYXdz...",
                        )),
                )
            }
            EditorDialogState::EditOAuth2 {
                grant_type,
                access_token,
                token_url,
                client_id,
                client_secret,
                active_field,
            } => {
                let af = *active_field;
                (
                    "Edit OAuth 2.0 Auth",
                    "Enter OAuth token and parameters. Tab navigates fields.",
                    500.0,
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(Self::render_dialog_input_row(
                            "Access Token",
                            inputs.get_input(&InputKey::EditorDialogField(0)),
                            access_token,
                            af == 0,
                            "ya29...",
                        ))
                        .child(Self::render_dialog_input_row(
                            "Grant Type",
                            inputs.get_input(&InputKey::EditorDialogField(1)),
                            grant_type,
                            af == 1,
                            "authorization_code",
                        ))
                        .child(Self::render_dialog_input_row(
                            "Token URL (optional)",
                            inputs.get_input(&InputKey::EditorDialogField(2)),
                            token_url,
                            af == 2,
                            "https://auth.example.com/token",
                        ))
                        .child(Self::render_dialog_input_row(
                            "Client ID (optional)",
                            inputs.get_input(&InputKey::EditorDialogField(3)),
                            client_id,
                            af == 3,
                            "client_123",
                        ))
                        .child(Self::render_dialog_input_row(
                            "Client Secret (optional)",
                            inputs.get_input(&InputKey::EditorDialogField(4)),
                            client_secret,
                            af == 4,
                            "secret_456",
                        )),
                )
            }
            EditorDialogState::AddFormParam {
                key,
                value,
                is_value_focused,
            } => {
                let is_val = *is_value_focused;
                (
                    "Add Form Parameter",
                    "Enter key and value for URL-encoded form data.",
                    460.0,
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(Self::render_dialog_input_row(
                            "Form Field Key",
                            inputs.get_input(&InputKey::EditorDialogField(0)),
                            key,
                            !is_val,
                            "key",
                        ))
                        .child(Self::render_dialog_input_row(
                            "Form Field Value",
                            inputs.get_input(&InputKey::EditorDialogField(1)),
                            value,
                            is_val,
                            "value",
                        )),
                )
            }
            EditorDialogState::AddMultipartField {
                name,
                value,
                file_path,
                is_file,
                active_field,
            } => {
                let af = *active_field;
                let is_f = *is_file;
                (
                    "Add Multipart Part",
                    "Enter multipart field name and text value or file path.",
                    480.0,
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(Self::render_dialog_input_row(
                            "Field Name",
                            inputs.get_input(&InputKey::EditorDialogField(0)),
                            name,
                            af == 0,
                            "name",
                        ))
                        .when(!is_f, |el| {
                            el.child(Self::render_dialog_input_row(
                                "Text Value",
                                inputs.get_input(&InputKey::EditorDialogField(1)),
                                value,
                                af == 1,
                                "value",
                            ))
                        })
                        .when(is_f, |el| {
                            el.child(Self::render_dialog_input_row(
                                "File Path",
                                inputs.get_input(&InputKey::EditorDialogField(2)),
                                file_path,
                                af == 2,
                                "path/to/file.bin",
                            ))
                        }),
                )
            }
        };

        div()
            .id("editor-dialog-overlay")
            .absolute()
            .inset_0()
            .bg(rgba(0x000000bb))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .id("editor-dialog-card")
                    .w(px(width))
                    .bg(rgb(0x272729))
                    .border_1()
                    .border_color(rgb(0x333333))
                    .rounded_md()
                    .p_5()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xffffff))
                                    .child(title),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x888888))
                                    .child("Esc to cancel • ↵ to save"),
                            ),
                    )
                    .child(div().text_xs().text_color(rgb(0xaaaaaa)).child(subtitle))
                    .child(body_el)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .mt_3()
                            .child(
                                div()
                                    .id("cancel-editor-dialog-btn")
                                    .px_3()
                                    .py_1()
                                    .bg(rgb(0x333333))
                                    .hover(|s| s.bg(rgb(0x444444)))
                                    .text_xs()
                                    .text_color(rgb(0xffffff))
                                    .rounded_sm()
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.state.cancel_editor_dialog();
                                        cx.notify();
                                    }))
                                    .child("Cancel"),
                            )
                            .child(
                                div()
                                    .id("confirm-editor-dialog-btn")
                                    .px_3()
                                    .py_1()
                                    .bg(rgb(0x0066cc))
                                    .hover(|s| s.bg(rgb(0x0071e3)))
                                    .text_xs()
                                    .text_color(rgb(0xffffff))
                                    .rounded_sm()
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.state.confirm_editor_dialog();
                                        cx.notify();
                                    }))
                                    .child("Save"),
                            ),
                    ),
            )
    }

    fn render_dialog_input_row(
        label: &'static str,
        input: Option<&Entity<InputState>>,
        val: &str,
        is_focused: bool,
        placeholder: &'static str,
    ) -> Div {
        let box_div = div()
            .flex()
            .items_center()
            .px_2()
            .py_1()
            .bg(rgb(0x1d1d1f))
            .border_1()
            .border_color(if is_focused {
                rgb(0x0066cc)
            } else {
                rgb(0x333333)
            })
            .rounded_sm();

        let field_el = if let Some(state) = input {
            box_div.child(
                Input::new(state)
                    .appearance(false)
                    .w_full()
                    .text_xs()
                    .text_color(rgb(0xffffff)),
            )
        } else {
            box_div
                .text_xs()
                .text_color(if val.is_empty() {
                    rgb(0x666666)
                } else {
                    rgb(0xffffff)
                })
                .child(if val.is_empty() {
                    placeholder.to_string()
                } else {
                    val.to_string()
                })
        };

        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(if is_focused {
                        rgb(0x2997ff)
                    } else {
                        rgb(0xaaaaaa)
                    })
                    .child(label),
            )
            .child(field_el)
    }
}

impl Default for AuraWorkspaceView {
    fn default() -> Self {
        Self {
            state: AuraAppState::new(),
            inputs: InputRegistry::new(),
            focus_handle: None,
            watcher_cancel: None,
            auto_save_seq: 0,
            divider_drag_start: None,
        }
    }
}

impl AuraWorkspaceView {
    fn handle_input_event(
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
                        _ => {}
                    }
                }
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

    fn handle_textarea_event(
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

    fn sync_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut inputs = std::mem::take(&mut self.inputs);

        let url_val = self
            .state
            .tabs
            .get(self.state.active_tab_index)
            .map(|t| t.request.url.clone())
            .unwrap_or_default();
        inputs.ensure_input(
            InputKey::Url,
            &url_val,
            "Enter request URL (e.g. https://postman-echo.com/get)...",
            false,
            window,
            cx,
            Self::handle_input_event,
        );

        let palette_q = self.state.command_palette_search.clone();
        inputs.ensure_input(
            InputKey::CommandPaletteSearch,
            &palette_q,
            "Search requests or type command... (Esc to dismiss)",
            false,
            window,
            cx,
            Self::handle_input_event,
        );

        if let Some(ref rd) = self.state.rename_dialog {
            let v = rd.current_name.clone();
            inputs.ensure_input(
                InputKey::RenameDialog,
                &v,
                "Enter name...",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
        }

        if let Some(ref md) = self.state.move_to_dialog {
            let v = md.search_query.clone();
            inputs.ensure_input(
                InputKey::MoveToSearch,
                &v,
                "Type to filter destination folders...",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
        }

        if let Some(ref ir) = self.state.inline_rename {
            let v = ir.current_value.clone();
            inputs.ensure_input(
                InputKey::InlineRename,
                &v,
                "Enter name...",
                false,
                window,
                cx,
                Self::handle_input_event,
            );
        }

        if let Some(ref dlg) = self.state.editor_dialog {
            let fields: Vec<(usize, String, &'static str)> = match dlg {
                EditorDialogState::AddHeader { key, value, .. }
                | EditorDialogState::EditHeader { key, value, .. } => vec![
                    (0, key.clone(), "Header Name"),
                    (1, value.clone(), "Header Value"),
                ],
                EditorDialogState::AddParam { key, value, .. }
                | EditorDialogState::EditParam { key, value, .. } => {
                    vec![(0, key.clone(), "Key"), (1, value.clone(), "Value")]
                }
                EditorDialogState::EditBody { content } => {
                    vec![(0, content.clone(), "{\n  \"key\": \"value\"\n}")]
                }
                EditorDialogState::EditBearerAuth { token } => {
                    vec![(0, token.clone(), "eyJhbGciOi...")]
                }
                EditorDialogState::EditBasicAuth {
                    username, password, ..
                } => vec![
                    (0, username.clone(), "Username"),
                    (1, password.clone(), "Password"),
                ],
                EditorDialogState::EditApiKeyAuth { key, value, .. } => vec![
                    (0, key.clone(), "e.g. X-API-Key"),
                    (1, value.clone(), "secret_key_123"),
                ],
                EditorDialogState::EditAwsSigV4 {
                    access_key,
                    secret_key,
                    region,
                    service,
                    session_token,
                    ..
                } => vec![
                    (0, access_key.clone(), "AKIA..."),
                    (1, secret_key.clone(), "wJalrXUtnFEMI..."),
                    (2, region.clone(), "us-east-1"),
                    (3, service.clone(), "execute-api"),
                    (4, session_token.clone(), "AQoDYXdz..."),
                ],
                EditorDialogState::EditOAuth2 {
                    access_token,
                    grant_type,
                    token_url,
                    client_id,
                    client_secret,
                    ..
                } => vec![
                    (0, access_token.clone(), "ya29..."),
                    (1, grant_type.clone(), "authorization_code"),
                    (2, token_url.clone(), "https://auth.example.com/token"),
                    (3, client_id.clone(), "client_123"),
                    (4, client_secret.clone(), "secret_456"),
                ],
                EditorDialogState::AddFormParam { key, value, .. } => {
                    vec![(0, key.clone(), "key"), (1, value.clone(), "value")]
                }
                EditorDialogState::AddMultipartField {
                    name,
                    value,
                    file_path,
                    ..
                } => vec![
                    (0, name.clone(), "name"),
                    (1, value.clone(), "value"),
                    (2, file_path.clone(), "path/to/file.bin"),
                ],
            };
            for (idx, val, ph) in fields {
                inputs.ensure_input(
                    InputKey::EditorDialogField(idx),
                    &val,
                    ph,
                    false,
                    window,
                    cx,
                    Self::handle_input_event,
                );
            }
        }

        if let Some(tab) = self.state.tabs.get(self.state.active_tab_index).cloned() {
            for (idx, p) in tab.request.params.iter().enumerate() {
                inputs.ensure_input(
                    InputKey::ParamKey(idx),
                    &p.key,
                    "Key",
                    false,
                    window,
                    cx,
                    Self::handle_input_event,
                );
                inputs.ensure_input(
                    InputKey::ParamValue(idx),
                    &p.value,
                    "Value",
                    false,
                    window,
                    cx,
                    Self::handle_input_event,
                );
            }

            for (idx, h) in tab.request.headers.iter().enumerate() {
                inputs.ensure_input(
                    InputKey::HeaderKey(idx),
                    &h.key,
                    "Header Name",
                    false,
                    window,
                    cx,
                    Self::handle_input_event,
                );
                inputs.ensure_input(
                    InputKey::HeaderValue(idx),
                    &h.value,
                    "Header Value",
                    false,
                    window,
                    cx,
                    Self::handle_input_event,
                );
            }

            match &tab.request.body {
                BodyType::Json(s) | BodyType::Xml(s) | BodyType::Text(s) => {
                    inputs.ensure_textarea(
                        InputKey::BodyText,
                        s,
                        "// Click or type to edit request body...",
                        window,
                        cx,
                        Self::handle_textarea_event,
                    );
                }
                BodyType::Raw { content, .. } => {
                    inputs.ensure_textarea(
                        InputKey::BodyText,
                        content,
                        "// Click or type to edit request body...",
                        window,
                        cx,
                        Self::handle_textarea_event,
                    );
                }
                BodyType::FormUrlEncoded(items) => {
                    for (idx, item) in items.iter().enumerate() {
                        inputs.ensure_input(
                            InputKey::FormParamKey(idx),
                            &item.key,
                            "Field Key",
                            false,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                        inputs.ensure_input(
                            InputKey::FormParamValue(idx),
                            &item.value,
                            "Field Value",
                            false,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                    }
                }
                BodyType::Form(map) => {
                    for (idx, (k, v)) in map.iter().enumerate() {
                        inputs.ensure_input(
                            InputKey::FormParamKey(idx),
                            k,
                            "Field Key",
                            false,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                        inputs.ensure_input(
                            InputKey::FormParamValue(idx),
                            v,
                            "Field Value",
                            false,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                    }
                }
                _ => {}
            }

            match &tab.request.auth {
                AuthConfig::Bearer { token } => {
                    inputs.ensure_input(
                        InputKey::AuthBearerToken,
                        token,
                        "Enter Bearer token or paste JWT here...",
                        false,
                        window,
                        cx,
                        Self::handle_input_event,
                    );
                }
                AuthConfig::Basic { username, password } => {
                    inputs.ensure_input(
                        InputKey::AuthBasicUsername,
                        username,
                        "Enter username...",
                        false,
                        window,
                        cx,
                        Self::handle_input_event,
                    );
                    inputs.ensure_input(
                        InputKey::AuthBasicPassword,
                        password,
                        "Enter password...",
                        true,
                        window,
                        cx,
                        Self::handle_input_event,
                    );
                }
                AuthConfig::ApiKey { key, value, .. } => {
                    inputs.ensure_input(
                        InputKey::AuthApiKeyName,
                        key,
                        "e.g. X-API-Key",
                        false,
                        window,
                        cx,
                        Self::handle_input_event,
                    );
                    inputs.ensure_input(
                        InputKey::AuthApiKeyValue,
                        value,
                        "secret_key_123...",
                        false,
                        window,
                        cx,
                        Self::handle_input_event,
                    );
                }
                AuthConfig::AwsSigV4 {
                    access_key,
                    secret_key,
                    region,
                    service,
                    session_token,
                } => {
                    let fields = [
                        (0, access_key.as_str(), "Enter Access Key..."),
                        (1, secret_key.as_str(), "Enter Secret Key..."),
                        (2, region.as_str(), "Enter Region..."),
                        (3, service.as_str(), "Enter Service..."),
                        (4, session_token.as_deref().unwrap_or(""), "Enter Session Token..."),
                    ];
                    for (idx, val, ph) in fields {
                        inputs.ensure_input(
                            InputKey::AuthAwsField(idx),
                            val,
                            ph,
                            false,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                    }
                }
                AuthConfig::OAuth2 {
                    grant_type,
                    access_token,
                    token_url,
                    client_id,
                    client_secret,
                } => {
                    let fields = [
                        (0, access_token.as_str(), "Enter Access Token..."),
                        (1, grant_type.as_str(), "Enter Grant Type..."),
                        (2, token_url.as_deref().unwrap_or(""), "Enter Token URL..."),
                        (3, client_id.as_deref().unwrap_or(""), "Enter Client ID..."),
                        (4, client_secret.as_deref().unwrap_or(""), "Enter Client Secret..."),
                    ];
                    for (idx, val, ph) in fields {
                        inputs.ensure_input(
                            InputKey::AuthOAuth2Field(idx),
                            val,
                            ph,
                            false,
                            window,
                            cx,
                            Self::handle_input_event,
                        );
                    }
                }
                AuthConfig::None | AuthConfig::Inherit => {}
            }

            inputs.ensure_textarea(
                InputKey::PreRequestScript,
                &tab.request.scripts.pre_request,
                "// Write pre-request script here...",
                window,
                cx,
                Self::handle_textarea_event,
            );
            inputs.ensure_textarea(
                InputKey::PostResponseScript,
                &tab.request.scripts.post_response,
                "// Write post-response script here...",
                window,
                cx,
                Self::handle_textarea_event,
            );
            inputs.ensure_textarea(
                InputKey::TestScript,
                &tab.request.scripts.tests,
                "// Write Jasmine/Chai assertions...",
                window,
                cx,
                Self::handle_textarea_event,
            );

            let spec_val = tab
                .spec_buffer
                .clone()
                .unwrap_or_else(|| aura_parser::request_to_spec(&tab.request));
            inputs.ensure_textarea(
                InputKey::SpecText,
                &spec_val,
                "// Empty .aura specification...",
                window,
                cx,
                Self::handle_textarea_event,
            );

            if let Some(ref resp) = tab.response {
                let body_formatted =
                    ResponsePaneView::format_body(&resp.body, self.state.active_body_mode);
                let body_display = if body_formatted.is_empty() {
                    "// Response body is empty".to_string()
                } else {
                    body_formatted
                };
                inputs.ensure_textarea(
                    InputKey::ResponseBody,
                    &body_display,
                    "",
                    window,
                    cx,
                    Self::handle_textarea_event,
                );

                let mut raw_http = format!("HTTP/1.1 {} {}\n", resp.status, resp.status_text);
                let mut header_entries: Vec<_> = resp.headers.iter().collect();
                header_entries.sort_by_key(|(k, _)| (*k).clone());
                for (k, v) in header_entries {
                    raw_http.push_str(&format!("{}: {}\n", k, v));
                }
                raw_http.push('\n');
                raw_http.push_str(&resp.body);
                inputs.ensure_textarea(
                    InputKey::ResponseRaw,
                    &raw_http,
                    "",
                    window,
                    cx,
                    Self::handle_textarea_event,
                );

                let json_pretty = match serde_json::from_str::<serde_json::Value>(&resp.body) {
                    Ok(val) => {
                        serde_json::to_string_pretty(&val).unwrap_or_else(|_| resp.body.clone())
                    }
                    Err(_) => {
                        "/* Response body is not valid JSON */\n\n".to_string() + &resp.body
                    }
                };
                inputs.ensure_textarea(
                    InputKey::ResponseJson,
                    &json_pretty,
                    "",
                    window,
                    cx,
                    Self::handle_textarea_event,
                );
            }
        }

        self.inputs = inputs;
    }
}

impl Render for AuraWorkspaceView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_inputs(window, cx);
        let active_tab = self.state.tabs.get(self.state.active_tab_index);
        let active_title = active_tab
            .map(|t| t.title.clone())
            .unwrap_or_else(|| "Untitled Request".to_string());
        let active_method = active_tab.map(|t| t.method).unwrap_or(HttpMethod::GET);

        let bg_color = rgb(0x1d1d1f);
        let surface_color = rgb(0x272729);
        let surface_tile_2 = rgb(0x2a2a2c);
        let text_color = rgb(0xf5f5f7);
        let text_muted = rgb(0x7a7a7a);
        let border_color = rgb(0x333333);
        let accent_blue = rgb(0x0066cc);
        let is_horizontal = self.state.orientation == LayoutOrientation::Horizontal;
        let split_pct = (self.state.split_ratio * 100.0).round() as u32;
        let is_in_flight = self.state.execution_service.is_in_flight();

        let focus = self
            .focus_handle
            .get_or_insert_with(|| cx.focus_handle())
            .clone();

        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(bg_color)
            .text_color(text_color)
            .track_focus(&focus)
            .when(self.state.is_dragging_divider && is_horizontal, |el| el.cursor_col_resize())
            .when(self.state.is_dragging_divider && !is_horizontal, |el| el.cursor_row_resize())
            .on_mouse_up(gpui::MouseButton::Left, cx.listener(|this, _, _window, cx| {
                if this.state.is_dragging_divider {
                    this.state.is_dragging_divider = false;
                    this.divider_drag_start = None;
                    this.save_preferences_sync();
                    cx.notify();
                }
            }))
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, window, cx| {
                if this.state.is_dragging_divider {
                    if let Some((start_x, start_y, start_ratio)) = this.divider_drag_start {
                        let vp = window.viewport_size();
                        let is_horizontal = this.state.orientation == LayoutOrientation::Horizontal;
                        if is_horizontal {
                            let sidebar_w = 255.0f32;
                            let available_w = (f32::from(vp.width) - sidebar_w).max(200.0);
                            let current_x: f32 = event.position.x.into();
                            let delta = current_x - start_x;
                            let new_ratio = (start_ratio + (delta / available_w)).clamp(0.2, 0.8);
                            this.state.set_split_ratio(new_ratio);
                        } else {
                            let chrome_h = 150.0f32;
                            let available_h = (f32::from(vp.height) - chrome_h).max(200.0);
                            let current_y: f32 = event.position.y.into();
                            let delta = current_y - start_y;
                            let new_ratio = (start_ratio + (delta / available_h)).clamp(0.2, 0.8);
                            this.state.set_split_ratio(new_ratio);
                        }
                        cx.notify();
                    }
                }
            }))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _window, cx| {
                if (event.keystroke.modifiers.control || event.keystroke.modifiers.platform)
                    && (event.keystroke.key == "s" || event.keystroke.key == "S")
                {
                    this.dispatch_save_active_tab(false, cx);
                    return;
                }

                let key = event.keystroke.key.as_str();

                if this.state.editor_dialog.is_some() {
                    if key == "escape" {
                        this.state.cancel_editor_dialog();
                        cx.notify();
                    }
                    return;
                }

                if this.state.rename_dialog.is_some() {
                    if key == "escape" {
                        this.state.cancel_rename();
                        cx.notify();
                    }
                    return;
                }

                if this.state.move_to_dialog.is_some() {
                    match key {
                        "escape" => {
                            this.state.cancel_move_to();
                            cx.notify();
                        }
                        "up" => {
                            this.state.move_to_select_prev();
                            cx.notify();
                        }
                        "down" => {
                            this.state.move_to_select_next();
                            cx.notify();
                        }
                        _ => {}
                    }
                    return;
                }

                if this.state.is_command_palette_open {
                    match key {
                        "escape" => {
                            this.state.close_command_palette();
                            cx.notify();
                        }
                        "up" => {
                            this.state.palette_select_prev();
                            cx.notify();
                        }
                        "down" => {
                            this.state.palette_select_next();
                            cx.notify();
                        }
                        _ => {}
                    }
                    return;
                }

                if key == "escape" && this.state.active_input_target.is_some() {
                    this.state.blur_active_input();
                    cx.notify();
                    return;
                }

                if key == "f2" {
                    if let Some(selected) = this.state.workspace_store.selected_path.clone() {
                        let is_folder = selected.is_dir();
                        this.state.request_rename(selected, is_folder);
                        cx.notify();
                    }
                }
            }))
            // Title Bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_2()
                    .bg(surface_color)
                    .border_b_1()
                    .border_color(border_color)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w_6()
                                    .h_6()
                                    .rounded_md()
                                    .bg(accent_blue)
                                    .child(div().text_xs().text_color(rgb(0xffffff)).child("A")),
                            )
                            .child(div().text_sm().child("Aura REST API Client v0.1.0"))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_muted)
                                    .child("(GPUI + deno_core)"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .bg(surface_tile_2)
                                    .border_1()
                                    .border_color(border_color)
                                    .text_xs()
                                    .text_color(text_color)
                                    .child("⌘ Command Palette"),
                            )
                            .child(
                                div()
                                    .id("toggle-layout-btn")
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .bg(surface_tile_2)
                                    .border_1()
                                    .border_color(border_color)
                                    .text_xs()
                                    .text_color(text_color)
                                    .child(if is_horizontal {
                                        "◫ Side-by-Side"
                                    } else {
                                        "⬒ Stacked"
                                    })
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.state.toggle_orientation();
                                        this.save_preferences_sync();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .id("open-workspace-btn")
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .bg(surface_tile_2)
                                    .border_1()
                                    .border_color(border_color)
                                    .hover(|s| s.bg(rgb(0x333333)))
                                    .text_xs()
                                    .text_color(text_color)
                                    .child("📁 Open Workspace")
                                    .on_click(cx.listener(|_this, _, _window, cx| {
                                        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                                            let mut cx = cx.clone();
                                            async move {
                                                if let Some(folder) = crate::services::WorkspaceService::pick_folder().await {
                                                    this.update(&mut cx, |this, cx| {
                                                        if let Ok(()) = this.state.open_workspace_sync(&folder) {
                                                            this.start_workspace_watcher(folder, cx);
                                                            cx.notify();
                                                        }
                                                    }).ok();
                                                }
                                            }
                                        }).detach();
                                    })),
                            )
                            .child(
                                div()
                                    .id("toggle-logs-btn")
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .bg(if self.state.is_log_viewer_open { accent_blue } else { surface_tile_2 })
                                    .border_1()
                                    .border_color(border_color)
                                    .hover(|s| s.bg(rgb(0x333333)))
                                    .text_xs()
                                    .text_color(text_color)
                                    .child("🪵 Logs")
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.state.toggle_log_viewer();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .bg(rgb(0x333333))
                                    .text_xs()
                                    .text_color(rgb(0xcccccc))
                                    .child("Zero Telemetry • Offline Sandboxed"),
                            ),
                    ),
            )
            // Body: Sidebar + Main Content Area
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .child(SidebarView::render(
                        &self.state,
                        self.inputs.get_input(&InputKey::InlineRename),
                        cx,
                        |this, col_id, req_id_or_path, _window, cx| {
                            if col_id.is_empty() {
                                let path = std::path::PathBuf::from(req_id_or_path);
                                this.state.workspace_store.select_node(path.clone());
                                let _ = this.state.open_file_in_tab(&path);
                            } else {
                                this.state.open_collection_request(col_id, req_id_or_path);
                            }
                            cx.notify();
                        },
                        |this, _window, cx| {
                            this.state.new_request();
                            cx.notify();
                        },
                        |this, folder_path, _window, cx| {
                            this.state.workspace_store.select_node(folder_path.to_path_buf());
                            this.state.toggle_folder(folder_path);
                            cx.notify();
                        },
                        |this, target_path, is_folder, pos, _window, cx| {
                            this.state.workspace_store.select_node(target_path.to_path_buf());
                            this.state.workspace_store.open_context_menu(target_path.to_path_buf(), is_folder, pos);
                            cx.notify();
                        },
                        |this, folder_path, _window, cx| {
                            let root = this.state.workspace_store.workspace_root.clone();
                            let srv = this.state.workspace_service.clone();
                            let folder = folder_path.to_path_buf();
                            cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                                let mut cx = cx.clone();
                                async move {
                                    if let Some(r) = root {
                                        if let Ok(p) = srv.create_request(&r, &folder, "Untitled Request", HttpMethod::GET).await {
                                            let _ = this.update(&mut cx, |this, cx| {
                                                let _ = this.state.open_file_in_tab(&p);
                                                let _ = this.state.reconcile_workspace_sync();
                                                cx.notify();
                                            });
                                        }
                                    }
                                }
                            })
                            .detach();
                        },
                        |this, item_path, _window, cx| {
                            let root = this.state.workspace_store.workspace_root.clone();
                            let srv = this.state.workspace_service.clone();
                            let item = item_path.to_path_buf();
                            cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                                let mut cx = cx.clone();
                                async move {
                                    if let Some(r) = root {
                                        if let Ok(p) = srv.duplicate_item(&r, &item).await {
                                            let _ = this.update(&mut cx, |this, cx| {
                                                if p.is_file() {
                                                    let _ = this.state.open_file_in_tab(&p);
                                                }
                                                let _ = this.state.reconcile_workspace_sync();
                                                cx.notify();
                                            });
                                        }
                                    }
                                }
                            })
                            .detach();
                        },
                        |this, dragging_path, target_parent, placement, _window, cx| {
                            this.handle_drop_item(
                                dragging_path.to_path_buf(),
                                target_parent.to_path_buf(),
                                placement,
                                cx,
                            );
                        },
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .child(TabBarView::render(
                                &self.state,
                                cx,
                                |this, idx, _window, cx| {
                                    this.state.select_tab(idx);
                                    cx.notify();
                                },
                                |this, tab_id, _window, cx| {
                                    this.state.close_tab(tab_id);
                                    cx.notify();
                                },
                                |this, _window, cx| {
                                    this.state.new_request();
                                    cx.notify();
                                },
                            ))
                            .child(RequestHeaderView::render(
                                active_method,
                                self.inputs
                                    .get_input(&InputKey::Url)
                                    .expect("URL input initialized in sync_inputs"),
                                is_in_flight,
                                self.state.active_input_target == Some(crate::app_state::ActiveInputTarget::Url),
                                self.state.is_active_tab_dirty(),
                                cx,
                                |this, _window, cx| {
                                    this.state.cycle_method();
                                    this.schedule_auto_save(cx);
                                    cx.notify();
                                },
                                |this, _window, cx| {
                                    this.state.set_active_input(Some(crate::app_state::ActiveInputTarget::Url));
                                    cx.notify();
                                },
                                |this, _window, cx| {
                                    this.dispatch_send_request(cx);
                                },
                                |this, _window, cx| {
                                    this.state.execution_service.cancel();
                                    this.state.status_message = Some("Request cancelled by user".to_string());
                                    cx.notify();
                                },
                                |this, _window, cx| {
                                    this.dispatch_save_active_tab(false, cx);
                                },
                            ))
                            .child(
                                div()
                                    .flex()
                                    .flex_1()
                                    .min_w_0()
                                    .min_h_0()
                                    .overflow_hidden()
                                    .when(is_horizontal, |el| el.flex_row())
                                    .when(!is_horizontal, |el| el.flex_col())
                                    .child(RequestEditorView::render(
                                        self.state.split_ratio,
                                        is_horizontal,
                                        &active_title,
                                        active_method,
                                        self.state.active_editor_tab,
                                        active_tab.map(|t| &t.request),
                                        self.state.active_input_target.as_ref(),
                                        &self.inputs,
                                        self.state.active_script_sub_tab,
                                        active_tab.and_then(|t| t.response.as_ref()).map(|r| r.assertions.as_slice()),
                                        active_tab.and_then(|t| t.spec_buffer.as_deref()),
                                        active_tab.and_then(|t| t.spec_error.as_deref()),
                                        cx,
                                        |this, action, _window, cx| {
                                            this.handle_request_editor_action(action, cx);
                                        },
                                    ))
                                    .child(SplitResizerView::render(
                                        is_horizontal,
                                        self.state.is_dragging_divider,
                                        cx,
                                        |this, event, _window, cx| {
                                            if event.click_count >= 2 {
                                                this.state.reset_divider();
                                                this.divider_drag_start = None;
                                                this.state.is_dragging_divider = false;
                                                this.save_preferences_sync();
                                                cx.notify();
                                            } else {
                                                let px_val: f32 = event.position.x.into();
                                                let py_val: f32 = event.position.y.into();
                                                this.divider_drag_start = Some((px_val, py_val, this.state.split_ratio));
                                                this.state.is_dragging_divider = true;
                                                cx.notify();
                                            }
                                        },
                                    ))
                                    .child(ResponsePaneView::render(
                                        self.state.split_ratio,
                                        is_horizontal,
                                        active_tab.and_then(|t| t.response.as_ref()),
                                        self.state.active_response_tab,
                                        self.state.active_body_mode,
                                        self.inputs.get_textarea(&InputKey::ResponseBody),
                                        self.inputs.get_textarea(&InputKey::ResponseRaw),
                                        self.inputs.get_textarea(&InputKey::ResponseJson),
                                        cx,
                                        |this, tab, _window, cx| {
                                            this.state.set_response_tab(tab);
                                            cx.notify();
                                        },
                                        |this, _window, cx| {
                                            this.state.toggle_response_body_mode();
                                            cx.notify();
                                        },
                                    )),
                            ),
                    ),
            )
            // Status Bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_1()
                    .bg(surface_color)
                    .border_t_1()
                    .border_color(border_color)
                    .text_xs()
                    .text_color(text_muted)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                self.state
                                    .status_message
                                    .clone()
                                    .unwrap_or_else(|| "Ready".to_string()),
                            )
                            .child("•")
                            .child("Workspace: Local Disk Synced")
                            .child("•")
                            .child("deno_core: Offline"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(format!(
                                "Layout: {} ({}%) [Reset: Ctrl+R]",
                                if is_horizontal {
                                    "Side-by-Side"
                                } else {
                                    "Stacked"
                                },
                                split_pct
                            ))
                            .child("•")
                            .child(format!("Keymap: {}", self.state.keybinding_preset.name()))
                            .child("•")
                            .child("Aura v0.1.0"),
                    ),
            )
            .when(self.state.is_command_palette_open, |el| {
                let search_input = self
                    .inputs
                    .get_input(&InputKey::CommandPaletteSearch)
                    .expect("Command palette search input initialized in sync_inputs");
                el.child(CommandPaletteView::render(&self.state, search_input))
            })
            .when(self.state.is_log_viewer_open, |el| {
                el.child(LogViewerView::render(&self.state, cx))
            })
            .when_some(self.state.pending_delete_path.clone(), |el, delete_path| {
                let item_name = delete_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("item")
                    .to_string();

                el.child(
                    div()
                        .absolute()
                        .inset_0()
                        .bg(rgba(0x000000bb))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .w(px(380.0))
                                .bg(rgb(0x272729))
                                .border_1()
                                .border_color(rgb(0x333333))
                                .rounded_md()
                                .p_5()
                                .flex()
                                .flex_col()
                                .gap_3()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(0xffffff))
                                        .child("Delete Item"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0xaaaaaa))
                                        .child(format!(
                                            "Are you sure you want to delete '{}' from disk? This action cannot be undone.",
                                            item_name
                                        )),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_end()
                                        .gap_2()
                                        .mt_2()
                                        .child(
                                            div()
                                                .id("cancel-delete-btn")
                                                .px_3()
                                                .py_1()
                                                .bg(rgb(0x333333))
                                                .hover(|s| s.bg(rgb(0x444444)))
                                                .text_xs()
                                                .text_color(rgb(0xffffff))
                                                .rounded_sm()
                                                .cursor_pointer()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.state.cancel_delete();
                                                    cx.notify();
                                                }))
                                                .child("Cancel"),
                                        )
                                        .child(
                                            div()
                                                .id("confirm-delete-btn")
                                                .px_3()
                                                .py_1()
                                                .bg(rgb(0xe74c3c))
                                                .hover(|s| s.bg(rgb(0xc0392b)))
                                                .text_xs()
                                                .text_color(rgb(0xffffff))
                                                .rounded_sm()
                                                .cursor_pointer()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.confirm_delete_action(cx);
                                                }))
                                                .child("Delete"),
                                        ),
                                ),
                        ),
                )
            })
            .when_some(self.state.rename_dialog.clone(), |el, rename_state| {
                let item_type = if rename_state.is_folder { "Folder" } else { "Request" };
                let current_val = rename_state.current_name.clone();

                el.child(
                    div()
                        .id("rename-dialog-overlay")
                        .absolute()
                        .inset_0()
                        .bg(rgba(0x000000bb))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .id("rename-dialog-card")
                                .w(px(420.0))
                                .bg(rgb(0x272729))
                                .border_1()
                                .border_color(rgb(0x333333))
                                .rounded_md()
                                .p_5()
                                .flex()
                                .flex_col()
                                .gap_3()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(rgb(0xffffff))
                                                .child(format!("Rename {}", item_type)),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0x888888))
                                                .child("Esc to cancel"),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0xaaaaaa))
                                        .child(format!(
                                            "Enter new name for '{}':",
                                            rename_state.original_name
                                        )),
                                )
                                .child(
                                    div()
                                        .id("rename-dialog-input-box")
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .px_2()
                                        .py_1()
                                        .bg(rgb(0x1d1d1f))
                                        .border_1()
                                        .border_color(rgb(0x0066cc))
                                        .rounded_sm()
                                        .gap_2()
                                        .child(div().flex_1().child(if let Some(inp) = self.inputs.get_input(&InputKey::RenameDialog) {
                                            Input::new(inp)
                                                .appearance(false)
                                                .w_full()
                                                .text_sm()
                                                .text_color(rgb(0xffffff))
                                                .into_any_element()
                                        } else {
                                            div().text_sm().text_color(rgb(0xffffff)).child(current_val.clone()).into_any_element()
                                        }))
                                        .when(!rename_state.is_folder, |el| {
                                            el.child(div().text_xs().text_color(rgb(0x777777)).child(".aura"))
                                        }),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_end()
                                        .gap_2()
                                        .mt_2()
                                        .child(
                                            div()
                                                .id("cancel-rename-btn")
                                                .px_3()
                                                .py_1()
                                                .bg(rgb(0x333333))
                                                .hover(|s| s.bg(rgb(0x444444)))
                                                .text_xs()
                                                .text_color(rgb(0xffffff))
                                                .rounded_sm()
                                                .cursor_pointer()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.state.cancel_rename();
                                                    cx.notify();
                                                }))
                                                .child("Cancel"),
                                        )
                                        .child(
                                            div()
                                                .id("confirm-rename-btn")
                                                .px_3()
                                                .py_1()
                                                .bg(rgb(0x0066cc))
                                                .hover(|s| s.bg(rgb(0x0071e3)))
                                                .text_xs()
                                                .text_color(rgb(0xffffff))
                                                .rounded_sm()
                                                .cursor_pointer()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.confirm_rename_action(cx);
                                                }))
                                                .child("Rename"),
                                        ),
                                ),
                        ),
                )
            })
            .when_some(self.state.move_to_dialog.clone(), |el, move_dialog| {
                let item_name = move_dialog
                    .source_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("item")
                    .to_string();
                let filtered = move_dialog.filtered_destinations();
                let sel_idx = move_dialog.selected_index;

                el.child(
                    div()
                        .id("move-to-dialog-overlay")
                        .absolute()
                        .inset_0()
                        .bg(rgba(0x000000bb))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .id("move-to-dialog-card")
                                .w(px(460.0))
                                .max_h(px(420.0))
                                .bg(rgb(0x272729))
                                .border_1()
                                .border_color(rgb(0x0066cc))
                                .rounded_md()
                                .p_4()
                                .flex()
                                .flex_col()
                                .gap_3()
                                .shadow_xl()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(rgb(0xffffff))
                                                .child(format!("Move '{}' To...", item_name)),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0x888888))
                                                .child("Esc to cancel"),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .px_2()
                                        .py_1()
                                        .bg(rgb(0x1d1d1f))
                                        .border_1()
                                        .border_color(rgb(0x333333))
                                        .rounded_sm()
                                        .gap_2()
                                        .child(div().text_xs().text_color(rgb(0x0066cc)).child("🔍"))
                                        .child(div().flex_1().child(if let Some(inp) = self.inputs.get_input(&InputKey::MoveToSearch) {
                                            Input::new(inp)
                                                .appearance(false)
                                                .w_full()
                                                .text_xs()
                                                .text_color(rgb(0xffffff))
                                                .into_any_element()
                                        } else {
                                            div().text_xs().text_color(rgb(0xffffff)).child(move_dialog.search_query.clone()).into_any_element()
                                        })),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .max_h(px(240.0))
                                        .overflow_hidden()
                                        .children(filtered.iter().enumerate().map(|(idx, dest)| {
                                            let is_sel = idx == sel_idx;
                                            let dest_path = dest.path.clone();
                                            let source_path = move_dialog.source_path.clone();
                                            div()
                                                .id(ElementId::NamedInteger("move-to-dest".into(), idx as u64))
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .px_3()
                                                .py_1p5()
                                                .rounded_sm()
                                                .cursor_pointer()
                                                .bg(if is_sel {
                                                    rgb(0x27374d)
                                                } else {
                                                    rgb(0x272729)
                                                })
                                                .hover(|s| s.bg(rgb(0x333338)))
                                                .when(is_sel, |s| s.border_l_2().border_color(rgb(0x0066cc)))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.state.cancel_move_to();
                                                    this.handle_drop_item(
                                                        source_path.clone(),
                                                        dest_path.clone(),
                                                        crate::stores::workspace_store::DragPlacement::Inside,
                                                        cx,
                                                    );
                                                }))
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap_2()
                                                        .text_xs()
                                                        .text_color(if is_sel {
                                                            rgb(0xffffff)
                                                        } else {
                                                            rgb(0xcccccc)
                                                        })
                                                        .child(if dest.is_root { "🏠" } else { "📁" })
                                                        .child(dest.display_name.clone()),
                                                )
                                        })),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .pt_2()
                                        .border_t_1()
                                        .border_color(rgb(0x333333))
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0x7a7a7a))
                                                .child("↑/↓ to navigate • Enter to move"),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .gap_2()
                                                .child(
                                                    div()
                                                        .id("cancel-move-to-btn")
                                                        .px_3()
                                                        .py_1()
                                                        .bg(rgb(0x333333))
                                                        .hover(|s| s.bg(rgb(0x444444)))
                                                        .text_xs()
                                                        .text_color(rgb(0xffffff))
                                                        .rounded_sm()
                                                        .cursor_pointer()
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.state.cancel_move_to();
                                                            cx.notify();
                                                        }))
                                                        .child("Cancel"),
                                                )
                                                .child(
                                                    div()
                                                        .id("confirm-move-to-btn")
                                                        .px_3()
                                                        .py_1()
                                                        .bg(rgb(0x0066cc))
                                                        .hover(|s| s.bg(rgb(0x0071e3)))
                                                        .text_xs()
                                                        .text_color(rgb(0xffffff))
                                                        .rounded_sm()
                                                        .cursor_pointer()
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.confirm_move_to_action(cx);
                                                        }))
                                                        .child("Move"),
                                                ),
                                        ),
                                ),
                        ),
                )
            })
            .when(self.state.workspace_store.active_context_menu.is_some(), |el| {
                el.child(ContextMenuView::render(
                    &self.state,
                    cx,
                    |this, action, _window, cx| {
                        this.handle_sidebar_menu_action(action, cx);
                    },
                    |this, _window, cx| {
                        this.state.workspace_store.close_context_menu();
                        cx.notify();
                    },
                ))
            })
            .when_some(self.state.editor_dialog.clone(), |el, dialog| {
                el.child(Self::render_editor_dialog(&dialog, &self.inputs, cx))
            })
    }
}
