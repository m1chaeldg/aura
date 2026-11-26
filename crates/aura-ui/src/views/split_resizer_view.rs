//! SplitResizer View: Draggable divider with hover feedback, drag resizing, and double-click 50/50 reset.

use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::ActiveTheme as _;

pub struct SplitResizerView;

impl SplitResizerView {
    pub fn render<V: 'static>(
        is_horizontal: bool,
        is_dragging: bool,
        cx: &mut Context<V>,
        on_mouse_down: impl Fn(&mut V, &MouseDownEvent, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let border_color = cx.theme().border;
        let surface_color = cx.theme().secondary.opacity(0.4);
        let divider_hover_color = cx.theme().primary_hover;
        let divider_active_color = cx.theme().primary;
        let grip_color = cx.theme().muted_foreground;

        div()
            .id("resizer-divider")
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .when(is_horizontal, |el| {
                el.w(px(6.0))
                    .cursor_col_resize()
                    .border_l_1()
                    .border_r_1()
                    .border_color(border_color)
                    .bg(if is_dragging {
                        divider_active_color
                    } else {
                        surface_color
                    })
                    .hover(move |s| s.bg(divider_hover_color))
                    .child(
                        div()
                            .w(px(2.0))
                            .h(px(28.0))
                            .rounded_full()
                            .bg(grip_color),
                    )
            })
            .when(!is_horizontal, |el| {
                el.h(px(6.0))
                    .cursor_row_resize()
                    .border_t_1()
                    .border_b_1()
                    .border_color(border_color)
                    .bg(if is_dragging {
                        divider_active_color
                    } else {
                        surface_color
                    })
                    .hover(move |s| s.bg(divider_hover_color))
                    .child(
                        div()
                            .h(px(2.0))
                            .w(px(28.0))
                            .rounded_full()
                            .bg(grip_color),
                    )
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event, window, cx| {
                    on_mouse_down(this, event, window, cx);
                }),
            )
    }
}
