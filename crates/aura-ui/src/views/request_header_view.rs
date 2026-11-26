//! RequestHeader View: Method badge selector, URL input field, and Send/Cancel action trigger using gpui_kit.

use aura_core::model::HttpMethod;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::input::{Input, InputContentType, InputState};
use gpui_kit::component::Sizable;

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
        url_input: &Entity<InputState>,
        is_in_flight: bool,
        _is_url_focused: bool,
        is_dirty: bool,
        cx: &mut Context<V>,
        on_cycle_method: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        _on_focus_url: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        on_send: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        on_cancel: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        on_save: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let border_color = rgb(0x333333);
        let surface_color = rgb(0x272729);

        let btn_label = Self::button_label(is_in_flight);
        let send_btn = {
            let btn = Button::new("send-btn")
                .label(btn_label)
                .small()
                .rounded(ButtonRounded::Medium)
                .on_click(cx.listener(move |this, _, window, cx| {
                    if is_in_flight {
                        on_cancel(this, window, cx);
                    } else {
                        on_send(this, window, cx);
                    }
                }));
            if is_in_flight {
                btn.danger()
            } else {
                btn.primary()
            }
        };

        let method_btn = {
            let btn = Button::new("method-selector-btn")
                .label(format!("{} ▾", method.as_str()))
                .small()
                .rounded(ButtonRounded::Medium)
                .on_click(cx.listener(move |this, _, window, cx| {
                    on_cycle_method(this, window, cx);
                }));
            match method {
                HttpMethod::GET => btn.primary(),
                HttpMethod::POST => btn.success(),
                HttpMethod::PUT => btn.warning(),
                HttpMethod::PATCH => btn.info(),
                HttpMethod::DELETE => btn.danger(),
                _ => btn.secondary(),
            }
        };

        let mut header = div()
            .id("request-header-container")
            .flex()
            .items_center()
            .gap_3()
            .px_4()
            .py_2()
            .bg(surface_color)
            .border_b_1()
            .border_color(border_color)
            .flex_shrink_0()
            .child(method_btn)
            .child(
                div().flex_1().child(
                    Input::new(url_input)
                        .id("url-text-input")
                        .content_type(InputContentType::Url)
                        .small()
                        .cleanable(true)
                        .w_full(),
                ),
            );

        if Self::is_save_button_visible(is_dirty) {
            let save_btn = Button::new("save-btn")
                .label("Save")
                .small()
                .rounded(ButtonRounded::Medium)
                .on_click(cx.listener(move |this, _, window, cx| {
                    on_save(this, window, cx);
                }));
            header = header.child(save_btn);
        }

        header.child(send_btn)
    }
}
