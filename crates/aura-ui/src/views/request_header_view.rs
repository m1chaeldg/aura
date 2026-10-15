//! RequestHeader View: Method badge selector, URL input field, and Send/Cancel action trigger.

use aura_core::model::HttpMethod;
use gpui::*;

pub struct RequestHeaderView;

impl RequestHeaderView {
    pub fn button_label(is_in_flight: bool) -> &'static str {
        if is_in_flight {
            "Cancel"
        } else {
            "Send ↵"
        }
    }

    pub fn render<V: 'static>(
        method: HttpMethod,
        url: &str,
        is_in_flight: bool,
        cx: &mut Context<V>,
        on_send: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        on_cancel: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let border_color = rgb(0x333333);
        let surface_color = rgb(0x272729);
        let accent_blue = rgb(0x0066cc);
        let cancel_red = rgb(0xe74c3c);

        let method_color = match method {
            HttpMethod::GET => rgb(0x0066cc),
            HttpMethod::POST => rgb(0x28a745),
            HttpMethod::PUT => rgb(0xf5a623),
            HttpMethod::PATCH => rgb(0x8e44ad),
            HttpMethod::DELETE => rgb(0xe74c3c),
            _ => rgb(0x7a7a7a),
        };

        let btn_bg = if is_in_flight { cancel_red } else { accent_blue };
        let btn_label = Self::button_label(is_in_flight);

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
                    .bg(btn_bg)
                    .hover(|s| s.bg(if is_in_flight { rgb(0xd03828) } else { rgb(0x0071e3) }))
                    .text_sm()
                    .text_color(rgb(0xffffff))
                    .child(btn_label)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if is_in_flight {
                            on_cancel(this, window, cx);
                        } else {
                            on_send(this, window, cx);
                        }
                    })),
            )
    }
}
