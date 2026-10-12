//! RequestHeader View: Method badge selector, URL input field, and Send action trigger.

use aura_core::model::HttpMethod;
use gpui::*;

pub struct RequestHeaderView;

impl RequestHeaderView {
    pub fn render<V: 'static>(
        method: HttpMethod,
        url: &str,
        cx: &mut Context<V>,
        on_send: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let border_color = rgb(0x333333);
        let surface_color = rgb(0x272729);
        let accent_blue = rgb(0x0066cc);

        let method_color = match method {
            HttpMethod::GET => rgb(0x0066cc),
            HttpMethod::POST => rgb(0x28a745),
            HttpMethod::PUT => rgb(0xf5a623),
            HttpMethod::PATCH => rgb(0x8e44ad),
            HttpMethod::DELETE => rgb(0xe74c3c),
            _ => rgb(0x7a7a7a),
        };

        div()
            .flex()
            .items_center()
            .gap_3()
            .px_4()
            .py_3()
            .bg(surface_color)
            .border_b_1()
            .border_color(border_color)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(method_color)
                    .text_xs()
                    .text_color(rgb(0xffffff))
                    .child(method.as_str()),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .items_center()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(rgb(0x1d1d1f))
                    .border_1()
                    .border_color(border_color)
                    .text_sm()
                    .text_color(rgb(0xffffff))
                    .child(url.to_string()),
            )
            .child(
                div()
                    .id("send-btn")
                    .flex()
                    .items_center()
                    .justify_center()
                    .px_5()
                    .py_2()
                    .rounded_md()
                    .bg(accent_blue)
                    .text_sm()
                    .text_color(rgb(0xffffff))
                    .child("Send ↵")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        on_send(this, window, cx);
                    })),
            )
    }
}
