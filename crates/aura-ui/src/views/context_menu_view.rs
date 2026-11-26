use crate::app_state::AuraAppState;
use crate::theme::icon;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::list::ListItem;
use gpui_kit::component::separator::Separator;
use gpui_kit::component::{ActiveTheme as _, Sizable as _, ThemeStyled as _, h_flex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarMenuAction {
    NewRequest,
    NewFolder,
    Duplicate,
    Copy,
    Paste,
    Rename,
    MoveTo,
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
                    .popover_style(cx)
                    .absolute()
                    .left(px(pos_x))
                    .top(px(pos_y))
                    .w(px(200.0))
                    .p_1()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(Self::render_menu_item(
                        "New Request",
                        IconName::FilePlus,
                        None,
                        false,
                        false,
                        SidebarMenuAction::NewRequest,
                        cx,
                        on_action,
                    ))
                    .child(Self::render_menu_item(
                        "New Folder",
                        IconName::FolderPlus,
                        None,
                        false,
                        false,
                        SidebarMenuAction::NewFolder,
                        cx,
                        on_action,
                    ))
                    .child(Separator::horizontal().my_1())
                    .child(Self::render_menu_item(
                        "Duplicate",
                        IconName::Copy,
                        Some("Ctrl+D"),
                        false,
                        false,
                        SidebarMenuAction::Duplicate,
                        cx,
                        on_action,
                    ))
                    .child(Self::render_menu_item(
                        "Copy",
                        IconName::Copy,
                        Some("Ctrl+C"),
                        false,
                        false,
                        SidebarMenuAction::Copy,
                        cx,
                        on_action,
                    ))
                    .child(Self::render_menu_item(
                        "Paste",
                        IconName::FileText,
                        Some("Ctrl+V"),
                        false,
                        !has_clipboard,
                        SidebarMenuAction::Paste,
                        cx,
                        on_action,
                    ))
                    .child(Separator::horizontal().my_1())
                    .child(Self::render_menu_item(
                        "Rename",
                        IconName::Pencil,
                        Some("F2"),
                        false,
                        false,
                        SidebarMenuAction::Rename,
                        cx,
                        on_action,
                    ))
                    .child(Self::render_menu_item(
                        "Move To...",
                        IconName::ArrowRight,
                        Some("Ctrl+M"),
                        false,
                        false,
                        SidebarMenuAction::MoveTo,
                        cx,
                        on_action,
                    ))
                    .child(Self::render_menu_item(
                        "Delete",
                        IconName::Trash,
                        Some("Del"),
                        true,
                        false,
                        SidebarMenuAction::Delete,
                        cx,
                        on_action,
                    )),
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_menu_item<V: 'static>(
        label: &'static str,
        icon_name: IconName,
        shortcut: Option<&'static str>,
        is_danger: bool,
        is_disabled: bool,
        action: SidebarMenuAction,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, SidebarMenuAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let fg = if is_disabled {
            cx.theme().muted_foreground.opacity(0.5)
        } else if is_danger {
            cx.theme().danger
        } else {
            cx.theme().popover_foreground
        };
        let muted = cx.theme().muted_foreground;

        let mut item = ListItem::new(label)
            .disabled(is_disabled)
            .px_2()
            .py_1()
            .rounded(cx.theme().radius)
            .text_xs()
            .child(
                h_flex()
                    .gap_2()
                    .text_color(fg)
                    .child(icon(icon_name).xsmall())
                    .child(label),
            );

        if let Some(sc) = shortcut {
            item = item.suffix(move |_, _| {
                div()
                    .text_xs()
                    .text_color(if is_disabled { muted.opacity(0.4) } else { muted })
                    .child(sc)
                    .into_any_element()
            });
        }

        if !is_disabled {
            item = item.on_click(cx.listener(move |this, _, window, cx| {
                on_action(this, action, window, cx);
            }));
        }

        item
    }
}
