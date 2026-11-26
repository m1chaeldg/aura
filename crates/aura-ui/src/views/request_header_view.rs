//! RequestHeader View: Method badge selector, URL input field, and Send/Cancel action trigger using gpui_kit.

use aura_core::model::HttpMethod;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::input::{Input, InputContentType, InputState};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::{ActiveTheme as _, Sizable, ThemeStyled as _, h_flex};

use crate::templating_ui::render_template_token;
use crate::theme::{icon, method_color};

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

    /// Determines whether the Save button should be visible in the request header.
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
        let btn_label = Self::button_label(is_in_flight);
        let send_btn = {
            let btn = Button::new("send-btn")
                .icon(icon(if is_in_flight {
                    IconName::Square
                } else {
                    IconName::Send
                }))
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
                .label(method.as_str())
                .dropdown_caret(true)
                .small()
                .rounded(ButtonRounded::Medium)
                .on_click(cx.listener(move |this, _, window, cx| {
                    on_toggle_dropdown(this, window, cx);
                }));
            match method {
                HttpMethod::GET => btn.info(),
                HttpMethod::POST => btn.success(),
                HttpMethod::PUT => btn.warning(),
                HttpMethod::PATCH => btn.primary(),
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
                    .popover_style(cx)
                    .absolute()
                    .top_full()
                    .left_0()
                    .mt_1()
                    .w(px(140.0))
                    .p_1()
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
                            .rounded(cx.theme().radius)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_select_method(this, m, window, cx);
                            }))
                            .child(
                                h_flex()
                                    .justify_between()
                                    .w_full()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(method_color(&m, cx))
                                            .child(m.as_str()),
                                    )
                                    .when(is_selected, |el| {
                                        el.child(
                                            icon(IconName::Check)
                                                .xsmall()
                                                .text_color(cx.theme().primary),
                                        )
                                    }),
                            )
                    }));
                el.child(deferred(menu).with_priority(2))
            });

        let mut header = h_flex()
            .id("request-header-container")
            .gap_3()
            .px_3()
            .py_2()
            .bg(cx.theme().background)
            .border_b_1()
            .border_color(cx.theme().border)
            .flex_shrink_0()
            .child(method_selector)
            .child(
                div().flex_1().min_w_0().child(
                    Input::new(url_input)
                        .id("url-text-input")
                        .content_type(InputContentType::Url)
                        .small()
                        .cleanable(true)
                        .token(render_template_token)
                        .w_full(),
                ),
            );

        if Self::is_save_button_visible(is_dirty) {
            let save_btn = Button::new("save-btn")
                .icon(icon(IconName::Save))
                .label("Save")
                .small()
                .outline()
                .rounded(ButtonRounded::Medium)
                .tooltip("Save Request (Ctrl+S)")
                .on_click(cx.listener(move |this, _, window, cx| {
                    on_save(this, window, cx);
                }));
            header = header.child(save_btn);
        }

        header.child(send_btn)
    }
}
