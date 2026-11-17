//! RequestHeader View: Method badge selector, URL input field, and Send/Cancel action trigger.

use aura_core::model::HttpMethod;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};

pub struct RequestHeaderView;

impl RequestHeaderView {
    pub fn button_label(is_in_flight: bool) -> &'static str {
        if is_in_flight {
            "Cancel"
        } else {
            "Send ↵"
        }
    }

    /// Determines whether the Save button should be visible in the request header
    pub fn is_save_button_visible(is_dirty: bool) -> bool {
        is_dirty
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render<V: 'static>(
        method: HttpMethod,
        url: &str,
        is_in_flight: bool,
        is_url_focused: bool,
        is_dirty: bool,
        cx: &mut Context<V>,
        on_cycle_method: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        on_focus_url: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        on_send: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        on_cancel: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        on_save: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let border_color = rgb(0x333333);
        let surface_color = rgb(0x272729);

        let method_color = match method {
            HttpMethod::GET => rgb(0x0066cc),
            HttpMethod::POST => rgb(0x28a745),
            HttpMethod::PUT => rgb(0xf5a623),
            HttpMethod::PATCH => rgb(0x8e44ad),
            HttpMethod::DELETE => rgb(0xe74c3c),
            _ => rgb(0x7a7a7a),
        };

        let btn_label = Self::button_label(is_in_flight);
        let mut send_btn = Button::new("send-btn")
            .label(btn_label)
            .rounded(ButtonRounded::Medium)
            .on_click(cx.listener(move |this, _, window, cx| {
                if is_in_flight {
                    on_cancel(this, window, cx);
                } else {
                    on_send(this, window, cx);
                }
            }));

        if is_in_flight {
            send_btn = send_btn.danger();
        } else {
            send_btn = send_btn.primary();
        }

        let url_border = if is_url_focused {
            rgb(0x0066cc)
        } else {
            border_color
        };

        let display_url = if url.is_empty() {
            if is_url_focused {
                "|"
            } else {
                "Enter request URL (e.g. https://postman-echo.com/get)..."
            }
        } else {
            url
        };

        let mut header = div()
            .id("request-header-container")
            .flex()
            .items_center()
            .gap_3()
            .px_4()
            .py_2p5()
            .bg(surface_color)
            .border_b_1()
            .border_color(border_color)
            .flex_shrink_0()
            .child(
                div()
                    .id("method-selector-btn")
                    .flex()
                    .items_center()
                    .justify_center()
                    .px_3()
                    .py_1p5()
                    .rounded_md()
                    .bg(method_color)
                    .hover(|s| s.opacity(0.85))
                    .cursor_pointer()
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xffffff))
                    .child(format!("{} ▾", method.as_str()))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        on_cycle_method(this, window, cx);
                    })),
            )
            .child(
                div()
                    .id("url-input-box")
                    .flex()
                    .flex_1()
                    .items_center()
                    .px_3()
                    .py_1p5()
                    .rounded_md()
                    .bg(rgb(0x1d1d1f))
                    .border_1()
                    .border_color(url_border)
                    .hover(|s| {
                        if !is_url_focused {
                            s.border_color(rgb(0x555555))
                        } else {
                            s
                        }
                    })
                    .cursor_text()
                    .text_sm()
                    .text_color(if url.is_empty() && !is_url_focused {
                        rgb(0x7a7a7a)
                    } else {
                        rgb(0xffffff)
                    })
                    .child(if is_url_focused && !url.is_empty() {
                        format!("{url}|")
                    } else {
                        display_url.to_string()
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        on_focus_url(this, window, cx);
                    })),
            );

        if Self::is_save_button_visible(is_dirty) {
            let save_btn = Button::new("save-btn")
                .label("Save")
                .rounded(ButtonRounded::Medium)
                .on_click(cx.listener(move |this, _, window, cx| {
                    on_save(this, window, cx);
                }));
            header = header.child(save_btn);
        }

        header.child(send_btn)
    }
}
