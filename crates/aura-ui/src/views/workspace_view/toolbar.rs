use super::AuraWorkspaceView;
use crate::theme::icon;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::separator::Separator;
use gpui_kit::component::sidebar::SidebarToggleButton;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::toolbar::{Toolbar, ToolbarGroup};
use gpui_kit::component::{ActiveTheme as _, Side, Sizable, ThemeStyled as _, h_flex};

impl AuraWorkspaceView {
    pub(super) fn render_top_toolbar(
        &self,
        is_horizontal: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view_entity = cx.entity().clone();
        let is_collapsed = self.state.is_sidebar_collapsed;

        Toolbar::new("workspace-top-toolbar")
            .small()
            .w_full()
            .px_3()
            .py_1()
            .bg(cx.theme().title_bar)
            .border_b_1()
            .border_color(cx.theme().title_bar_border)
            .justify_between()
            .content(
                h_flex()
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
                    .child(Tag::primary().small().child("Aura"))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(cx.theme().foreground)
                            .child("REST API Client"),
                    ),
            )
            .content(
                ToolbarGroup::new("workspace-toolbar-actions")
                    .small()
                    .child(
                        Button::new("cmd-palette-btn")
                            .icon(icon(IconName::Search))
                            .label("Command Palette")
                            .small()
                            .ghost()
                            .rounded(ButtonRounded::Medium)
                            .tooltip("Search requests & commands (Ctrl+K)")
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.state.toggle_command_palette();
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("toggle-layout-btn")
                            .icon(icon(if is_horizontal {
                                IconName::Columns2
                            } else {
                                IconName::Rows2
                            }))
                            .label(if is_horizontal {
                                "Side-by-Side"
                            } else {
                                "Stacked"
                            })
                            .small()
                            .ghost()
                            .rounded(ButtonRounded::Medium)
                            .tooltip("Toggle split orientation")
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.state.toggle_orientation();
                                this.save_preferences_sync();
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("open-workspace-btn")
                            .icon(icon(IconName::FolderOpen))
                            .label("Workspace")
                            .small()
                            .ghost()
                            .rounded(ButtonRounded::Medium)
                            .tooltip("Open workspace folder")
                            .on_click(cx.listener(|_this, _, _window, cx| {
                                Self::spawn_open_workspace_picker(cx);
                            })),
                    )
                    .child(
                        Button::new("import-wsdl-toolbar-btn")
                            .icon(icon(IconName::FileText))
                            .label("Import WSDL")
                            .small()
                            .ghost()
                            .rounded(ButtonRounded::Medium)
                            .tooltip("Import SOAP WSDL definition into collection")
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.state.open_wsdl_import_dialog();
                                cx.notify();
                            })),
                    )
                    .child({
                        let btn = Button::new("toggle-logs-btn")
                            .icon(icon(IconName::ScrollText))
                            .label("Logs")
                            .small()
                            .rounded(ButtonRounded::Medium)
                            .tooltip("Toggle diagnostic logs")
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.state.toggle_log_viewer();
                                cx.notify();
                            }));
                        if self.state.is_log_viewer_open {
                            btn.primary()
                        } else {
                            btn.ghost()
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
                                    .icon(icon(IconName::Globe))
                                    .label(active_env_label.to_string())
                                    .dropdown_caret(true)
                                    .small()
                                    .outline()
                                    .rounded(ButtonRounded::Medium)
                                    .tooltip("Select active environment profile")
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.state.toggle_env_dropdown();
                                        cx.notify();
                                    })),
                            )
                            .when(is_env_open, |el| {
                                let menu = div()
                                    .id("env-selector-dropdown-menu")
                                    .popover_style(cx)
                                    .absolute()
                                    .top_full()
                                    .right_0()
                                    .mt_1()
                                    .w(px(220.0))
                                    .p_1()
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
                                            .rounded(cx.theme().radius)
                                            .on_click(cx.listener(|this, _, _window, cx| {
                                                this.state.set_active_environment(None);
                                                cx.notify();
                                            }))
                                            .child(
                                                h_flex()
                                                    .justify_between()
                                                    .w_full()
                                                    .text_xs()
                                                    .child(
                                                        div()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .child("No Environment"),
                                                    )
                                                    .when(is_none, |el| {
                                                        el.child(
                                                            icon(IconName::Check)
                                                                .xsmall()
                                                                .text_color(cx.theme().primary),
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
                                            .rounded(cx.theme().radius)
                                            .on_click(cx.listener(
                                                move |this, _, _window, cx| {
                                                    this.state.set_active_environment(Some(
                                                        eid_click.clone(),
                                                    ));
                                                    cx.notify();
                                                },
                                            ))
                                            .child(
                                                h_flex()
                                                    .justify_between()
                                                    .w_full()
                                                    .text_xs()
                                                    .child(
                                                        div()
                                                            .text_color(if is_sel {
                                                                cx.theme().foreground
                                                            } else {
                                                                cx.theme().muted_foreground
                                                            })
                                                            .truncate()
                                                            .child(ename),
                                                    )
                                                    .when(is_sel, |el| {
                                                        el.child(
                                                            icon(IconName::Check)
                                                                .xsmall()
                                                                .text_color(cx.theme().primary),
                                                        )
                                                    }),
                                            )
                                        },
                                    ))
                                    .child(Separator::horizontal().my_1())
                                    .child(
                                        ListItem::new("env-opt-new")
                                            .px_2()
                                            .py_1()
                                            .rounded(cx.theme().radius)
                                            .on_click(cx.listener(|this, _, _window, cx| {
                                                this.state.open_new_environment_editor();
                                                cx.notify();
                                            }))
                                            .child(
                                                h_flex()
                                                    .gap_1p5()
                                                    .text_xs()
                                                    .text_color(cx.theme().primary)
                                                    .child(icon(IconName::Plus).xsmall())
                                                    .child("New Environment..."),
                                            ),
                                    );
                                el.child(deferred(menu).with_priority(1))
                            })
                    }),
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
