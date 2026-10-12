//! ResponsePane View: Status pills, duration, size, and formatted response body.

use gpui::prelude::FluentBuilder;
use gpui::*;

pub struct ResponsePaneView;

impl ResponsePaneView {
    pub fn render(split_ratio: f32, is_horizontal: bool) -> impl IntoElement {
        let border_color = rgb(0x333333);
        let green_color = rgb(0x28a745);
        let text_muted = rgb(0x7a7a7a);

        div()
            .flex()
            .flex_col()
            .when(is_horizontal, |el| el.w(gpui::relative(1.0 - split_ratio)))
            .when(!is_horizontal, |el| el.h(gpui::relative(1.0 - split_ratio)))
            .p_4()
            .bg(rgb(0x1b1b1d))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(border_color)
                    .py_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(green_color)
                            .child("RESPONSE PANE"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(text_muted)
                            .child("Status: 200 OK • Time: --ms • Size: --B"),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .py_3()
                    .text_xs()
                    .text_color(text_muted)
                    .child("Ready to dispatch request to live Postman Echo endpoints."),
            )
    }
}
