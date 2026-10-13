use crate::app_state::AuraAppState;
use gpui::prelude::FluentBuilder;
use gpui::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarMenuAction {
    NewRequest,
    NewFolder,
    Duplicate,
    Copy,
    Paste,
    Rename,
    Delete,
}

pub struct ContextMenuView;

impl ContextMenuView {
    pub fn render<V: 'static>(
        state: &AuraAppState,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, SidebarMenuAction, &mut Window, &mut Context<V>) + 'static + Copy,
        on_dismiss: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let (pos_x, pos_y) = state
            .workspace_store
            .active_context_menu
            .as_ref()
            .map(|m| m.position)
            .unwrap_or((100.0, 100.0));

        let has_clipboard = state.workspace_store.clipboard_item.is_some();

        div()
            .id("context-menu-backdrop")
            .absolute()
            .inset_0()
            .on_click(cx.listener(move |this, _, window, cx| {
                on_dismiss(this, window, cx);
            }))
            .child(
                div()
                    .id("context-menu-popup")
                    .absolute()
                    .left(px(pos_x))
                    .top(px(pos_y))
                    .w(px(190.0))
                    .bg(rgb(0x252528))
                    .border_1()
                    .border_color(rgb(0x333333))
                    .rounded_md()
                    .p_1()
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(Self::render_menu_item(

                        "New Request",
                        None,
                        false,
                        false,
                        SidebarMenuAction::NewRequest,
                        cx,
                        on_action,
                    ))
                    .child(Self::render_menu_item(
                        "New Folder",
                        None,
                        false,
                        false,
                        SidebarMenuAction::NewFolder,
                        cx,
                        on_action,
                    ))
                    .child(Self::render_divider())
                    .child(Self::render_menu_item(
                        "Duplicate",
                        Some("Ctrl+D"),
                        false,
                        false,
                        SidebarMenuAction::Duplicate,
                        cx,
                        on_action,
                    ))
                    .child(Self::render_menu_item(
                        "Copy",
                        Some("Ctrl+C"),
                        false,
                        false,
                        SidebarMenuAction::Copy,
                        cx,
                        on_action,
                    ))
                    .child(Self::render_menu_item(
                        "Paste",
                        Some("Ctrl+V"),
                        false,
                        !has_clipboard,
                        SidebarMenuAction::Paste,
                        cx,
                        on_action,
                    ))
                    .child(Self::render_divider())
                    .child(Self::render_menu_item(
                        "Rename",
                        Some("F2"),
                        false,
                        false,
                        SidebarMenuAction::Rename,
                        cx,
                        on_action,
                    ))
                    .child(Self::render_menu_item(
                        "Delete",
                        Some("Del"),
                        true,
                        false,
                        SidebarMenuAction::Delete,
                        cx,
                        on_action,
                    )),
            )
    }

    fn render_menu_item<V: 'static>(
        label: &'static str,
        shortcut: Option<&'static str>,
        is_danger: bool,
        is_disabled: bool,
        action: SidebarMenuAction,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, SidebarMenuAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let text_color = if is_disabled {
            rgb(0x555555)
        } else if is_danger {
            rgb(0xeeeeee)
        } else {
            rgb(0xdddddd)
        };

        let hover_bg = if is_disabled {
            rgb(0x252528)
        } else if is_danger {
            rgb(0xe74c3c)
        } else {
            rgb(0x0066cc)
        };

        let hover_text = if is_disabled {
            rgb(0x555555)
        } else {
            rgb(0xffffff)
        };

        let mut row = div()
            .id(label)
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py_1()
            .rounded_sm()
            .text_xs()
            .text_color(text_color);


        if !is_disabled {
            row = row
                .cursor_pointer()
                .hover(move |s| s.bg(hover_bg).text_color(hover_text))
                .on_click(cx.listener(move |this, _, window, cx| {
                    on_action(this, action, window, cx);
                }));
        }

        row.child(div().child(label)).when_some(shortcut, |el, sc| {
            el.child(
                div()
                    .text_color(if is_disabled { rgb(0x444444) } else { rgb(0x888888) })
                    .child(sc),
            )
        })
    }

    fn render_divider() -> impl IntoElement {
        div().h(px(1.0)).bg(rgb(0x333333)).my_1()
    }
}
