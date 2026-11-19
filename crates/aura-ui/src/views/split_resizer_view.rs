//! SplitResizer View: Draggable divider with hover feedback, drag resizing, and double-click 50/50 reset.

use gpui::prelude::FluentBuilder;
use gpui::*;

pub struct SplitResizerView;

impl SplitResizerView {
    pub fn render<V: 'static>(
        is_horizontal: bool,
        is_dragging: bool,
        cx: &mut Context<V>,
        on_mouse_down: impl Fn(&mut V, &MouseDownEvent, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let border_color = rgb(0x333333);
        let surface_color = rgb(0x272729);
        let divider_hover_color = rgb(0x0071e3);
        let divider_active_color = rgb(0x2997ff);

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
                    .hover(|s| s.bg(divider_hover_color))
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
                    .hover(|s| s.bg(divider_hover_color))
            })
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    on_mouse_down(this, event, window, cx);
                }),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(if is_dragging {
                        rgb(0xffffff)
                    } else {
                        divider_hover_color
                    })
                    .child(if is_horizontal { "⋮" } else { "···" }),
            )
    }
}
