//! RequestHeader View: Method badge selector, URL input field, and Send/Cancel action trigger using gpui_kit.

use aura_core::model::{HttpMethod, RequestProtocol};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::input::{Input, InputContentType, InputState};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::{ActiveTheme as _, Sizable, ThemeStyled as _, h_flex};

use crate::templating_ui::render_template_token;
use crate::theme::{icon, method_color, protocol_color};

const ALL_METHODS: [HttpMethod; 7] = [
    HttpMethod::GET,
    HttpMethod::POST,
    HttpMethod::PUT,
    HttpMethod::PATCH,
    HttpMethod::DELETE,
    HttpMethod::HEAD,
    HttpMethod::OPTIONS,
];

const ALL_PROTOCOLS: [RequestProtocol; 6] = [
    RequestProtocol::Http,
    RequestProtocol::Graphql,
    RequestProtocol::Websocket,
    RequestProtocol::Sse,
    RequestProtocol::Grpc,
    RequestProtocol::Soap,
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
        protocol: RequestProtocol,
        is_protocol_dropdown_open: bool,
        is_ws_connected: bool,
        method: HttpMethod,
        url_input: &Entity<InputState>,
        is_in_flight: bool,
        _is_url_focused: bool,
        is_dirty: bool,
        is_method_dropdown_open: bool,
        cx: &mut Context<V>,
        on_toggle_protocol_dropdown: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        on_select_protocol: impl Fn(&mut V, RequestProtocol, &mut Window, &mut Context<V>) + 'static + Copy,
        on_dismiss_protocol_dropdown: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        on_toggle_dropdown: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        on_select_method: impl Fn(&mut V, HttpMethod, &mut Window, &mut Context<V>) + 'static + Copy,
        on_dismiss_dropdown: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        _on_focus_url: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        on_send: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        on_cancel: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        on_save: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let is_ws = protocol == RequestProtocol::Websocket;
        let btn_label = if is_ws {
            if is_ws_connected {
                "Disconnect"
            } else {
                "Connect"
            }
        } else {
            Self::button_label(is_in_flight)
        };
        let send_btn = {
            let active_abort = if is_ws { is_ws_connected } else { is_in_flight };
            let btn = Button::new("send-btn")
                .icon(icon(if active_abort {
                    IconName::Square
                } else if is_ws {
                    IconName::Zap
                } else {
                    IconName::Send
                }))
                .label(btn_label)
                .small()
                .rounded(ButtonRounded::Medium)
                .on_click(cx.listener(move |this, _, window, cx| {
                    if active_abort {
                        on_cancel(this, window, cx);
                    } else {
                        on_send(this, window, cx);
                    }
                }));
            if active_abort {
                btn.danger()
            } else {
                btn.primary()
            }
        };

        let protocol_selector = div()
            .relative()
            .child(
                Button::new("protocol-selector-btn")
                    .label(protocol.as_str())
                    .dropdown_caret(true)
                    .small()
                    .outline()
                    .rounded(ButtonRounded::Medium)
                    .tooltip("Switch request protocol (HTTP, GraphQL, WebSocket, SSE, gRPC, SOAP)")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        on_toggle_protocol_dropdown(this, window, cx);
                    })),
            )
            .when(is_protocol_dropdown_open, |el| {
                let menu = div()
                    .id("protocol-dropdown-menu")
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
                        on_dismiss_protocol_dropdown(this, window, cx);
                    }))
                    .children(ALL_PROTOCOLS.iter().enumerate().map(|(idx, &p)| {
                        let is_selected = p == protocol;
                        ListItem::new(ElementId::NamedInteger("proto-opt".into(), idx as u64))
                            .selected(is_selected)
                            .px_2()
                            .py_1()
                            .rounded(cx.theme().radius)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_select_protocol(this, p, window, cx);
                            }))
                            .child(
                                h_flex()
                                    .justify_between()
                                    .w_full()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(protocol_color(p, cx))
                                            .child(p.as_str()),
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
            .child(protocol_selector)
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
