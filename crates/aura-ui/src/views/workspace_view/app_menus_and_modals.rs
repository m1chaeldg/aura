use super::AuraWorkspaceView;
use crate::app_state::TopMenuKind;
use crate::input_registry::InputKey;
use crate::preferences::{AiHttpKind, UpdateCheckFrequency};
use crate::services::{CURRENT_VERSION, GITHUB_REPO, UpdateCheckOutcome, UpdateService};
use crate::theme::icon;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::dialog::{Dialog, DialogFooter};
use gpui_kit::component::input::Input;
use gpui_kit::component::list::ListItem;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Sizable, ThemeStyled as _, h_flex, v_flex};

impl AuraWorkspaceView {
    pub fn handle_palette_execution(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.state.execute_selected_palette_item() {
            if id.starts_with("cmd:") {
                self.execute_app_command(&id, cx);
            }
        }
    }

    pub fn execute_app_command(&mut self, cmd_id: &str, cx: &mut Context<Self>) {
        self.state.close_top_menu();
        match cmd_id {
            "cmd:new_request" => {
                self.spawn_create_new_request(cx);
            }
            "cmd:save_request" => {
                self.dispatch_save_active_tab(false, cx);
            }
            "cmd:close_tab" => {
                self.state.close_active_tab();
            }
            "cmd:export_code" => {
                self.state.is_codegen_modal_open = true;
            }
            "cmd:open_workspace" => {
                Self::spawn_open_workspace_picker(cx);
            }
            "cmd:new_environment" => {
                self.state.open_new_environment_editor();
            }
            "cmd:import_wsdl" => {
                self.state.open_wsdl_import_dialog();
            }
            "cmd:git_sync" => {
                self.state.is_git_modal_open = true;
                self.spawn_git_refresh(cx);
            }
            "cmd:toggle_sidebar" => {
                self.state.toggle_sidebar();
            }
            "cmd:toggle_layout" => {
                self.state.toggle_orientation();
                self.save_preferences_sync();
            }
            "cmd:reset_divider" => {
                self.state.reset_divider();
                self.save_preferences_sync();
            }
            "cmd:toggle_logs" => {
                self.state.toggle_log_viewer();
            }
            "cmd:cookies" => {
                self.state.is_cookie_modal_open = true;
            }
            "cmd:proxy" => {
                self.state.is_proxy_modal_open = true;
            }
            "cmd:capture_proxy" => {
                self.state.is_interceptor_modal_open = true;
            }
            "cmd:security_vault" => {
                self.state.is_security_vault_modal_open = true;
            }
            "cmd:load_tester" => {
                self.state.is_load_tester_modal_open = true;
            }
            "cmd:ai_toggle" => {
                self.state.toggle_ai_drawer();
            }
            "cmd:ai_discover" => {
                self.state.is_ai_drawer_open = true;
                self.spawn_ai_discovery(cx);
            }
            "cmd:ai_settings" => {
                self.state.settings_tab_ai = true;
                self.state.is_settings_modal_open = true;
            }
            "cmd:settings" => {
                self.state.settings_tab_ai = false;
                self.state.is_settings_modal_open = true;
            }
            "cmd:help" => {
                self.state.is_help_modal_open = true;
            }
            "cmd:about" => {
                self.state.is_about_modal_open = true;
            }
            "cmd:check_updates" => {
                self.spawn_check_for_updates(true, cx);
            }
            _ => {}
        }
        cx.notify();
    }

    pub fn spawn_check_for_updates(
        &mut self,
        open_modal_if_current: bool,
        cx: &mut Context<Self>,
    ) {
        if self.state.update_checking {
            return;
        }
        self.state.update_checking = true;
        self.state.update_status_message = Some("Checking GitHub Releases for updates...".to_string());
        if open_modal_if_current {
            self.state.is_update_modal_open = true;
        }
        cx.notify();

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let res = UpdateService::check_latest_release().await;
                let now_epoch = chrono::Utc::now().timestamp();
                let _ = this.update(&mut cx, |this, cx| {
                    this.state.update_checking = false;
                    this.state.last_update_check_epoch = Some(now_epoch);
                    match res {
                        Ok(UpdateCheckOutcome::UpdateAvailable(info)) => {
                            this.state.update_status_message = Some(format!(
                                "New release {} available (current: v{CURRENT_VERSION}).",
                                info.tag_name
                            ));
                            this.state.update_available = Some(info);
                            this.state.is_update_modal_open = true;
                        }
                        Ok(UpdateCheckOutcome::UpToDate { current_version }) => {
                            this.state.update_available = None;
                            this.state.update_status_message = Some(format!(
                                "Aura v{current_version} is up to date."
                            ));
                        }
                        Err(e) => {
                            this.state.update_status_message =
                                Some(format!("Update check failed: {e}"));
                        }
                    }
                    this.save_preferences_sync();
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub fn spawn_install_update(
        &mut self,
        download_url: String,
        asset_name: String,
        cx: &mut Context<Self>,
    ) {
        if self.state.update_installing {
            return;
        }
        self.state.update_installing = true;
        self.state.update_status_message = Some(format!("Downloading {asset_name}..."));
        cx.notify();

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let res = UpdateService::download_and_install_update(&download_url, &asset_name).await;
                let _ = this.update(&mut cx, |this, cx| {
                    this.state.update_installing = false;
                    match res {
                        Ok(msg) => {
                            this.state.update_status_message = Some(msg);
                        }
                        Err(e) => {
                            this.state.update_status_message =
                                Some(format!("Auto-install failed: {e}. Use 'Open Release Page' to update manually."));
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub(super) fn render_top_menu_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let menus: &[(TopMenuKind, &str, &[(&str, &str)])] = &[
            (
                TopMenuKind::File,
                "File",
                &[
                    ("cmd:new_request", "New Request (Ctrl+N)"),
                    ("cmd:save_request", "Save Request (Ctrl+S)"),
                    ("cmd:close_tab", "Close Active Tab (Ctrl+W)"),
                    ("cmd:export_code", "Export Code..."),
                ],
            ),
            (
                TopMenuKind::Workspace,
                "Workspace",
                &[
                    ("cmd:open_workspace", "Open Workspace Folder..."),
                    ("cmd:new_environment", "New Environment..."),
                    ("cmd:import_wsdl", "Import WSDL..."),
                    ("cmd:git_sync", "Git Status & Sync"),
                ],
            ),
            (
                TopMenuKind::View,
                "View",
                &[
                    ("cmd:toggle_sidebar", "Toggle Sidebar"),
                    ("cmd:toggle_layout", "Toggle Split Orientation"),
                    ("cmd:reset_divider", "Reset Split 50/50"),
                    ("cmd:toggle_logs", "Toggle Diagnostic Logs"),
                ],
            ),
            (
                TopMenuKind::Tools,
                "Tools",
                &[
                    ("cmd:cookies", "Cookie Jar Manager"),
                    ("cmd:proxy", "Global Proxy Settings"),
                    ("cmd:capture_proxy", "Capture Proxy & Interceptor"),
                    ("cmd:security_vault", "Security Vaults & mTLS"),
                    ("cmd:load_tester", "Load Tester"),
                    ("cmd:settings", "Settings & Auto-Update..."),
                ],
            ),
            (
                TopMenuKind::Ai,
                "AI",
                &[
                    ("cmd:ai_toggle", "Toggle AI Assistant"),
                    ("cmd:ai_discover", "Discover Local CLIs..."),
                    ("cmd:ai_settings", "AI & Agent Settings..."),
                ],
            ),
            (
                TopMenuKind::Help,
                "Help",
                &[
                    ("cmd:help", "Documentation & Shortcuts"),
                    ("cmd:check_updates", "Check for Updates..."),
                    ("cmd:about", "About Aura & Version"),
                ],
            ),
        ];

        let mut bar = h_flex().gap_0p5();
        for &(kind, title, items) in menus {
            let is_open = self.state.open_top_menu == Some(kind);
            let btn_id = format!("top-menu-{title}");
            let menu_id = format!("top-menu-dropdown-{title}");
            bar = bar.child(
                div()
                    .relative()
                    .child({
                        let btn = Button::new(SharedString::from(btn_id))
                            .label(title)
                            .xsmall()
                            .rounded(ButtonRounded::Small)
                            .on_click(cx.listener(move |this, _, _window, cx| {
                                this.state.toggle_top_menu(kind);
                                cx.notify();
                            }));
                        if is_open { btn.primary() } else { btn.ghost() }
                    })
                    .when(is_open, |el| {
                        let dropdown = div()
                            .id(SharedString::from(menu_id))
                            .popover_style(cx)
                            .absolute()
                            .top_full()
                            .left_0()
                            .mt_1()
                            .w(px(230.))
                            .p_1()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .on_mouse_down_out(cx.listener(|this, _, _window, cx| {
                                this.state.close_top_menu();
                                cx.notify();
                            }))
                            .children(items.iter().enumerate().map(|(idx, (cmd_id, label))| {
                                let cmd = (*cmd_id).to_string();
                                ListItem::new(ElementId::NamedInteger(
                                    format!("menu-item-{title}").into(),
                                    idx as u64,
                                ))
                                .px_2()
                                .py_1()
                                .rounded(cx.theme().radius)
                                .on_click(cx.listener(move |this, _, _window, cx| {
                                    this.execute_app_command(&cmd, cx);
                                }))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().foreground)
                                        .child(*label),
                                )
                            }));
                        el.child(deferred(dropdown).with_priority(2))
                    }),
            );
        }
        bar
    }

    pub(super) fn apply_app_info_and_settings_modals(
        &self,
        el: Div,
        cx: &mut Context<Self>,
    ) -> Div {
        let muted = cx.theme().muted_foreground;
        let fg = cx.theme().foreground;
        let border = cx.theme().border;
        let radius = cx.theme().radius;

        el.when(self.state.is_settings_modal_open, |el| {
            let view_close = cx.entity().clone();
            let view_auto_save = cx.entity().clone();
            let freq = self.state.update_check_frequency;
            let last_check = self
                .state
                .last_update_check_epoch
                .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
                .unwrap_or_else(|| "Never".to_string());

            el.child(
                Dialog::new(cx)
                    .w(px(480.0))
                    .title("Settings & Auto-Update")
                    .on_close(move |_, _window, app| {
                        let _ = view_close.update(app, |this, cx| {
                            this.state.is_settings_modal_open = false;
                            cx.notify();
                        });
                    })
                    .child(
                        v_flex()
                            .gap_3()
                            .child(
                                h_flex()
                                    .gap_1p5()
                                    .child({
                                        let btn = Button::new("settings-tab-general")
                                            .label("General & Updates")
                                            .xsmall()
                                            .rounded(ButtonRounded::Small)
                                            .on_click(cx.listener(|this, _, _window, cx| {
                                                this.state.settings_tab_ai = false;
                                                cx.notify();
                                            }));
                                        if !self.state.settings_tab_ai {
                                            btn.primary()
                                        } else {
                                            btn.outline()
                                        }
                                    })
                                    .child({
                                        let btn = Button::new("settings-tab-ai")
                                            .label("AI & Agents")
                                            .xsmall()
                                            .rounded(ButtonRounded::Small)
                                            .on_click(cx.listener(|this, _, _window, cx| {
                                                this.state.settings_tab_ai = true;
                                                cx.notify();
                                            }));
                                        if self.state.settings_tab_ai {
                                            btn.primary()
                                        } else {
                                            btn.outline()
                                        }
                                    }),
                            )
                            .when(self.state.settings_tab_ai, |el| {
                                self.render_ai_settings_section(el, fg, muted, border, radius, cx)
                            })
                            .when(!self.state.settings_tab_ai, |el| {
                                el.child(
                                v_flex()
                                    .gap_1p5()
                                    .p_3()
                                    .rounded(radius)
                                    .border_1()
                                    .border_color(border)
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(fg)
                                            .child("Workspace & Editor"),
                                    )
                                    .child(
                                        Checkbox::new("settings-autosave-toggle")
                                            .checked(self.state.auto_save_enabled)
                                            .label("Auto-save modified requests to disk")
                                            .small()
                                            .on_click(move |_, _window, app| {
                                                let _ = view_auto_save.update(app, |this, cx| {
                                                    this.state.auto_save_enabled =
                                                        !this.state.auto_save_enabled;
                                                    this.save_preferences_sync();
                                                    cx.notify();
                                                });
                                            }),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .gap_2()
                                    .p_3()
                                    .rounded(radius)
                                    .border_1()
                                    .border_color(border)
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(fg)
                                                    .child("GitHub Release Auto-Update"),
                                            )
                                            .child(
                                                Tag::secondary()
                                                    .xsmall()
                                                    .child(format!("v{CURRENT_VERSION}")),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(muted)
                                            .child(format!(
                                                "Checks github.com/{GITHUB_REPO}/releases. Last checked: {last_check}"
                                            )),
                                    )
                                    .child({
                                        let mut row = h_flex().flex_wrap().gap_1p5();
                                        for (idx, option) in
                                            UpdateCheckFrequency::ALL.iter().copied().enumerate()
                                        {
                                            let is_sel = freq == option;
                                            let btn = Button::new(ElementId::NamedInteger(
                                                "update-freq-btn".into(),
                                                idx as u64,
                                            ))
                                            .label(option.label())
                                            .xsmall()
                                            .rounded(ButtonRounded::Small)
                                            .on_click(cx.listener(move |this, _, _window, cx| {
                                                this.state.update_check_frequency = option;
                                                this.save_preferences_sync();
                                                cx.notify();
                                            }));
                                            row = row.child(if is_sel {
                                                btn.primary()
                                            } else {
                                                btn.outline()
                                            });
                                        }
                                        row
                                    })
                                    .when_some(
                                        self.state.update_status_message.clone(),
                                        |el, msg| {
                                            el.child(div().text_xs().text_color(muted).child(msg))
                                        },
                                    ),
                            )
                            }),
                    )
                    .footer(
                        DialogFooter::new()
                            .child(
                                Button::new("settings-check-updates-now")
                                    .icon(icon(IconName::RefreshCw))
                                    .label(if self.state.update_checking {
                                        "Checking..."
                                    } else {
                                        "Check for Updates Now"
                                    })
                                    .small()
                                    .outline()
                                    .rounded(ButtonRounded::Medium)
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.spawn_check_for_updates(true, cx);
                                    })),
                            )
                            .child(
                                Button::new("settings-close-btn")
                                    .label("Done")
                                    .small()
                                    .primary()
                                    .rounded(ButtonRounded::Medium)
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.state.is_settings_modal_open = false;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
        })
        .when(self.state.is_update_modal_open, |el| {
            let view_close = cx.entity().clone();
            let release = self.state.update_available.clone();
            let status_msg = self
                .state
                .update_status_message
                .clone()
                .unwrap_or_else(|| format!("Current version: v{CURRENT_VERSION}"));

            el.child(
                Dialog::new(cx)
                    .w(px(500.0))
                    .title("Aura Software Update")
                    .on_close(move |_, _window, app| {
                        let _ = view_close.update(app, |this, cx| {
                            this.state.is_update_modal_open = false;
                            cx.notify();
                        });
                    })
                    .child(
                        v_flex()
                            .gap_3()
                            .child(div().text_xs().text_color(fg).child(status_msg))
                            .when_some(release.clone(), |el, info| {
                                el.child(
                                    v_flex()
                                        .gap_2()
                                        .p_3()
                                        .rounded(radius)
                                        .border_1()
                                        .border_color(border)
                                        .child(
                                            h_flex()
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .text_color(fg)
                                                        .child(info.name.clone()),
                                                )
                                                .child(Tag::success().xsmall().child(info.tag_name)),
                                        )
                                        .child(
                                            div()
                                                .id("update-release-notes-scroll")
                                                .max_h(px(200.))
                                                .overflow_y_scrollbar()
                                                .text_xs()
                                                .text_color(muted)
                                                .child(if info.body.trim().is_empty() {
                                                    "No release notes provided.".to_string()
                                                } else {
                                                    info.body.clone()
                                                }),
                                        ),
                                )
                            }),
                    )
                    .footer({
                        let mut footer = DialogFooter::new();
                        if let Some(info) = release {
                            let html_url = info.html_url.clone();
                            footer = footer.child(
                                Button::new("update-open-browser-btn")
                                    .label("Open Release Page")
                                    .small()
                                    .outline()
                                    .rounded(ButtonRounded::Medium)
                                    .on_click(cx.listener(move |this, _, _window, cx| {
                                        if let Err(e) = UpdateService::open_url_in_browser(&html_url)
                                        {
                                            this.state.update_status_message =
                                                Some(format!("Failed to open browser: {e}"));
                                            cx.notify();
                                        }
                                    })),
                            );
                            if let (Some(url), Some(asset)) =
                                (info.asset_download_url, info.asset_name)
                            {
                                footer = footer.child(
                                    Button::new("update-install-btn")
                                        .label(if self.state.update_installing {
                                            "Installing..."
                                        } else {
                                            "Download & Install Update"
                                        })
                                        .small()
                                        .primary()
                                        .rounded(ButtonRounded::Medium)
                                        .on_click(cx.listener(move |this, _, _window, cx| {
                                            this.spawn_install_update(
                                                url.clone(),
                                                asset.clone(),
                                                cx,
                                            );
                                        })),
                                );
                            }
                        } else {
                            footer = footer.child(
                                Button::new("update-recheck-btn")
                                    .label("Check Again")
                                    .small()
                                    .outline()
                                    .rounded(ButtonRounded::Medium)
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.spawn_check_for_updates(true, cx);
                                    })),
                            );
                        }
                        footer.child(
                            Button::new("update-close-btn")
                                .label("Close")
                                .small()
                                .ghost()
                                .rounded(ButtonRounded::Medium)
                                .on_click(cx.listener(|this, _, _window, cx| {
                                    this.state.is_update_modal_open = false;
                                    cx.notify();
                                })),
                        )
                    }),
            )
        })
        .when(self.state.is_about_modal_open, |el| {
            let view_close = cx.entity().clone();
            el.child(
                Dialog::new(cx)
                    .w(px(420.0))
                    .title("About Aura")
                    .on_close(move |_, _window, app| {
                        let _ = view_close.update(app, |this, cx| {
                            this.state.is_about_modal_open = false;
                            cx.notify();
                        });
                    })
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(Tag::primary().small().child("Aura"))
                                    .child(
                                        Tag::secondary()
                                            .small()
                                            .child(format!("v{CURRENT_VERSION}")),
                                    ),
                            )
                            .child(div().text_xs().text_color(fg).child(
                                "High-performance desktop REST, GraphQL, gRPC, WebSocket & SOAP API client.",
                            ))
                            .child(div().text_xs().text_color(muted).child(
                                "Built with Rust, GPUI, Tokio, and sandboxed deno_core V8 scripting.",
                            ))
                            .child(div().text_xs().text_color(muted).child(format!(
                                "Repository: https://github.com/{GITHUB_REPO}"
                            ))),
                    )
                    .footer(
                        DialogFooter::new()
                            .child(
                                Button::new("about-check-updates-btn")
                                    .label("Check for Updates")
                                    .small()
                                    .outline()
                                    .rounded(ButtonRounded::Medium)
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.state.is_about_modal_open = false;
                                        this.spawn_check_for_updates(true, cx);
                                    })),
                            )
                            .child(
                                Button::new("about-close-btn")
                                    .label("Close")
                                    .small()
                                    .primary()
                                    .rounded(ButtonRounded::Medium)
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.state.is_about_modal_open = false;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
        })
        .when(self.state.is_help_modal_open, |el| {
            let view_close = cx.entity().clone();
            let help_sections: &[(&str, &str)] = &[
                (
                    "Keyboard Shortcuts",
                    "Ctrl+K: Command Palette · Ctrl+N: New Request · Ctrl+S: Save · Ctrl+W: Close Tab · Ctrl+Enter: Send Request",
                ),
                (
                    "Variables & Cascade",
                    "Precedence: Request → Active Environment → Collection → Built-in ({{$uuid}}, {{$timestamp}}, {{$faker.*}}) → OS Env ($VAR). Secrets: vault://, aws-sm://, keychain://.",
                ),
                (
                    "Scripting & BDD Tests",
                    "Use aura.variables.get/set, fetch(), aura.sendRequest(), aura.setNextRequest(), describe/it/test/expect, and offline faker.* in Pre-request, Post-response, and Tests.",
                ),
                (
                    "Declarative [assert] & [extract]",
                    "Zero-JS rules in .aura Spec or Tests tab: e.g. res.status: eq 200, res.body.id: isNumber, or [extract] token = \"jsonpath:$.token\".",
                ),
            ];
            el.child(
                Dialog::new(cx)
                    .w(px(540.0))
                    .title("Aura Help & Quick Reference")
                    .on_close(move |_, _window, app| {
                        let _ = view_close.update(app, |this, cx| {
                            this.state.is_help_modal_open = false;
                            cx.notify();
                        });
                    })
                    .child(
                        v_flex()
                            .id("help-modal-scroll")
                            .max_h(px(360.))
                            .overflow_y_scrollbar()
                            .gap_2()
                            .children(help_sections.iter().map(|(title, body)| {
                                v_flex()
                                    .gap_1()
                                    .p_2p5()
                                    .rounded(radius)
                                    .border_1()
                                    .border_color(border)
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(fg)
                                            .child(*title),
                                    )
                                    .child(div().text_xs().text_color(muted).child(*body))
                            })),
                    )
                    .footer(
                        DialogFooter::new().child(
                            Button::new("help-close-btn")
                                .label("Close")
                                .small()
                                .primary()
                                .rounded(ButtonRounded::Medium)
                                .on_click(cx.listener(|this, _, _window, cx| {
                                    this.state.is_help_modal_open = false;
                                    cx.notify();
                                })),
                        ),
                    ),
            )
        })
    }

    pub(super) fn render_ai_settings_section(
        &self,
        el: Div,
        fg: Hsla,
        muted: Hsla,
        border: Hsla,
        radius: Pixels,
        cx: &mut Context<Self>,
    ) -> Div {
        let active_id = self.state.ai_prefs.active_provider_id.clone();
        let cli_name_in = self.inputs.get_input(&InputKey::AiCustomCliName);
        let cli_cmd_in = self.inputs.get_input(&InputKey::AiCustomCliCmd);
        let cli_args_in = self.inputs.get_input(&InputKey::AiCustomCliArgs);
        let http_name_in = self.inputs.get_input(&InputKey::AiHttpName);
        let http_url_in = self.inputs.get_input(&InputKey::AiHttpUrl);
        let http_model_in = self.inputs.get_input(&InputKey::AiHttpModel);
        let http_key_in = self.inputs.get_input(&InputKey::AiHttpApiKey);

        el.child(
            v_flex()
                .id("ai-settings-scroll")
                .max_h(px(360.))
                .overflow_y_scrollbar()
                .gap_2p5()
                .child(
                    v_flex()
                        .gap_1p5()
                        .p_3()
                        .rounded(radius)
                        .border_1()
                        .border_color(border)
                        .child(
                            h_flex()
                                .justify_between()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(fg)
                                        .child("Active Providers & Auto-Discovery"),
                                )
                                .child(
                                    Button::new("ai-settings-discover-btn")
                                        .icon(icon(IconName::RefreshCw))
                                        .label(if self.state.ai_discovery_in_flight {
                                            "Scanning..."
                                        } else {
                                            "Discover CLIs"
                                        })
                                        .xsmall()
                                        .outline()
                                        .rounded(ButtonRounded::Small)
                                        .on_click(cx.listener(|this, _, _window, cx| {
                                            this.spawn_ai_discovery(cx);
                                        })),
                                ),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted)
                                .child("Scans PATH for claude, codex, copilot, cursor, gemini, agy & local Ollama/LM Studio."),
                        )
                        .children(self.state.ai_prefs.cli_profiles.iter().enumerate().map(|(i, p)| {
                            let pid = p.id.clone();
                            let is_sel = active_id.as_deref() == Some(pid.as_str());
                            h_flex()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(fg)
                                        .child(format!("CLI: {} ({})", p.name, p.command)),
                                )
                                .child(
                                    Button::new(ElementId::NamedInteger("ai-set-cli".into(), i as u64))
                                        .label(if is_sel { "Active" } else { "Use" })
                                        .xsmall()
                                        .rounded(ButtonRounded::Small)
                                        .on_click(cx.listener(move |this, _, _window, cx| {
                                            this.state.ai_prefs.active_provider_id = Some(pid.clone());
                                            this.save_ai_preferences_sync();
                                            cx.notify();
                                        })),
                                )
                        }))
                        .children(self.state.ai_prefs.http_profiles.iter().enumerate().map(|(i, p)| {
                            let pid = p.id.clone();
                            let is_sel = active_id.as_deref() == Some(pid.as_str());
                            h_flex()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(fg)
                                        .child(format!("HTTP: {} [{}]", p.name, p.model)),
                                )
                                .child(
                                    Button::new(ElementId::NamedInteger("ai-set-http".into(), i as u64))
                                        .label(if is_sel { "Active" } else { "Use" })
                                        .xsmall()
                                        .rounded(ButtonRounded::Small)
                                        .on_click(cx.listener(move |this, _, _window, cx| {
                                            this.state.ai_prefs.active_provider_id = Some(pid.clone());
                                            this.save_ai_preferences_sync();
                                            cx.notify();
                                        })),
                                )
                        })),
                )
                .child(
                    v_flex()
                        .gap_1p5()
                        .p_3()
                        .rounded(radius)
                        .border_1()
                        .border_color(border)
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(fg)
                                .child("Safety & Context"),
                        )
                        .child(
                            Checkbox::new("ai-settings-redact")
                                .checked(self.state.ai_prefs.redact_secrets)
                                .label("Redact sensitive headers, tokens & env secrets")
                                .small()
                                .on_click(cx.listener(|this, _, _window, cx| {
                                    this.state.ai_prefs.redact_secrets = !this.state.ai_prefs.redact_secrets;
                                    this.save_ai_preferences_sync();
                                    cx.notify();
                                })),
                        )
                        .child(
                            Checkbox::new("ai-settings-auto-apply")
                                .checked(self.state.ai_prefs.auto_apply_edits)
                                .label("Auto-apply proposed body, script & spec edits")
                                .small()
                                .on_click(cx.listener(|this, _, _window, cx| {
                                    this.state.ai_prefs.auto_apply_edits = !this.state.ai_prefs.auto_apply_edits;
                                    this.save_ai_preferences_sync();
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    v_flex()
                        .gap_1p5()
                        .p_3()
                        .rounded(radius)
                        .border_1()
                        .border_color(border)
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(fg)
                                .child("Add Custom Agent CLI"),
                        )
                        .when_some(cli_name_in, |el, s| el.child(Input::new(&s).small()))
                        .when_some(cli_cmd_in, |el, s| el.child(Input::new(&s).small()))
                        .when_some(cli_args_in, |el, s| el.child(Input::new(&s).small()))
                        .child(
                            Button::new("ai-add-cli-btn")
                                .label("Add CLI Profile")
                                .xsmall()
                                .primary()
                                .rounded(ButtonRounded::Small)
                                .on_click(cx.listener(|this, _, _window, cx| {
                                    this.add_custom_ai_cli_profile();
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    v_flex()
                        .gap_1p5()
                        .p_3()
                        .rounded(radius)
                        .border_1()
                        .border_color(border)
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(fg)
                                .child("Add OpenAI-Compatible / Anthropic Endpoint"),
                        )
                        .when_some(http_name_in, |el, s| el.child(Input::new(&s).small()))
                        .when_some(http_url_in, |el, s| el.child(Input::new(&s).small()))
                        .when_some(http_model_in, |el, s| el.child(Input::new(&s).small()))
                        .when_some(http_key_in, |el, s| el.child(Input::new(&s).small()))
                        .child(
                            h_flex()
                                .gap_1p5()
                                .child(
                                    Button::new("ai-add-openai-btn")
                                        .label("Add OpenAI-Compatible")
                                        .xsmall()
                                        .primary()
                                        .rounded(ButtonRounded::Small)
                                        .on_click(cx.listener(|this, _, _window, cx| {
                                            this.add_custom_ai_http_profile(AiHttpKind::OpenAiCompatible);
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("ai-add-anthropic-btn")
                                        .label("Add Anthropic")
                                        .xsmall()
                                        .outline()
                                        .rounded(ButtonRounded::Small)
                                        .on_click(cx.listener(|this, _, _window, cx| {
                                            this.add_custom_ai_http_profile(AiHttpKind::Anthropic);
                                            cx.notify();
                                        })),
                                ),
                        ),
                ),
        )
    }
}
