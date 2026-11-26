use super::AuraWorkspaceView;
use crate::input_registry::InputKey;
use crate::stores::workspace_store::DragPlacement;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::dialog::{Dialog, DialogFooter};
use gpui_kit::component::input::Input;
use gpui_kit::component::list::ListItem;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::Sizable;

impl AuraWorkspaceView {
    pub(super) fn apply_workspace_modals(&self, el: Div, cx: &mut Context<Self>) -> Div {
        el.when_some(self.state.pending_delete_path.clone(), |el, delete_path| {
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
                        div().text_xs().text_color(rgb(0xaaaaaa)).child(format!(
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
                            .child(div().text_xs().text_color(rgb(0xaaaaaa)).child(format!(
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
                                                .text_color(rgb(0xffffff))
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
                                        .text_color(rgb(0xffffff))
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
                                                    .child(if dest.is_root { "🏠" } else { "📁" })
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
    }
}
