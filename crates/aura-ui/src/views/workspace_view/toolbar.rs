use super::AuraWorkspaceView;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::sidebar::SidebarToggleButton;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::toolbar::{Toolbar, ToolbarGroup};
use gpui_kit::component::{Side, Sizable};

impl AuraWorkspaceView {
    pub(super) fn render_top_toolbar(
        &self,
        is_horizontal: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let text_muted = rgb(0x7a7a7a);
        let view_entity = cx.entity().clone();
        let is_collapsed = self.state.is_sidebar_collapsed;

        Toolbar::new("workspace-top-toolbar")
            .small()
            .w_full()
            .h(px(38.))
            .px_3()
            .bg(rgb(0x222224))
            .border_b_1()
            .border_color(rgb(0x333333))
            .justify_between()
            .content(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        SidebarToggleButton::new()
                            .side(Side::Left)
                            .collapsed(is_collapsed)
                            .accessibility_label("Toggle Sidebar")
                            .on_click(move |_, _window, app| {
                                let _ = view_entity.update(app, |this, cx| {
                                    this.state.toggle_sidebar();
                                    cx.notify();
                                });
                            }),
                    )
                    .child(Tag::primary().small().child("A"))
                    .child(div().text_sm().child("Aura REST API Client v0.1.0"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(text_muted)
                            .child("(GPUI + deno_core)"),
                    ),
            )
            .content(
                ToolbarGroup::new("workspace-toolbar-actions")
                    .small()
                    .child(
                        Button::new("cmd-palette-btn")
                            .label("⌘ Command Palette")
                            .small()
                            .rounded(ButtonRounded::Medium)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.state.toggle_command_palette();
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("toggle-layout-btn")
                            .label(if is_horizontal {
                                "◫ Side-by-Side"
                            } else {
                                "⬒ Stacked"
                            })
                            .small()
                            .rounded(ButtonRounded::Medium)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.state.toggle_orientation();
                                this.save_preferences_sync();
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("open-workspace-btn")
                            .label("📁 Open Workspace")
                            .small()
                            .rounded(ButtonRounded::Medium)
                            .on_click(cx.listener(|_this, _, _window, cx| {
                                Self::spawn_open_workspace_picker(cx);
                            })),
                    )
                    .child({
                        let btn = Button::new("toggle-logs-btn")
                            .label("🪵 Logs")
                            .small()
                            .rounded(ButtonRounded::Medium)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.state.toggle_log_viewer();
                                cx.notify();
                            }));
                        if self.state.is_log_viewer_open {
                            btn.primary()
                        } else {
                            btn
                        }
                    })
                    .content(
                        Tag::secondary()
                            .small()
                            .child("Zero Telemetry • Offline Sandboxed"),
                    ),
            )
    }

    fn spawn_open_workspace_picker(cx: &mut Context<Self>) {
        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let Some(folder) = crate::services::WorkspaceService::pick_folder().await else {
                    return;
                };
                let _ = this.update(&mut cx, |this, cx| {
                    match this.state.open_workspace_sync(&folder) {
                        Ok(()) => {
                            this.start_workspace_watcher(folder.clone(), cx);
                            this.save_preferences_sync();
                            this.state.status_message =
                                Some(format!("Opened workspace: {}", folder.display()));
                        }
                        Err(e) => {
                            this.state.status_message =
                                Some(format!("Failed to open workspace: {e}"));
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }
}
