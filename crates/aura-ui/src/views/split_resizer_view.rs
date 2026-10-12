//! SplitResizer View: Draggable divider with hover feedback and double-click 50/50 reset.

use gpui::prelude::FluentBuilder;
use gpui::*;

pub struct SplitResizerView;

impl SplitResizerView {
    pub fn render<V: 'static>(
        is_horizontal: bool,
        cx: &mut Context<V>,
        on_reset: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let border_color = rgb(0x333333);
        let surface_color = rgb(0x272729);
        let divider_hover_color = rgb(0x0071e3);

        div()
            .id("resizer-divider")
            .flex()
            .items_center()
            .justify_center()
            .when(is_horizontal, |el| {
                el.w(px(6.0))
                    .border_l_1()
                    .border_r_1()
                    .border_color(border_color)
                    .bg(surface_color)
            })
            .when(!is_horizontal, |el| {
                el.h(px(6.0))
                    .border_t_1()
                    .border_b_1()
                    .border_color(border_color)
                    .bg(surface_color)
            })
            .on_click(cx.listener(move |this, _, window, cx| {
                on_reset(this, window, cx);
            }))
            .child(
                div()
                    .text_xs()
                    .text_color(divider_hover_color)
                    .child(if is_horizontal { "⋮" } else { "···" }),
            )
    }
}
