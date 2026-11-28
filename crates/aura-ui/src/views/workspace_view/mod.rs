//! Modular GPUI Desktop Workspace View coordinating single-responsibility sub-views.

use crate::app_state::AuraAppState;
use crate::input_registry::{InputKey, InputRegistry};
use crate::stores::LayoutOrientation;
use crate::views::{CommandPaletteView, ContextMenuView, LogViewerView};

use aura_core::model::HttpMethod;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::resizable::h_resizable;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::tree::TreeState;
use std::collections::HashMap;
use std::rc::Rc;

mod ai_actions;
mod app_menus_and_modals;
mod editor_actions;
mod editor_dialog;
mod execution_actions;
mod git_actions;
mod git_modal;
mod input_events;
mod input_sync;
mod key_handler;
mod load_actions;
mod load_data_sla_panel;
mod load_report_panel;
mod load_requests_panel;
mod load_tester_modal;
mod main_pane;
mod mock_server_modal;
mod modals;
mod network_modals;
mod security_modal;
mod sidebar_panel;
mod status_bar;
mod toolbar;
mod workspace_actions;

pub struct AuraWorkspaceView {
    pub state: AuraAppState,
    pub inputs: InputRegistry,
    pub focus_handle: Option<FocusHandle>,
    #[allow(dead_code)]
    watcher_cancel: Option<tokio_util::sync::CancellationToken>,
    pub auto_save_seq: u64,
    pub sidebar_tree_state: Entity<TreeState>,
    pub sidebar_tree_meta: Rc<HashMap<String, crate::views::sidebar_view::SidebarNodeMeta>>,
    pub sidebar_tree_sig: String,
}

impl AuraWorkspaceView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut view = Self {
            state: AuraAppState::new(),
            inputs: InputRegistry::new(),
            focus_handle: Some(cx.focus_handle()),
            watcher_cancel: None,
            auto_save_seq: 0,
            sidebar_tree_state: cx.new(|cx| TreeState::new(cx)),
            sidebar_tree_meta: Rc::new(HashMap::new()),
            sidebar_tree_sig: String::new(),
        };
        let prefs = crate::preferences::AuraPreferences::load();
        view.state.auto_save_enabled = prefs.auto_save_enabled;
        view.state.auto_save_delay_ms = prefs.auto_save_delay_ms;
        view.state.domain_client_certs = prefs.domain_client_certs.clone();
        view.state.vault_config = prefs.vault_config.clone();
        view.state.global_proxy = prefs.global_proxy.clone();
        view.state.cookie_jar = prefs.cookie_jar.clone();
        view.state.update_check_frequency = prefs.update_check_frequency;
        view.state.last_update_check_epoch = prefs.last_update_check_epoch;
        view.state.ai_prefs = prefs.ai.clone();
        if view.state.ai_prefs.cli_profiles.is_empty() {
            view.spawn_ai_discovery(cx);
        }
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
        view.spawn_git_refresh(cx);
        let now_epoch = chrono::Utc::now().timestamp();
        if crate::services::UpdateService::should_check_now(
            view.state.update_check_frequency,
            view.state.last_update_check_epoch,
            now_epoch,
        ) {
            view.spawn_check_for_updates(false, cx);
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
                            if this.state.is_git_modal_open {
                                this.spawn_git_refresh(cx);
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
        prefs.domain_client_certs = self.state.domain_client_certs.clone();
        prefs.vault_config = self.state.vault_config.clone();
        prefs.global_proxy = self.state.global_proxy.clone();
        prefs.cookie_jar = self.state.cookie_jar.clone();
        prefs.update_check_frequency = self.state.update_check_frequency;
        prefs.last_update_check_epoch = self.state.last_update_check_epoch;
        if let Some(ref root) = self.state.workspace_store.workspace_root {
            prefs.last_workspace_path = Some(root.clone());
        }
        let _ = prefs.save();
    }

}

impl Default for AuraWorkspaceView {
    #[allow(unreachable_code)]
    fn default() -> Self {
        Self {
            state: AuraAppState::new(),
            inputs: InputRegistry::new(),
            focus_handle: None,
            watcher_cancel: None,
            auto_save_seq: 0,
            sidebar_tree_state: unreachable!("AuraWorkspaceView::default() requires App context; use AuraWorkspaceView::new(cx)"),
            sidebar_tree_meta: Rc::new(HashMap::new()),
            sidebar_tree_sig: String::new(),
        }
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

        let is_horizontal = self.state.orientation == LayoutOrientation::Horizontal;
        let split_pct = (self.state.split_ratio * 100.0).round() as u32;
        let is_in_flight = self.state.execution_service.is_in_flight();

        let focus = self
            .focus_handle
            .get_or_insert_with(|| cx.focus_handle())
            .clone();

        let root = div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .track_focus(&focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                this.handle_key_down(event, cx);
            }))
            .child(self.render_top_toolbar(is_horizontal, cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .child(
                        h_resizable("workspace-main-split")
                            .child(self.render_sidebar_panel(cx))
                            .child(self.render_main_pane(
                                active_tab,
                                &active_title,
                                active_method,
                                is_horizontal,
                                is_in_flight,
                                cx,
                            ))
                            .when(self.state.is_ai_drawer_open, |el| {
                                el.child(self.render_ai_drawer_panel(cx))
                            }),
                    ),
            )
            .child(self.render_status_bar(is_horizontal, split_pct, cx))
            .when(self.state.is_command_palette_open, |el| {
                let search_input = self
                    .inputs
                    .get_input(&InputKey::CommandPaletteSearch)
                    .expect("Command palette search input initialized in sync_inputs");
                el.child(CommandPaletteView::render(
                    &self.state,
                    search_input,
                    cx,
                    |this, idx, _window, cx| {
                        this.state.selected_palette_index = idx;
                        this.handle_palette_execution(cx);
                        cx.notify();
                    },
                    |this, _window, cx| {
                        this.state.close_command_palette();
                        cx.notify();
                    },
                ))
            })
            .when(self.state.is_log_viewer_open, |el| {
                let log_textarea = self.inputs.get_textarea(&InputKey::LogViewerText);
                el.child(LogViewerView::render(&self.state, log_textarea, window, cx))
            });

        let root = self.apply_workspace_modals(root, cx);
        let root = self.apply_app_info_and_settings_modals(root, cx);
        root.when(
            self.state.workspace_store.active_context_menu.is_some(),
            |el| {
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
            },
        )
        .when_some(self.state.editor_dialog.clone(), |el, dialog| {
            el.child(Self::render_editor_dialog(&dialog, &self.inputs, cx))
        })
    }
}
