use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::ActiveInputTarget;
use crate::input_registry::{InputKey, InputRegistry};
use crate::theme::subtle_surface;
use aura_core::model::AuraRequest;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_settings_panel<V: 'static>(
        request: Option<&AuraRequest>,
        timeout_error: Option<&str>,
        max_redirects_error: Option<&str>,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
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

        let card_bg = subtle_surface(cx);
        let field_bg = cx.theme().background;
        let focus_border = cx.theme().primary;
        let danger_color = cx.theme().danger;
        let radius = cx.theme().radius;
        let view_entity = cx.entity().clone();

        let render_toggle_row =
            |id: &'static str,
             label: &'static str,
             desc: &'static str,
             enabled: bool,
             action: RequestEditorAction| {
                let entity = view_entity.clone();
                h_flex()
                    .justify_between()
                    .p_3()
                    .rounded(radius)
                    .bg(card_bg)
                    .border_1()
                    .border_color(border)
                    .child(
                        v_flex()
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
                    .child(
                        Switch::new(id)
                            .checked(enabled)
                            .small()
                            .on_change(move |_, window, app| {
                                let _ = entity.update(app, |this, cx| {
                                    on_action(this, action.clone(), window, cx);
                                });
                            }),
                    )
            };

        v_flex()
            .gap_3()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(muted)
                    .child("REQUEST EXECUTION SETTINGS"),
            )
            .child(
                v_flex()
                    .gap_1p5()
                    .p_3()
                    .rounded(radius)
                    .bg(card_bg)
                    .border_1()
                    .border_color(if timeout_error.is_some() {
                        danger_color
                    } else if is_timeout_focused {
                        focus_border
                    } else {
                        border
                    })
                    .child(
                        h_flex()
                            .justify_between()
                            .gap_3()
                            .child(
                                v_flex()
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
                                            .child("0 = unlimited (no timeout), or 100 - 300000 ms"),
                                    ),
                            )
                            .child(div().w(px(140.0)).child(Self::render_inline_input(
                                "setting-timeout-input",
                                inputs.get_input(&InputKey::SettingTimeout),
                                &timeout.to_string(),
                                is_timeout_focused,
                                false,
                                field_bg,
                                border,
                                normal,
                            ))),
                    )
                    .when_some(timeout_error, |el, err| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(danger_color)
                                .child(err.to_string()),
                        )
                    }),
            )
            .child(render_toggle_row(
                "setting-follow-redirects-btn",
                "Follow Redirects",
                "Automatically follow HTTP 3xx redirect responses",
                redirects,
                RequestEditorAction::ToggleSettingFollowRedirects,
            ))
            .child(
                v_flex()
                    .gap_1p5()
                    .p_3()
                    .rounded(radius)
                    .bg(card_bg)
                    .border_1()
                    .border_color(if max_redirects_error.is_some() {
                        danger_color
                    } else if is_redirects_focused {
                        focus_border
                    } else {
                        border
                    })
                    .child(
                        h_flex()
                            .justify_between()
                            .gap_3()
                            .child(
                                v_flex()
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
                                            .child("Valid range: 0 - 50 hops"),
                                    ),
                            )
                            .child(div().w(px(140.0)).child(Self::render_inline_input(
                                "setting-max-redirects-input",
                                inputs.get_input(&InputKey::SettingMaxRedirects),
                                &max_redirects.to_string(),
                                is_redirects_focused,
                                false,
                                field_bg,
                                border,
                                normal,
                            ))),
                    )
                    .when_some(max_redirects_error, |el, err| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(danger_color)
                                .child(err.to_string()),
                        )
                    }),
            )
            .child(render_toggle_row(
                "setting-verify-ssl-btn",
                "Verify SSL Certificates",
                "Validate TLS/SSL certificates on HTTPS requests",
                ssl,
                RequestEditorAction::ToggleSettingVerifySsl,
            ))
            .child(render_toggle_row(
                "setting-inherit-auth-btn",
                "Inherit Collection Auth",
                "Use parent folder or collection authentication when Auth is Inherit",
                inherit_auth,
                RequestEditorAction::ToggleSettingInheritAuth,
            ))
    }
}
