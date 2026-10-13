//! Modular GPUI Desktop Workspace View coordinating single-responsibility sub-views.

use crate::app_state::AuraAppState;
use crate::stores::LayoutOrientation;
use crate::views::{
    CommandPaletteView, LogViewerView, RequestEditorView, RequestHeaderView, ResponsePaneView,
    SidebarView, SplitResizerView, TabBarView,
};
use aura_core::model::HttpMethod;
use gpui::prelude::FluentBuilder;
use gpui::*;

pub struct AuraWorkspaceView {
    pub state: AuraAppState,
    #[allow(dead_code)]
    watcher_cancel: Option<tokio_util::sync::CancellationToken>,
}

impl AuraWorkspaceView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut view = Self {
            state: AuraAppState::new(),
            watcher_cancel: None,
        };
        let prefs = crate::preferences::AuraPreferences::load();
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

            cx.spawn(|this: WeakEntity<Self>, cx: &mut AsyncApp| {
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

    /// Confirms pending item deletion off the main thread and syncs workspace.
    pub fn confirm_delete_action(&mut self, cx: &mut Context<Self>) {
        let mut state_clone = self.state.clone();
        self.state.pending_delete_path = None;
        cx.notify();

        cx.spawn(|this: WeakEntity<Self>, cx: &mut AsyncApp| {
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
}


impl Default for AuraWorkspaceView {
    fn default() -> Self {
        Self {
            state: AuraAppState::new(),
            watcher_cancel: None,
        }
    }
}

impl Render for AuraWorkspaceView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_tab = self.state.tabs.get(self.state.active_tab_index);
        let active_title = active_tab
            .map(|t| t.title.clone())
            .unwrap_or_else(|| "Untitled Request".to_string());
        let active_method = active_tab.map(|t| t.method).unwrap_or(HttpMethod::GET);
        let active_url = active_tab
            .map(|t| t.request.url.clone())
            .unwrap_or_else(|| "https://postman-echo.com/get".to_string());

        let bg_color = rgb(0x1d1d1f);
        let surface_color = rgb(0x272729);
        let surface_tile_2 = rgb(0x2a2a2c);
        let text_color = rgb(0xf5f5f7);
        let text_muted = rgb(0x7a7a7a);
        let border_color = rgb(0x333333);
        let accent_blue = rgb(0x0066cc);
        let is_horizontal = self.state.orientation == LayoutOrientation::Horizontal;
        let split_pct = (self.state.split_ratio * 100.0).round() as u32;

        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(bg_color)
            .text_color(text_color)
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
                                        cx.spawn(|this: WeakEntity<Self>, cx: &mut AsyncApp| {
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
                        cx,
                        |this, col_id, req_id_or_path, _window, cx| {
                            if col_id.is_empty() {
                                let path = std::path::PathBuf::from(req_id_or_path);
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
                            this.state.toggle_folder(folder_path);
                            cx.notify();
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
                                &active_url,
                                cx,
                                |this, _window, cx| {
                                    this.state.status_message =
                                        Some("Dispatching request...".to_string());
                                    cx.notify();
                                },
                            ))
                            .child(
                                div()
                                    .flex()
                                    .flex_1()
                                    .when(is_horizontal, |el| el.flex_row())
                                    .when(!is_horizontal, |el| el.flex_col())
                                    .child(RequestEditorView::render(
                                        self.state.split_ratio,
                                        is_horizontal,
                                        &active_title,
                                        active_method,
                                    ))
                                    .child(SplitResizerView::render(
                                        is_horizontal,
                                        cx,
                                        |this, _window, cx| {
                                            this.state.reset_divider();
                                            cx.notify();
                                        },
                                    ))
                                    .child(ResponsePaneView::render(
                                        self.state.split_ratio,
                                        is_horizontal,
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
                el.child(CommandPaletteView::render(&self.state))
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
    }
}
