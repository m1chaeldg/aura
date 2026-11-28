use super::AuraWorkspaceView;
use crate::app_state::EnvironmentEditorMode;
use crate::input_registry::InputKey;
use crate::stores::workspace_store::DragPlacement;
use crate::templating_ui::render_template_token;
use crate::theme::icon;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::dialog::{Dialog, DialogFooter};
use gpui_kit::component::input::{Editor, Input};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex};
use gpui_kit::base::Disableable as _;

impl AuraWorkspaceView {
    pub(super) fn apply_workspace_modals(&self, el: Div, cx: &mut Context<Self>) -> Div {
        let el = el.when_some(self.state.pending_delete_path.clone(), |el, delete_path| {
            let item_name = delete_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("item")
                .to_string();
            let view_close = cx.entity().clone();

            el.child(
                Dialog::new(cx)
                    .w(px(380.0))
                    .title("Delete Item")
                    .on_close(move |_, _window, app| {
                        let _ = view_close.update(app, |this, cx| {
                            this.state.cancel_delete();
                            cx.notify();
                        });
                    })
                    .child(
                        div().text_xs().text_color(cx.theme().muted_foreground).child(format!(
                            "Are you sure you want to delete '{}' from disk? This action cannot be undone.",
                            item_name
                        )),
                    )
                    .footer(
                        DialogFooter::new()
                            .child(
                                Button::new("cancel-delete-btn")
                                    .label("Cancel")
                                    .small()
                                    .rounded(ButtonRounded::Medium)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.state.cancel_delete();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("confirm-delete-btn")
                                    .label("Delete")
                                    .danger()
                                    .small()
                                    .rounded(ButtonRounded::Medium)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.confirm_delete_action(cx);
                                    })),
                            ),
                    ),
            )
        })
        .when_some(self.state.rename_dialog.clone(), |el, rename_state| {
            let item_type = if rename_state.is_folder { "Folder" } else { "Request" };
            let current_val = rename_state.current_name.clone();
            let view_close = cx.entity().clone();

            el.child(
                Dialog::new(cx)
                    .w(px(420.0))
                    .title(format!("Rename {}", item_type))
                    .on_close(move |_, _window, app| {
                        let _ = view_close.update(app, |this, cx| {
                            this.state.cancel_rename();
                            cx.notify();
                        });
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(div().text_xs().text_color(cx.theme().muted_foreground).child(format!(
                                "Enter new name for '{}':",
                                rename_state.original_name
                            )))
                            .child(
                                div()
                                    .id("rename-dialog-input-box")
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    .child(div().flex_1().child(
                                        if let Some(inp) = self.inputs.get_input(&InputKey::RenameDialog) {
                                            Input::new(inp).small().w_full().into_any_element()
                                        } else {
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().foreground)
                                                .child(current_val.clone())
                                                .into_any_element()
                                        },
                                    ))
                                    .when(!rename_state.is_folder, |el| {
                                        el.child(Tag::secondary().small().child(".aura"))
                                    }),
                            ),
                    )
                    .footer(
                        DialogFooter::new()
                            .child(
                                Button::new("cancel-rename-btn")
                                    .label("Cancel")
                                    .small()
                                    .rounded(ButtonRounded::Medium)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.state.cancel_rename();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("confirm-rename-btn")
                                    .label("Rename")
                                    .primary()
                                    .small()
                                    .rounded(ButtonRounded::Medium)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.confirm_rename_action(cx);
                                    })),
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
            let view_close = cx.entity().clone();

            el.child(
                Dialog::new(cx)
                    .w(px(460.0))
                    .title(format!("Move '{}' To...", item_name))
                    .on_close(move |_, _window, app| {
                        let _ = view_close.update(app, |this, cx| {
                            this.state.cancel_move_to();
                            cx.notify();
                        });
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(
                                if let Some(inp) = self.inputs.get_input(&InputKey::MoveToSearch) {
                                    Input::new(inp)
                                        .small()
                                        .cleanable(true)
                                        .w_full()
                                        .into_any_element()
                                } else {
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().foreground)
                                        .child(move_dialog.search_query.clone())
                                        .into_any_element()
                                },
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
                                        ListItem::new(ElementId::NamedInteger("move-to-dest".into(), idx as u64))
                                            .selected(is_sel)
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.state.cancel_move_to();
                                                this.handle_drop_item(
                                                    source_path.clone(),
                                                    dest_path.clone(),
                                                    DragPlacement::Inside,
                                                    cx,
                                                );
                                            }))
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_2()
                                                    .text_xs()
                                                    .child(icon(if dest.is_root {
                                                        IconName::House
                                                    } else {
                                                        IconName::Folder
                                                    }).xsmall())
                                                    .child(dest.display_name.clone()),
                                            )
                                    })),
                            ),
                    )
                    .footer(
                        DialogFooter::new()
                            .child(
                                Button::new("cancel-move-to-btn")
                                    .label("Cancel")
                                    .small()
                                    .rounded(ButtonRounded::Medium)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.state.cancel_move_to();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("confirm-move-to-btn")
                                    .label("Move")
                                    .primary()
                                    .small()
                                    .rounded(ButtonRounded::Medium)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.confirm_move_to_action(cx);
                                    })),
                            ),
                    ),
            )
        })
        .when_some(self.state.environment_editor.clone(), |el, env_ed| {
            let Some(current_draft) = env_ed.current().cloned() else {
                return el;
            };
            let is_existing = current_draft.original_path.is_some();
            let env_id = current_draft.profile.id.clone();
            let is_active_env =
                self.state.active_environment_id.as_deref() == Some(env_id.as_str());
            let is_code_mode = current_draft.mode == EnvironmentEditorMode::Code;
            let has_code_error = current_draft.code_error.is_some();
            let dirty_count = env_ed.dirty_count();
            let confirm_close = env_ed.confirm_close;
            let last_saved_msg = env_ed.last_saved_message.clone();
            let active_env_id = self.state.active_environment_id.clone();
            let view_close = cx.entity().clone();
            let border = cx.theme().border;
            let primary_color = cx.theme().primary;
            let danger_color = cx.theme().danger;
            let danger_bg = cx.theme().danger.opacity(0.14);
            let warning_color = cx.theme().warning;
            let subtle_bg = cx.theme().secondary.opacity(0.35);
            let active_bg = cx.theme().primary.opacity(0.14);
            let muted_bg = cx.theme().muted.opacity(0.25);
            let muted_fg = cx.theme().muted_foreground;
            let fg_color = cx.theme().foreground;
            let radius = cx.theme().radius;

            el.child(
                Dialog::new(cx)
                    .w(px(860.0))
                    .title("Environment Manager")
                    .on_close(move |_, _window, app| {
                        let _ = view_close.update(app, |this, cx| {
                            this.state.request_close_environment_editor();
                            cx.notify();
                        });
                    })
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .h(px(430.0))
                            // Left pane: Profile list
                            .child(
                                div()
                                    .w(px(225.0))
                                    .flex_shrink_0()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .pr_3()
                                    .border_r_1()
                                    .border_color(border)
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(muted_fg)
                                                    .child(format!(
                                                        "PROFILES ({})",
                                                        env_ed.drafts.len()
                                                    )),
                                            )
                                            .child(
                                                Button::new("env-mgr-new-btn")
                                                    .icon(icon(IconName::Plus))
                                                    .label("New")
                                                    .xsmall()
                                                    .ghost()
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.state.open_new_environment_editor();
                                                        cx.notify();
                                                    })),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id("env-mgr-profile-list")
                                            .flex_1()
                                            .overflow_y_scroll()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .children(env_ed.drafts.iter().enumerate().map(
                                                |(idx, draft)| {
                                                    let did = draft.profile.id.clone();
                                                    let did_click = did.clone();
                                                    let did_act = did.clone();
                                                    let is_selected = did == env_id;
                                                    let is_active =
                                                        active_env_id.as_deref() == Some(did.as_str());
                                                    let display_name = if draft.profile.name.trim().is_empty() {
                                                        "(Unnamed)".to_string()
                                                    } else {
                                                        draft.profile.name.clone()
                                                    };
                                                    let var_count = draft.profile.variables.len();

                                                    div()
                                                        .id(ElementId::NamedInteger(
                                                            "env-mgr-profile-row".into(),
                                                            idx as u64,
                                                        ))
                                                        .flex()
                                                        .items_center()
                                                        .justify_between()
                                                        .gap_1p5()
                                                        .px_2()
                                                        .py_1p5()
                                                        .rounded(radius)
                                                        .cursor_pointer()
                                                        .bg(if is_selected {
                                                            active_bg
                                                        } else {
                                                            subtle_bg
                                                        })
                                                        .border_1()
                                                        .border_color(if is_selected {
                                                            primary_color
                                                        } else {
                                                            border
                                                        })
                                                        .on_click(cx.listener(move |this, _, _, cx| {
                                                            this.state.select_environment_draft(&did_click);
                                                            cx.notify();
                                                        }))
                                                        .child(
                                                            div()
                                                                .flex_1()
                                                                .min_w_0()
                                                                .flex()
                                                                .flex_col()
                                                                .gap_0p5()
                                                                .child(
                                                                    div()
                                                                        .flex()
                                                                        .items_center()
                                                                        .gap_1p5()
                                                                        .when(draft.is_dirty, |row| {
                                                                            row.child(
                                                                                div()
                                                                                    .w(px(6.0))
                                                                                    .h(px(6.0))
                                                                                    .rounded_full()
                                                                                    .bg(warning_color)
                                                                                    .flex_shrink_0(),
                                                                            )
                                                                        })
                                                                        .child(
                                                                            div()
                                                                                .text_xs()
                                                                                .font_weight(if is_selected {
                                                                                    FontWeight::SEMIBOLD
                                                                                } else {
                                                                                    FontWeight::NORMAL
                                                                                })
                                                                                .text_color(fg_color)
                                                                                .truncate()
                                                                                .child(display_name),
                                                                        ),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_xs()
                                                                        .text_color(muted_fg)
                                                                        .child(format!(
                                                                            "{} var{}",
                                                                            var_count,
                                                                            if var_count == 1 { "" } else { "s" }
                                                                        )),
                                                                ),
                                                        )
                                                        .when(draft.original_path.is_some(), |row| {
                                                            row.child(
                                                                Button::new(ElementId::NamedInteger(
                                                                    "env-mgr-set-active".into(),
                                                                    idx as u64,
                                                                ))
                                                                .icon(icon(IconName::Check))
                                                                .xsmall()
                                                                .tooltip(if is_active {
                                                                    "Active environment (click to deactivate)"
                                                                } else {
                                                                    "Set as active environment"
                                                                })
                                                                .map(|b| {
                                                                    if is_active {
                                                                        b.primary()
                                                                    } else {
                                                                        b.ghost()
                                                                    }
                                                                })
                                                                .on_click(cx.listener(
                                                                    move |this, _, _, cx| {
                                                                        cx.stop_propagation();
                                                                        if this.state.active_environment_id.as_deref()
                                                                            == Some(did_act.as_str())
                                                                        {
                                                                            this.state.set_active_environment(None);
                                                                        } else {
                                                                            this.state.set_active_environment(Some(
                                                                                did_act.clone(),
                                                                            ));
                                                                        }
                                                                        cx.notify();
                                                                    },
                                                                )),
                                                            )
                                                        })
                                                },
                                            )),
                                    ),
                            )
                            // Right pane: Active profile editor
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_3()
                                    // Mode switcher bar (UI Edit vs Code Editor) + Active toggle + Saved badge
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .gap_2()
                                            .child(
                                                h_flex()
                                                    .gap_1()
                                                    .p_0p5()
                                                    .rounded(radius)
                                                    .bg(subtle_bg)
                                                    .border_1()
                                                    .border_color(border)
                                                    .child({
                                                        let btn = Button::new("env-mode-ui-btn")
                                                            .label("UI Editor")
                                                            .xsmall()
                                                            .rounded(ButtonRounded::Small)
                                                            .on_click(cx.listener(|this, _, _, cx| {
                                                                this.state.set_environment_editor_mode(
                                                                    EnvironmentEditorMode::Ui,
                                                                );
                                                                cx.notify();
                                                            }));
                                                        if !is_code_mode {
                                                            btn.primary()
                                                        } else {
                                                            btn.ghost()
                                                        }
                                                    })
                                                    .child({
                                                        let btn = Button::new("env-mode-code-btn")
                                                            .label("Code Editor (TOML)")
                                                            .xsmall()
                                                            .rounded(ButtonRounded::Small)
                                                            .on_click(cx.listener(|this, _, _, cx| {
                                                                this.state.set_environment_editor_mode(
                                                                    EnvironmentEditorMode::Code,
                                                                );
                                                                cx.notify();
                                                            }));
                                                        if is_code_mode {
                                                            btn.primary()
                                                        } else {
                                                            btn.ghost()
                                                        }
                                                    }),
                                            )
                                            .child(
                                                h_flex()
                                                    .gap_2()
                                                    .when_some(last_saved_msg, |bar, msg| {
                                                        bar.child(
                                                            Tag::success()
                                                                .outline()
                                                                .xsmall()
                                                                .child(msg),
                                                        )
                                                    })
                                                    .when(is_existing, |bar| {
                                                        let eid_act = env_id.clone();
                                                        bar.child(
                                                            Button::new("env-ed-active-btn")
                                                                .icon(icon(IconName::Check))
                                                                .label(if is_active_env {
                                                                    "Active"
                                                                } else {
                                                                    "Set Active"
                                                                })
                                                                .xsmall()
                                                                .map(|b| {
                                                                    if is_active_env {
                                                                        b.primary()
                                                                    } else {
                                                                        b.outline()
                                                                    }
                                                                })
                                                                .on_click(cx.listener(
                                                                    move |this, _, _, cx| {
                                                                        if this.state.active_environment_id.as_deref()
                                                                            == Some(eid_act.as_str())
                                                                        {
                                                                            this.state.set_active_environment(None);
                                                                        } else {
                                                                            this.state.set_active_environment(Some(
                                                                                eid_act.clone(),
                                                                            ));
                                                                        }
                                                                        cx.notify();
                                                                    },
                                                                )),
                                                        )
                                                    })
                                                    .when(is_code_mode, |bar| {
                                                        bar.child(
                                                            h_flex()
                                                                .gap_2()
                                                                .child(if has_code_error {
                                                                    Tag::danger()
                                                                        .outline()
                                                                        .xsmall()
                                                                        .child("Invalid TOML")
                                                                } else {
                                                                    Tag::success()
                                                                        .outline()
                                                                        .xsmall()
                                                                        .child("Valid TOML")
                                                                })
                                                                .child(
                                                                    Button::new("env-code-format-btn")
                                                                        .label("Format")
                                                                        .xsmall()
                                                                        .secondary()
                                                                        .rounded(ButtonRounded::Small)
                                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                                            this.state
                                                                                .format_environment_code_buffer();
                                                                            cx.notify();
                                                                        })),
                                                                ),
                                                        )
                                                    }),
                                            ),
                                    )
                            .when(!is_code_mode, |body| {
                                body.child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(muted_fg)
                                                .child("ENVIRONMENT NAME"),
                                        )
                                        .child(
                                            if let Some(inp) =
                                                self.inputs.get_input(&InputKey::EnvName)
                                            {
                                                Input::new(inp).small().w_full().into_any_element()
                                            } else {
                                                div()
                                                    .text_xs()
                                                    .text_color(fg_color)
                                                    .child(current_draft.profile.name.clone())
                                                    .into_any_element()
                                            },
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(muted_fg)
                                                .child(format!(
                                                    "VARIABLES ({} • disabled keys save with ~)",
                                                    current_draft.profile.variables.len()
                                                )),
                                        )
                                        .child(
                                            Button::new("env-add-var-btn")
                                                .label("+ Add Variable")
                                                .xsmall()
                                                .primary()
                                                .rounded(ButtonRounded::Small)
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    if let Some(ref mut ed) =
                                                        this.state.environment_editor
                                                    {
                                                        if let Some(draft) = ed.current_mut() {
                                                            draft.profile.variables.push(
                                                                aura_core::model::VariableItem {
                                                                    key: String::new(),
                                                                    value: String::new(),
                                                                    enabled: true,
                                                                    is_secret: false,
                                                                },
                                                            );
                                                            draft.is_dirty = true;
                                                            ed.last_saved_message = None;
                                                        }
                                                        cx.notify();
                                                    }
                                                })),
                                        ),
                                )
                                .child(
                                    div()
                                        .id("env-vars-scroll-list")
                                        .flex_1()
                                        .overflow_y_scroll()
                                        .flex()
                                        .flex_col()
                                        .gap_1p5()
                                        .children(current_draft.profile.variables.iter().enumerate().map(
                                            |(idx, var)| {
                                                let enabled = var.enabled;
                                                let view_cb = cx.entity().clone();
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_2()
                                                    .p_1p5()
                                                    .rounded(radius)
                                                    .bg(if enabled {
                                                        subtle_bg
                                                    } else {
                                                        muted_bg
                                                    })
                                                    .border_1()
                                                    .border_color(border)
                                                    .child(
                                                        Checkbox::new(ElementId::NamedInteger(
                                                            "env-var-chk".into(),
                                                            idx as u64,
                                                        ))
                                                        .checked(enabled)
                                                        .on_click(move |checked, _, app| {
                                                            let _ = view_cb.update(app, |this, cx| {
                                                                if let Some(ref mut ed) =
                                                                    this.state.environment_editor
                                                                {
                                                                    if let Some(draft) = ed.current_mut() {
                                                                        if let Some(v) = draft
                                                                            .profile
                                                                            .variables
                                                                            .get_mut(idx)
                                                                        {
                                                                            v.enabled = *checked;
                                                                            draft.is_dirty = true;
                                                                            ed.last_saved_message = None;
                                                                        }
                                                                    }
                                                                }
                                                                cx.notify();
                                                            });
                                                        }),
                                                    )
                                                    .child(div().flex_1().child(
                                                        if let Some(inp) = self
                                                            .inputs
                                                            .get_input(&InputKey::EnvVarKey(idx))
                                                        {
                                                            Input::new(inp)
                                                                .xsmall()
                                                                .token(render_template_token)
                                                                .w_full()
                                                                .into_any_element()
                                                        } else {
                                                            div()
                                                                .text_xs()
                                                                .child(var.key.clone())
                                                                .into_any_element()
                                                        },
                                                    ))
                                                    .child(div().flex_1().child(
                                                        if let Some(inp) = self
                                                            .inputs
                                                            .get_input(&InputKey::EnvVarValue(idx))
                                                        {
                                                            Input::new(inp)
                                                                .xsmall()
                                                                .token(render_template_token)
                                                                .w_full()
                                                                .into_any_element()
                                                        } else {
                                                            div()
                                                                .text_xs()
                                                                .child(var.value.clone())
                                                                .into_any_element()
                                                        },
                                                    ))
                                                    .child(
                                                        Button::new(ElementId::NamedInteger(
                                                            "env-var-sec".into(),
                                                            idx as u64,
                                                        ))
                                                        .label(if var.is_secret { "Secret" } else { "Plain" })
                                                        .xsmall()
                                                        .outline()
                                                        .on_click(cx.listener(move |this, _, _, cx| {
                                                            this.state.toggle_env_var_secret(idx);
                                                            cx.notify();
                                                        })),
                                                    )
                                                    .when(var.is_secret, |row| {
                                                        let revealed = self
                                                            .state
                                                            .revealed_env_secret_rows
                                                            .contains(&idx);
                                                        row.child(
                                                            Button::new(ElementId::NamedInteger(
                                                                "env-var-rev".into(),
                                                                idx as u64,
                                                            ))
                                                            .label(if revealed { "Hide" } else { "Show" })
                                                            .xsmall()
                                                            .ghost()
                                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                                this.state.toggle_env_var_reveal(idx);
                                                                cx.notify();
                                                            })),
                                                        )
                                                    })
                                                    .child(
                                                        Button::new(ElementId::NamedInteger(
                                                            "env-var-kc".into(),
                                                            idx as u64,
                                                        ))
                                                        .label("Keychain")
                                                        .xsmall()
                                                        .outline()
                                                        .on_click(cx.listener(move |this, _, _, cx| {
                                                            match this
                                                                .state
                                                                .store_env_var_in_keychain(idx)
                                                            {
                                                                Ok(uri) => {
                                                                    this.state.status_message = Some(
                                                                        format!(
                                                                            "Stored env secret in OS Keychain as {uri}"
                                                                        ),
                                                                    );
                                                                }
                                                                Err(e) => {
                                                                    this.state.status_message =
                                                                        Some(e);
                                                                }
                                                            }
                                                            cx.notify();
                                                        })),
                                                    )
                                                    .child(
                                                        Button::new(ElementId::NamedInteger(
                                                            "env-var-del".into(),
                                                            idx as u64,
                                                        ))
                                                        .icon(icon(IconName::X))
                                                        .xsmall()
                                                        .ghost()
                                                        .on_click(cx.listener(move |this, _, _, cx| {
                                                            if let Some(ref mut ed) =
                                                                this.state.environment_editor
                                                            {
                                                                if let Some(draft) = ed.current_mut() {
                                                                    if idx < draft.profile.variables.len() {
                                                                        draft.profile.variables.remove(idx);
                                                                        draft.is_dirty = true;
                                                                        ed.last_saved_message = None;
                                                                    }
                                                                }
                                                                cx.notify();
                                                            }
                                                        })),
                                                    )
                                            },
                                        )),
                                )
                            })
                            .when(is_code_mode, |body| {
                                body.child(
                                    div()
                                        .flex_1()
                                        .rounded(radius)
                                        .border_1()
                                        .border_color(if has_code_error {
                                            danger_color
                                        } else {
                                            border
                                        })
                                        .overflow_hidden()
                                        .child(
                                            if let Some(ed) =
                                                self.inputs.get_editor(&InputKey::EnvCode)
                                            {
                                                Editor::new(ed)
                                                    .h(px(340.0))
                                                    .w_full()
                                                    .text_xs()
                                                    .into_any_element()
                                            } else {
                                                div()
                                                    .p_2()
                                                    .h(px(340.0))
                                                    .bg(muted_bg)
                                                    .font_family("monospace")
                                                    .text_xs()
                                                    .text_color(fg_color)
                                                    .child(current_draft.code_buffer.clone())
                                                    .into_any_element()
                                            },
                                        ),
                                )
                                .when_some(current_draft.code_error.clone(), |b, err| {
                                    b.child(
                                        div()
                                            .px_2p5()
                                            .py_1p5()
                                            .rounded(radius)
                                            .bg(danger_bg)
                                            .border_1()
                                            .border_color(danger_color)
                                            .text_xs()
                                            .text_color(danger_color)
                                            .child(err),
                                    )
                                })
                            }),
                            ),
                    )
                    .footer({
                        let eid_dup = env_id.clone();
                        let eid_copy = env_id.clone();
                        let eid_del = env_id.clone();
                        DialogFooter::new()
                            .when(!confirm_close, |f| {
                                f.child(
                                    Button::new("env-ed-dup-btn")
                                        .label("Duplicate")
                                        .small()
                                        .secondary()
                                        .rounded(ButtonRounded::Medium)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            let _ = this.state.duplicate_environment_in_editor(&eid_dup);
                                            cx.notify();
                                        })),
                                )
                                .when(is_existing, |f| {
                                    f.child(
                                        Button::new("env-ed-copy-btn")
                                            .label("Copy")
                                            .small()
                                            .secondary()
                                            .rounded(ButtonRounded::Medium)
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.state.copy_environment(&eid_copy);
                                                cx.notify();
                                            })),
                                    )
                                })
                                .child(
                                    Button::new("env-ed-del-btn")
                                        .label("Delete")
                                        .small()
                                        .danger()
                                        .rounded(ButtonRounded::Medium)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            let _ = this.state.delete_environment_in_editor(&eid_del);
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("env-ed-close-btn")
                                        .label("Close")
                                        .small()
                                        .rounded(ButtonRounded::Medium)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.state.request_close_environment_editor();
                                            cx.notify();
                                        })),
                                )
                                .when(dirty_count > 1, |f| {
                                    f.child(
                                        Button::new("env-ed-save-all-btn")
                                            .label(format!("Save All ({dirty_count})"))
                                            .small()
                                            .secondary()
                                            .rounded(ButtonRounded::Medium)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                if let Err(e) = this.state.save_all_environment_drafts() {
                                                    this.state.status_message =
                                                        Some(format!("Failed to save environments: {e}"));
                                                }
                                                cx.notify();
                                            })),
                                    )
                                })
                                .child(
                                    Button::new("env-ed-save-btn")
                                        .label(if current_draft.is_dirty { "Save *" } else { "Save" })
                                        .primary()
                                        .small()
                                        .rounded(ButtonRounded::Medium)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            if let Err(e) = this.state.save_environment_editor() {
                                                this.state.status_message =
                                                    Some(format!("Failed to save environment: {e}"));
                                            }
                                            cx.notify();
                                        })),
                                )
                            })
                            .when(confirm_close, |f| {
                                f.child(
                                    div()
                                        .text_xs()
                                        .text_color(warning_color)
                                        .child(format!(
                                            "Unsaved changes in {dirty_count} environment(s):"
                                        )),
                                )
                                .child(
                                    Button::new("env-ed-discard-close-btn")
                                        .label("Discard")
                                        .small()
                                        .danger()
                                        .rounded(ButtonRounded::Medium)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.state.close_environment_editor();
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("env-ed-cancel-close-btn")
                                        .label("Cancel")
                                        .small()
                                        .rounded(ButtonRounded::Medium)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.state.cancel_close_environment_editor();
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("env-ed-save-close-btn")
                                        .label("Save & Close")
                                        .primary()
                                        .small()
                                        .rounded(ButtonRounded::Medium)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            if let Err(e) = this.state.save_all_and_close_environment_editor() {
                                                this.state.status_message =
                                                    Some(format!("Failed to save environments: {e}"));
                                            }
                                            cx.notify();
                                        })),
                                )
                            })
                    }),
            )
        })
        .when_some(self.state.wsdl_import_dialog.clone(), |el, dlg| {
            let view_close = cx.entity().clone();
            let danger_color = cx.theme().red;
            let muted_color = cx.theme().muted_foreground;
            let border_color = cx.theme().border;
            let wsdl_text = dlg.wsdl_text.clone();
            let err_opt = dlg.error.clone();

            el.child(
                Dialog::new(cx)
                    .w(px(620.0))
                    .title("Import SOAP WSDL")
                    .on_close(move |_, _window, app| {
                        let _ = view_close.update(app, |this, cx| {
                            this.state.close_wsdl_import_dialog();
                            cx.notify();
                        });
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(
                                h_flex()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(muted_color)
                                            .child("Paste a WSDL XML definition or choose a .wsdl / .xml file:"),
                                    )
                                    .child(
                                        Button::new("wsdl-pick-file-btn")
                                            .icon(icon(IconName::FolderOpen))
                                            .label("Choose .wsdl File")
                                            .xsmall()
                                            .outline()
                                            .rounded(ButtonRounded::Small)
                                            .on_click(cx.listener(|this, _, _window, cx| {
                                                this.spawn_pick_wsdl_file(cx);
                                            })),
                                    ),
                            )
                            .child(if let Some(editor) = self.inputs.get_editor(&InputKey::WsdlImportText) {
                                div()
                                    .h(px(260.0))
                                    .w_full()
                                    .rounded(cx.theme().radius)
                                    .border_1()
                                    .border_color(border_color)
                                    .overflow_hidden()
                                    .child(Editor::new(editor).h(px(260.0)).w_full())
                                    .into_any_element()
                            } else {
                                div()
                                    .text_xs()
                                    .text_color(muted_color)
                                    .child("WSDL editor initializing...")
                                    .into_any_element()
                            })
                            .when_some(err_opt, |el, err| {
                                el.child(div().text_xs().text_color(danger_color).child(err))
                            }),
                    )
                    .footer(
                        DialogFooter::new()
                            .child(
                                Button::new("wsdl-cancel-btn")
                                    .label("Cancel")
                                    .small()
                                    .rounded(ButtonRounded::Medium)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.state.close_wsdl_import_dialog();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("wsdl-import-btn")
                                    .label("Import Operations")
                                    .primary()
                                    .small()
                                    .rounded(ButtonRounded::Medium)
                                    .disabled(wsdl_text.trim().is_empty())
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Err(e) = this.state.import_wsdl_to_workspace(&wsdl_text) {
                                            if let Some(ref mut d) = this.state.wsdl_import_dialog {
                                                d.error = Some(format!("{e}"));
                                            }
                                        }
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
        });
        let el = self.render_security_vault_modal(el, cx);
        let el = self.render_git_modal(el, cx);
        let el = self.render_load_tester_modal(el, cx);
        self.render_network_modals(el, cx)
    }
}
