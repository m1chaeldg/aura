//! RequestHeader View: Method badge selector, URL input field, and Send/Cancel action trigger using gpui_kit.

use aura_core::model::HttpMethod;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::input::{Input, InputContentType, InputState};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::Sizable;

const ALL_METHODS: [HttpMethod; 7] = [
    HttpMethod::GET,
    HttpMethod::POST,
    HttpMethod::PUT,
    HttpMethod::PATCH,
    HttpMethod::DELETE,
    HttpMethod::HEAD,
    HttpMethod::OPTIONS,
];

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

    fn method_color(method: HttpMethod) -> Rgba {
        match method {
            HttpMethod::GET => rgb(0x2997ff),
            HttpMethod::POST => rgb(0x28a745),
            HttpMethod::PUT => rgb(0xf5a623),
            HttpMethod::PATCH => rgb(0x8e44ad),
            HttpMethod::DELETE => rgb(0xe74c3c),
            HttpMethod::HEAD => rgb(0x9e9ea4),
            HttpMethod::OPTIONS => rgb(0x17a2b8),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render<V: 'static>(
        method: HttpMethod,
        url_input: &Entity<InputState>,
        is_in_flight: bool,
        _is_url_focused: bool,
        is_dirty: bool,
        is_method_dropdown_open: bool,
        cx: &mut Context<V>,
        on_toggle_dropdown: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        on_select_method: impl Fn(&mut V, HttpMethod, &mut Window, &mut Context<V>) + 'static + Copy,
        on_dismiss_dropdown: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
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
                    on_toggle_dropdown(this, window, cx);
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

        let method_selector = div()
            .relative()
            .child(method_btn)
            .when(is_method_dropdown_open, |el| {
                let menu = div()
                    .id("method-dropdown-menu")
                    .absolute()
                    .top_full()
                    .left_0()
                    .mt_1()
                    .w(px(136.0))
                    .bg(rgb(0x252528))
                    .border_1()
                    .border_color(rgb(0x333333))
                    .rounded_md()
                    .p_1()
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .on_mouse_down_out(cx.listener(move |this, _, window, cx| {
                        on_dismiss_dropdown(this, window, cx);
                    }))
                    .children(ALL_METHODS.iter().enumerate().map(|(idx, &m)| {
                        let is_selected = m == method;
                        ListItem::new(ElementId::NamedInteger("method-opt".into(), idx as u64))
                            .selected(is_selected)
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_select_method(this, m, window, cx);
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .w_full()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(Self::method_color(m))
                                            .child(m.as_str()),
                                    )
                                    .when(is_selected, |el| {
                                        el.child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0x2997ff))
                                                .child("✓"),
                                        )
                                    }),
                            )
                    }));
                el.child(deferred(menu).with_priority(2))
            });

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
            .child(method_selector)
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
