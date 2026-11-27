//! CommandPalette View: Keyboard-driven search modal overlay using gpui_kit.

use crate::app_state::AuraAppState;
use crate::templating_ui::render_templated_text;
use crate::theme::{icon, method_tag_variant};
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::separator::Separator;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Sizable, ThemeStyled as _, h_flex, v_flex};

pub struct CommandPaletteView;

impl CommandPaletteView {
    pub fn render<V: 'static>(
        state: &AuraAppState,
        search_input: &Entity<InputState>,
        cx: &mut Context<V>,
        on_select: impl Fn(&mut V, usize, &mut Window, &mut Context<V>) + 'static + Copy,
        on_dismiss: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let tpl_ctx = state.active_template_context();
        let filtered = state.filtered_palette_items();
        let sel_idx = state.selected_palette_index;
        let muted = cx.theme().muted_foreground;
        let fg = cx.theme().foreground;

        div()
            .id("command-palette-backdrop")
            .absolute()
            .inset_0()
            .flex()
            .items_start()
            .justify_center()
            .pt_16()
            .bg(cx.theme().overlay)
            .on_click(cx.listener(move |this, _, window, cx| {
                on_dismiss(this, window, cx);
            }))
            .child(
                v_flex()
                    .id("command-palette-modal")
                    .popover_style(cx)
                    .w(px(520.))
                    .overflow_hidden()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .child(
                        div().p_3().child(
                            Input::new(search_input)
                                .id("command-palette-search-input")
                                .prefix(icon(IconName::Search).xsmall())
                                .small()
                                .cleanable(true)
                                .w_full(),
                        ),
                    )
                    .child(Separator::horizontal())
                    .child(if filtered.is_empty() {
                        div()
                            .py_4()
                            .child(
                                Empty::new().header(EmptyHeader::new().description(
                                    EmptyDescription::new()
                                        .child("No matching commands or requests."),
                                )),
                            )
                            .into_any_element()
                    } else {
                        v_flex()
                            .id("command-palette-list-scroll")
                            .max_h(px(360.))
                            .overflow_y_scrollbar()
                            .py_1()
                            .children(filtered.iter().enumerate().map(|(idx, item)| {
                                let is_sel = idx == sel_idx;
                                let item_method = item.method;
                                let subtitle = item.subtitle.clone();
                                let tpl_ctx_row = tpl_ctx.clone();

                                ListItem::new(ElementId::NamedInteger(
                                    "palette-row".into(),
                                    idx as u64,
                                ))
                                .selected(is_sel)
                                .px_3()
                                .py_1p5()
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_select(this, idx, window, cx);
                                }))
                                .suffix(move |_, cx| {
                                    div()
                                        .text_xs()
                                        .max_w(px(220.))
                                        .truncate()
                                        .child(render_templated_text(
                                            ElementId::NamedInteger("palette-sub".into(), idx as u64),
                                            &subtitle,
                                            &tpl_ctx_row,
                                            muted,
                                            cx,
                                        ))
                                        .into_any_element()
                                })
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .child(if let Some(m) = item_method {
                                            Tag::new()
                                                .with_variant(method_tag_variant(&m))
                                                .outline()
                                                .xsmall()
                                                .child(m.as_str())
                                        } else {
                                            Tag::info().outline().xsmall().child("CMD")
                                        })
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(fg)
                                                .child(item.title.clone()),
                                        ),
                                )
                            }))
                            .into_any_element()
                    }),
            )
    }
}
