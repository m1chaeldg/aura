use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::ActiveInputTarget;
use crate::input_registry::{InputKey, InputRegistry};
use aura_core::model::AuraRequest;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::Sizable;

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_settings_panel<V: 'static>(
        request: Option<&AuraRequest>,
        timeout_error: Option<&str>,
        max_redirects_error: Option<&str>,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let timeout = request.map(|r| r.settings.timeout_ms).unwrap_or(0);
        let redirects = request.map(|r| r.settings.follow_redirects).unwrap_or(true);
        let max_redirects = request.map(|r| r.settings.max_redirects).unwrap_or(10);
        let ssl = request.map(|r| r.settings.verify_ssl).unwrap_or(true);
        let inherit_auth = request.map(|r| r.settings.inherit_auth).unwrap_or(true);

        let is_timeout_focused = matches!(active_input, Some(ActiveInputTarget::SettingTimeout));
        let is_redirects_focused =
            matches!(active_input, Some(ActiveInputTarget::SettingMaxRedirects));

        let render_toggle_row =
            |id: &'static str,
             label: &'static str,
             desc: &'static str,
             enabled: bool,
             action: RequestEditorAction| {
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p_3()
                    .rounded_md()
                    .bg(rgb(0x272729))
                    .border_1()
                    .border_color(border)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(normal)
                                    .child(label),
                            )
                            .child(div().text_xs().text_color(muted).child(desc)),
                    )
                    .child({
                        let btn = Button::new(id)
                            .label(if enabled { "Enabled" } else { "Disabled" })
                            .small()
                            .rounded(ButtonRounded::Medium)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, action.clone(), window, cx);
                            }));
                        if enabled {
                            btn.primary()
                        } else {
                            btn.secondary()
                        }
                    })
            };

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(muted)
                    .child("REQUEST EXECUTION SETTINGS"),
            )
            // Timeout (ms)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1p5()
                    .p_3()
                    .rounded_md()
                    .bg(rgb(0x272729))
                    .border_1()
                    .border_color(if timeout_error.is_some() {
                        rgb(0xe74c3c)
                    } else if is_timeout_focused {
                        rgb(0x0066cc)
                    } else {
                        border
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(normal)
                                            .child("Request Timeout (ms)"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(muted)
                                            .child("0 = unlimited (no timeout), or 100 – 300000 ms"),
                                    ),
                            )
                            .child(
                                div().w(px(140.0)).child(Self::render_inline_input(
                                    "setting-timeout-input",
                                    inputs.get_input(&InputKey::SettingTimeout),
                                    &timeout.to_string(),
                                    is_timeout_focused,
                                    false,
                                    rgb(0x222224),
                                    rgb(0x38383a),
                                    normal,
                                )),
                            ),
                    )
                    .when_some(timeout_error, |el, err| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(rgb(0xe74c3c))
                                .child(err.to_string()),
                        )
                    }),
            )
            // Follow Redirects
            .child(render_toggle_row(
                "setting-follow-redirects-btn",
                "Follow Redirects",
                "Automatically follow HTTP 3xx redirect responses",
                redirects,
                RequestEditorAction::ToggleSettingFollowRedirects,
            ))
            // Max Redirects
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1p5()
                    .p_3()
                    .rounded_md()
                    .bg(rgb(0x272729))
                    .border_1()
                    .border_color(if max_redirects_error.is_some() {
                        rgb(0xe74c3c)
                    } else if is_redirects_focused {
                        rgb(0x0066cc)
                    } else {
                        border
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(normal)
                                            .child("Max Redirects"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(muted)
                                            .child("Valid range: 0 – 50 hops"),
                                    ),
                            )
                            .child(
                                div().w(px(140.0)).child(Self::render_inline_input(
                                    "setting-max-redirects-input",
                                    inputs.get_input(&InputKey::SettingMaxRedirects),
                                    &max_redirects.to_string(),
                                    is_redirects_focused,
                                    false,
                                    rgb(0x222224),
                                    rgb(0x38383a),
                                    normal,
                                )),
                            ),
                    )
                    .when_some(max_redirects_error, |el, err| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(rgb(0xe74c3c))
                                .child(err.to_string()),
                        )
                    }),
            )
            // Verify SSL
            .child(render_toggle_row(
                "setting-verify-ssl-btn",
                "Verify SSL Certificates",
                "Validate TLS/SSL certificates on HTTPS requests",
                ssl,
                RequestEditorAction::ToggleSettingVerifySsl,
            ))
            // Inherit Auth
            .child(render_toggle_row(
                "setting-inherit-auth-btn",
                "Inherit Collection Auth",
                "Use parent folder or collection authentication when Auth is Inherit",
                inherit_auth,
                RequestEditorAction::ToggleSettingInheritAuth,
            ))
    }

}
