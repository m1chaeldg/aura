use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::ActiveInputTarget;
use crate::input_registry::{InputKey, InputRegistry};
use aura_core::model::{AuraRequest, AuthConfig};
use base64::Engine;
use gpui::prelude::FluentBuilder;
use gpui::*;

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_auth_panel<V: 'static>(
        request: Option<&AuraRequest>,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let auth = request.map(|r| &r.auth);
        let active_auth_type = auth.map(|a| a.type_name()).unwrap_or("Inherit");
        const STRATEGIES: [&str; 7] = [
            "Inherit",
            "None",
            "Bearer Token",
            "Basic Auth",
            "API Key",
            "AWS SigV4",
            "OAuth 2.0",
        ];

        div()
            .flex()
            .flex_col()
            .gap_3()
            // Strategy Selector Bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .flex_wrap()
                    .children(STRATEGIES.iter().copied().map(|strat| {
                        let is_active = active_auth_type == strat;
                        div()
                            .id(strat)
                            .px_2p5()
                            .py_1()
                            .rounded_sm()
                            .bg(if is_active { rgb(0x0066cc) } else { rgb(0x272729) })
                            .border_1()
                            .border_color(if is_active { rgb(0x0071e3) } else { border })
                            .hover(|s| s.bg(rgb(0x333333)))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(if is_active { rgb(0xffffff) } else { muted })
                            .child(strat)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::SetAuthType(strat), window, cx);
                            }))
                    })),
            )
            // Direct Inline Auth Configuration
            .child(match auth {
                Some(AuthConfig::Inherit) | None => {
                    div()
                        .p_4()
                        .rounded_md()
                        .bg(rgb(0x222224))
                        .border_1()
                        .border_color(border)
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(rgb(0xffffff)).child("Inherit Auth from Parent"))
                        .child(div().text_xs().text_color(muted).child("This request will automatically use the authentication settings defined on its parent collection or folder."))
                }
                Some(AuthConfig::None) => {
                    div()
                        .p_4()
                        .rounded_md()
                        .bg(rgb(0x222224))
                        .border_1()
                        .border_color(border)
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(rgb(0xffffff)).child("No Authentication"))
                        .child(div().text_xs().text_color(muted).child("No Authorization headers or authentication query parameters will be included in the request."))
                }
                Some(AuthConfig::Bearer { token }) => {
                    let is_token_focused = matches!(active_input, Some(ActiveInputTarget::AuthBearerToken));
                    let is_jwt = token.split('.').count() == 3;
                    let decoded = if is_jwt {
                        aura_core::security::inspect_jwt(token).ok()
                    } else {
                        None
                    };

                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("BEARER TOKEN / JWT"))
                                .child(Self::render_inline_input(
                                    "input-bearer-token",
                                    inputs.get_input(&InputKey::AuthBearerToken),
                                    token,
                                    is_token_focused,
                                    false,
                                    rgb(0x18181a),
                                    border,
                                    if is_token_focused { rgb(0xffffff) } else { normal },
                                )),
                        )
                        .when_some(decoded, |el, jwt| {
                            el.child(
                                div()
                                    .p_3()
                                    .rounded_md()
                                    .bg(rgb(0x222224))
                                    .border_1()
                                    .border_color(rgb(0x0066cc))
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_2()
                                                    .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(rgb(0x2997ff)).child("JWT INSPECTOR"))
                                                    .child(div().px_1p5().py_0p5().rounded_sm().bg(rgb(0x333333)).text_xs().text_color(rgb(0xffffff)).child(format!("alg: {}", jwt.algorithm))),
                                            )
                                            .child(
                                                div()
                                                    .px_2()
                                                    .py_0p5()
                                                    .rounded_sm()
                                                    .bg(if jwt.is_expired { rgb(0xe74c3c) } else { rgb(0x28a745) })
                                                    .text_xs()
                                                    .text_color(rgb(0xffffff))
                                                    .child(if jwt.is_expired { "EXPIRED" } else { "VALID" }),
                                            ),
                                    )
                                    .when_some(jwt.subject, |el, sub| {
                                        el.child(div().text_xs().text_color(muted).child(format!("Subject (sub): {sub}")))
                                    })
                                    .when_some(jwt.issuer, |el, iss| {
                                        el.child(div().text_xs().text_color(muted).child(format!("Issuer (iss): {iss}")))
                                    })
                                    .child(
                                        div()
                                            .p_2()
                                            .rounded_sm()
                                            .bg(rgb(0x18181a))
                                            .font_family("monospace")
                                            .text_xs()
                                            .text_color(rgb(0xcccccc))
                                            .child(serde_json::to_string_pretty(&jwt.payload).unwrap_or_default()),
                                    ),
                            )
                        })
                }
                Some(AuthConfig::Basic { username, password }) => {
                    let is_user_focused = matches!(active_input, Some(ActiveInputTarget::AuthBasicUsername));
                    let is_pass_focused = matches!(active_input, Some(ActiveInputTarget::AuthBasicPassword));
                    let preview = {
                        let encoded = base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
                        format!("Authorization: Basic {encoded}")
                    };

                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("BASIC AUTHENTICATION"))
                        .child(
                            div()
                                .flex()
                                .gap_3()
                                // Username input
                                .child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(div().text_xs().text_color(muted).child("Username"))
                                        .child(Self::render_inline_input(
                                            "input-basic-username",
                                            inputs.get_input(&InputKey::AuthBasicUsername),
                                            username,
                                            is_user_focused,
                                            false,
                                            rgb(0x18181a),
                                            border,
                                            if is_user_focused { rgb(0xffffff) } else { normal },
                                        )),
                                )
                                // Password input
                                .child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(div().text_xs().text_color(muted).child("Password"))
                                        .child(Self::render_inline_input(
                                            "input-basic-password",
                                            inputs.get_input(&InputKey::AuthBasicPassword),
                                            password,
                                            is_pass_focused,
                                            true,
                                            rgb(0x18181a),
                                            border,
                                            if is_pass_focused { rgb(0xffffff) } else { normal },
                                        )),
                                ),
                        )
                        // Live Base64 Preview
                        .child(
                            div()
                                .p_2p5()
                                .rounded_sm()
                                .bg(rgb(0x18181a))
                                .border_1()
                                .border_color(rgb(0x28a745))
                                .font_family("monospace")
                                .text_xs()
                                .text_color(rgb(0x28a745))
                                .child(preview),
                        )
                }
                Some(AuthConfig::ApiKey { key, value, in_header }) => {
                    let in_hdr = *in_header;
                    let is_key_focused = matches!(active_input, Some(ActiveInputTarget::AuthApiKeyName));
                    let is_val_focused = matches!(active_input, Some(ActiveInputTarget::AuthApiKeyValue));

                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("API KEY AUTHENTICATION"))
                        .child(
                            div()
                                .flex()
                                .gap_3()
                                // Key Name
                                .child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(div().text_xs().text_color(muted).child("Key Name"))
                                        .child(Self::render_inline_input(
                                            "input-apikey-name",
                                            inputs.get_input(&InputKey::AuthApiKeyName),
                                            key,
                                            is_key_focused,
                                            false,
                                            rgb(0x18181a),
                                            border,
                                            if is_key_focused { rgb(0xffffff) } else { normal },
                                        )),
                                )
                                // Key Value
                                .child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(div().text_xs().text_color(muted).child("Key Value"))
                                        .child(Self::render_inline_input(
                                            "input-apikey-val",
                                            inputs.get_input(&InputKey::AuthApiKeyValue),
                                            value,
                                            is_val_focused,
                                            false,
                                            rgb(0x18181a),
                                            border,
                                            if is_val_focused { rgb(0xffffff) } else { normal },
                                        )),
                                ),
                        )
                        // Placement Toggle
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .mt_1()
                                .child(div().text_xs().text_color(muted).child("Send In:"))
                                .child(
                                    div()
                                        .id("btn-apikey-hdr")
                                        .px_2()
                                        .py_0p5()
                                        .rounded_sm()
                                        .bg(if in_hdr { rgb(0x0066cc) } else { rgb(0x333333) })
                                        .hover(|s| s.bg(rgb(0x444444)))
                                        .cursor_pointer()
                                        .text_xs()
                                        .text_color(rgb(0xffffff))
                                        .child("Header")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(this, RequestEditorAction::ToggleApiKeyPlacement(true), window, cx);
                                        })),
                                )
                                .child(
                                    div()
                                        .id("btn-apikey-query")
                                        .px_2()
                                        .py_0p5()
                                        .rounded_sm()
                                        .bg(if !in_hdr { rgb(0x0066cc) } else { rgb(0x333333) })
                                        .hover(|s| s.bg(rgb(0x444444)))
                                        .cursor_pointer()
                                        .text_xs()
                                        .text_color(rgb(0xffffff))
                                        .child("Query Params")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(this, RequestEditorAction::ToggleApiKeyPlacement(false), window, cx);
                                        })),
                                ),
                        )
                }
                Some(AuthConfig::AwsSigV4 { access_key, secret_key, region, service, session_token }) => {
                    let fields = [
                        ("Access Key", access_key.as_str(), 0),
                        ("Secret Key", secret_key.as_str(), 1),
                        ("Region", region.as_str(), 2),
                        ("Service", service.as_str(), 3),
                        ("Session Token", session_token.as_deref().unwrap_or(""), 4),
                    ];

                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("AWS SIGNATURE V4"))
                        .children(fields.into_iter().map(|(label, val, f_idx)| {
                            let is_f = matches!(active_input, Some(ActiveInputTarget::AuthAwsField(i)) if *i == f_idx);
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(div().text_xs().text_color(muted).child(label))
                                .child(Self::render_inline_input(
                                    ElementId::NamedInteger("input-aws-field".into(), f_idx as u64),
                                    inputs.get_input(&InputKey::AuthAwsField(f_idx)),
                                    val,
                                    is_f,
                                    false,
                                    rgb(0x18181a),
                                    border,
                                    if is_f { rgb(0xffffff) } else { normal },
                                ))
                        }))
                }
                Some(AuthConfig::OAuth2 { grant_type, access_token, token_url, client_id, client_secret }) => {
                    let fields = [
                        ("Access Token", access_token.as_str(), 0),
                        ("Grant Type", grant_type.as_str(), 1),
                        ("Token URL", token_url.as_deref().unwrap_or(""), 2),
                        ("Client ID", client_id.as_deref().unwrap_or(""), 3),
                        ("Client Secret", client_secret.as_deref().unwrap_or(""), 4),
                    ];

                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("OAUTH 2.0"))
                        .children(fields.into_iter().map(|(label, val, f_idx)| {
                            let is_f = matches!(active_input, Some(ActiveInputTarget::AuthOAuth2Field(i)) if *i == f_idx);
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(div().text_xs().text_color(muted).child(label))
                                .child(Self::render_inline_input(
                                    ElementId::NamedInteger("input-oauth-field".into(), f_idx as u64),
                                    inputs.get_input(&InputKey::AuthOAuth2Field(f_idx)),
                                    val,
                                    is_f,
                                    false,
                                    rgb(0x18181a),
                                    border,
                                    if is_f { rgb(0xffffff) } else { normal },
                                ))
                        }))
                }
            })
    }

    // =========================================================================
    // Scripts Panel (Sub-tabs for Pre & Post Scripts + Quick Snippets)
    // =========================================================================

}
