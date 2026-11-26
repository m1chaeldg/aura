use super::AuraWorkspaceView;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::list::ListItem;
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
                    .content({
                        let active_env_label = self
                            .state
                            .active_environment()
                            .map(|e| e.name.as_str())
                            .unwrap_or("No Env");
                        let is_env_open = self.state.is_env_dropdown_open;
                        let active_env_id = self.state.active_environment_id.clone();
                        let env_list: Vec<(String, String)> = self
                            .state
                            .environments
                            .iter()
                            .map(|(_, e)| (e.id.clone(), e.name.clone()))
                            .collect();

                        div()
                            .relative()
                            .child(
                                Button::new("env-selector-dropdown-btn")
                                    .label(format!("🌐 {active_env_label} ▾"))
                                    .small()
                                    .rounded(ButtonRounded::Medium)
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.state.toggle_env_dropdown();
                                        cx.notify();
                                    })),
                            )
                            .when(is_env_open, |el| {
                                let menu = div()
                                    .id("env-selector-dropdown-menu")
                                    .absolute()
                                    .top_full()
                                    .right_0()
                                    .mt_1()
                                    .w(px(210.0))
                                    .bg(rgb(0x252528))
                                    .border_1()
                                    .border_color(rgb(0x333333))
                                    .rounded_md()
                                    .p_1()
                                    .shadow_lg()
                                    .flex()
                                    .flex_col()
                                    .gap_0p5()
                                    .on_mouse_down_out(cx.listener(|this, _, _window, cx| {
                                        this.state.close_env_dropdown();
                                        cx.notify();
                                    }))
                                    .child({
                                        let is_none = active_env_id.is_none();
                                        ListItem::new("env-opt-none")
                                            .selected(is_none)
                                            .px_2()
                                            .py_1()
                                            .rounded_sm()
                                            .on_click(cx.listener(|this, _, _window, cx| {
                                                this.state.set_active_environment(None);
                                                cx.notify();
                                            }))
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .w_full()
                                                    .text_xs()
                                                    .child(
                                                        div()
                                                            .text_color(rgb(0xaaaaaa))
                                                            .child("No Environment"),
                                                    )
                                                    .when(is_none, |el| {
                                                        el.child(
                                                            div()
                                                                .text_color(rgb(0x2997ff))
                                                                .child("✓"),
                                                        )
                                                    }),
                                            )
                                    })
                                    .children(env_list.into_iter().enumerate().map(
                                        |(idx, (eid, ename))| {
                                            let is_sel =
                                                active_env_id.as_deref() == Some(eid.as_str());
                                            let eid_click = eid.clone();
                                            ListItem::new(ElementId::NamedInteger(
                                                "env-opt".into(),
                                                idx as u64,
                                            ))
                                            .selected(is_sel)
                                            .px_2()
                                            .py_1()
                                            .rounded_sm()
                                            .on_click(cx.listener(
                                                move |this, _, _window, cx| {
                                                    this.state.set_active_environment(Some(
                                                        eid_click.clone(),
                                                    ));
                                                    cx.notify();
                                                },
                                            ))
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .w_full()
                                                    .text_xs()
                                                    .child(
                                                        div()
                                                            .text_color(if is_sel {
                                                                rgb(0xffffff)
                                                            } else {
                                                                rgb(0xcccccc)
                                                            })
                                                            .truncate()
                                                            .child(ename),
                                                    )
                                                    .when(is_sel, |el| {
                                                        el.child(
                                                            div()
                                                                .text_color(rgb(0x2997ff))
                                                                .child("✓"),
                                                        )
                                                    }),
                                            )
                                        },
                                    ))
                                    .child(div().h(px(1.0)).bg(rgb(0x333333)).my_1())
                                    .child(
                                        ListItem::new("env-opt-new")
                                            .px_2()
                                            .py_1()
                                            .rounded_sm()
                                            .on_click(cx.listener(|this, _, _window, cx| {
                                                this.state.open_new_environment_editor();
                                                cx.notify();
                                            }))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgb(0x2997ff))
                                                    .child("+ New Environment..."),
                                            ),
                                    );
                                el.child(deferred(menu).with_priority(1))
                            })
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
