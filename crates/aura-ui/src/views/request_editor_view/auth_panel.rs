use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::ActiveInputTarget;
use crate::input_registry::{InputKey, InputRegistry};
use crate::theme::subtle_surface;
use aura_core::model::{AuraRequest, AuthConfig};
use base64::Engine;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_auth_panel<V: 'static>(
        request: Option<&AuraRequest>,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
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

        let surface = subtle_surface(cx);
        let field_bg = cx.theme().background;
        let primary_color = cx.theme().primary;
        let success_color = cx.theme().success;
        let radius = cx.theme().radius;

        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .gap_1p5()
                    .flex_wrap()
                    .children(STRATEGIES.iter().copied().map(|strat| {
                        let is_active = active_auth_type == strat;
                        let btn = Button::new(strat)
                            .label(strat)
                            .xsmall()
                            .rounded(ButtonRounded::Small)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::SetAuthType(strat), window, cx);
                            }));
                        if is_active {
                            btn.primary()
                        } else {
                            btn.outline()
                        }
                    })),
            )
            .child(match auth {
                Some(AuthConfig::Inherit) | None => v_flex()
                    .p_4()
                    .rounded(radius)
                    .bg(surface)
                    .border_1()
                    .border_color(border)
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(normal)
                            .child("Inherit Auth from Parent"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted)
                            .child("This request automatically uses authentication settings defined on its parent collection or folder."),
                    ),
                Some(AuthConfig::None) => v_flex()
                    .p_4()
                    .rounded(radius)
                    .bg(surface)
                    .border_1()
                    .border_color(border)
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(normal)
                            .child("No Authentication"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted)
                            .child("No Authorization headers or authentication query parameters will be included in the request."),
                    ),
                Some(AuthConfig::Bearer { token }) => {
                    let is_token_focused =
                        matches!(active_input, Some(ActiveInputTarget::AuthBearerToken));
                    let is_jwt = token.split('.').count() == 3;
                    let decoded = if is_jwt {
                        aura_core::security::inspect_jwt(token).ok()
                    } else {
                        None
                    };

                    v_flex()
                        .gap_3()
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(muted)
                                        .child("BEARER TOKEN / JWT"),
                                )
                                .child(Self::render_inline_input(
                                    "input-bearer-token",
                                    inputs.get_input(&InputKey::AuthBearerToken),
                                    token,
                                    is_token_focused,
                                    false,
                                    field_bg,
                                    border,
                                    normal,
                                )),
                        )
                        .when_some(decoded, |el, jwt| {
                            el.child(
                                v_flex()
                                    .p_3()
                                    .rounded(radius)
                                    .bg(surface)
                                    .border_1()
                                    .border_color(primary_color)
                                    .gap_2()
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(
                                                h_flex()
                                                    .gap_2()
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .font_weight(FontWeight::SEMIBOLD)
                                                            .text_color(primary_color)
                                                            .child("JWT INSPECTOR"),
                                                    )
                                                    .child(
                                                        Tag::secondary()
                                                            .xsmall()
                                                            .child(format!("alg: {}", jwt.algorithm)),
                                                    ),
                                            )
                                            .child(if jwt.is_expired {
                                                Tag::danger().xsmall().child("EXPIRED")
                                            } else {
                                                Tag::success().xsmall().child("VALID")
                                            }),
                                    )
                                    .when_some(jwt.subject, |el, sub| {
                                        el.child(
                                            div()
                                                .text_xs()
                                                .text_color(muted)
                                                .child(format!("Subject (sub): {sub}")),
                                        )
                                    })
                                    .when_some(jwt.issuer, |el, iss| {
                                        el.child(
                                            div()
                                                .text_xs()
                                                .text_color(muted)
                                                .child(format!("Issuer (iss): {iss}")),
                                        )
                                    })
                                    .child(
                                        div()
                                            .p_2()
                                            .rounded(radius)
                                            .bg(field_bg)
                                            .font_family("monospace")
                                            .text_xs()
                                            .text_color(normal)
                                            .child(
                                                serde_json::to_string_pretty(&jwt.payload)
                                                    .unwrap_or_default(),
                                            ),
                                    ),
                            )
                        })
                }
                Some(AuthConfig::Basic { username, password }) => {
                    let is_user_focused =
                        matches!(active_input, Some(ActiveInputTarget::AuthBasicUsername));
                    let is_pass_focused =
                        matches!(active_input, Some(ActiveInputTarget::AuthBasicPassword));
                    let preview = {
                        let encoded = base64::engine::general_purpose::STANDARD
                            .encode(format!("{username}:{password}"));
                        format!("Authorization: Basic {encoded}")
                    };

                    v_flex()
                        .gap_3()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(muted)
                                .child("BASIC AUTHENTICATION"),
                        )
                        .child(
                            h_flex()
                                .gap_3()
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .gap_1()
                                        .child(div().text_xs().text_color(muted).child("Username"))
                                        .child(Self::render_inline_input(
                                            "input-basic-username",
                                            inputs.get_input(&InputKey::AuthBasicUsername),
                                            username,
                                            is_user_focused,
                                            false,
                                            field_bg,
                                            border,
                                            normal,
                                        )),
                                )
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .gap_1()
                                        .child(div().text_xs().text_color(muted).child("Password"))
                                        .child(Self::render_inline_input(
                                            "input-basic-password",
                                            inputs.get_input(&InputKey::AuthBasicPassword),
                                            password,
                                            is_pass_focused,
                                            true,
                                            field_bg,
                                            border,
                                            normal,
                                        )),
                                ),
                        )
                        .child(
                            div()
                                .p_2p5()
                                .rounded(radius)
                                .bg(field_bg)
                                .border_1()
                                .border_color(success_color)
                                .font_family("monospace")
                                .text_xs()
                                .text_color(success_color)
                                .child(preview),
                        )
                }
                Some(AuthConfig::ApiKey {
                    key,
                    value,
                    in_header,
                }) => {
                    let in_hdr = *in_header;
                    let is_key_focused =
                        matches!(active_input, Some(ActiveInputTarget::AuthApiKeyName));
                    let is_val_focused =
                        matches!(active_input, Some(ActiveInputTarget::AuthApiKeyValue));

                    v_flex()
                        .gap_3()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(muted)
                                .child("API KEY AUTHENTICATION"),
                        )
                        .child(
                            h_flex()
                                .gap_3()
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .gap_1()
                                        .child(div().text_xs().text_color(muted).child("Key Name"))
                                        .child(Self::render_inline_input(
                                            "input-apikey-name",
                                            inputs.get_input(&InputKey::AuthApiKeyName),
                                            key,
                                            is_key_focused,
                                            false,
                                            field_bg,
                                            border,
                                            normal,
                                        )),
                                )
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .gap_1()
                                        .child(div().text_xs().text_color(muted).child("Key Value"))
                                        .child(Self::render_inline_input(
                                            "input-apikey-val",
                                            inputs.get_input(&InputKey::AuthApiKeyValue),
                                            value,
                                            is_val_focused,
                                            false,
                                            field_bg,
                                            border,
                                            normal,
                                        )),
                                ),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .mt_1()
                                .child(div().text_xs().text_color(muted).child("Send In:"))
                                .child({
                                    let btn = Button::new("btn-apikey-hdr")
                                        .label("Header")
                                        .xsmall()
                                        .rounded(ButtonRounded::Small)
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(
                                                this,
                                                RequestEditorAction::ToggleApiKeyPlacement(true),
                                                window,
                                                cx,
                                            );
                                        }));
                                    if in_hdr {
                                        btn.primary()
                                    } else {
                                        btn.outline()
                                    }
                                })
                                .child({
                                    let btn = Button::new("btn-apikey-query")
                                        .label("Query Params")
                                        .xsmall()
                                        .rounded(ButtonRounded::Small)
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(
                                                this,
                                                RequestEditorAction::ToggleApiKeyPlacement(false),
                                                window,
                                                cx,
                                            );
                                        }));
                                    if !in_hdr {
                                        btn.primary()
                                    } else {
                                        btn.outline()
                                    }
                                }),
                        )
                }
                Some(AuthConfig::AwsSigV4 {
                    use_profile,
                    profile,
                    credentials_path,
                    access_key,
                    secret_key,
                    region,
                    service,
                    session_token,
                }) => {
                    let use_prof = *use_profile;
                    let fields: Vec<(&str, &str, usize)> = if use_prof {
                        vec![
                            ("AWS Profile (defaults to 'default')", profile.as_str(), 5),
                            (
                                "Credentials File Path (defaults to ~/.aws/credentials)",
                                credentials_path.as_deref().unwrap_or(""),
                                6,
                            ),
                            ("Region (or from ~/.aws/config)", region.as_str(), 2),
                            ("Service", service.as_str(), 3),
                        ]
                    } else {
                        vec![
                            ("Access Key", access_key.as_str(), 0),
                            ("Secret Key", secret_key.as_str(), 1),
                            ("Region", region.as_str(), 2),
                            ("Service", service.as_str(), 3),
                            ("Session Token", session_token.as_deref().unwrap_or(""), 4),
                        ]
                    };

                    v_flex()
                        .gap_3()
                        .child(
                            h_flex()
                                .justify_between()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(muted)
                                        .child("AWS SIGNATURE V4"),
                                )
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .child(
                                            Button::new("aws-mode-profile")
                                                .label("AWS Profile")
                                                .xsmall()
                                                .rounded(ButtonRounded::Small)
                                                .when(use_prof, |b| b.primary())
                                                .when(!use_prof, |b| b.outline())
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    on_action(
                                                        this,
                                                        RequestEditorAction::ToggleAwsUseProfile(true),
                                                        window,
                                                        cx,
                                                    );
                                                })),
                                        )
                                        .child(
                                            Button::new("aws-mode-manual")
                                                .label("Manual Keys")
                                                .xsmall()
                                                .rounded(ButtonRounded::Small)
                                                .when(!use_prof, |b| b.primary())
                                                .when(use_prof, |b| b.outline())
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    on_action(
                                                        this,
                                                        RequestEditorAction::ToggleAwsUseProfile(false),
                                                        window,
                                                        cx,
                                                    );
                                                })),
                                        ),
                                ),
                        )
                        .children(fields.into_iter().map(|(label, val, f_idx)| {
                            let is_f = matches!(active_input, Some(ActiveInputTarget::AuthAwsField(i)) if *i == f_idx);
                            v_flex()
                                .gap_1()
                                .child(div().text_xs().text_color(muted).child(label))
                                .child(Self::render_inline_input(
                                    ElementId::NamedInteger("input-aws-field".into(), f_idx as u64),
                                    inputs.get_input(&InputKey::AuthAwsField(f_idx)),
                                    val,
                                    is_f,
                                    f_idx == 1,
                                    field_bg,
                                    border,
                                    normal,
                                ))
                        }))
                }
                Some(AuthConfig::OAuth2 {
                    grant_type,
                    access_token,
                    token_url,
                    client_id,
                    client_secret,
                }) => {
                    let fields = [
                        ("Access Token", access_token.as_str(), 0),
                        ("Grant Type", grant_type.as_str(), 1),
                        ("Token URL", token_url.as_deref().unwrap_or(""), 2),
                        ("Client ID", client_id.as_deref().unwrap_or(""), 3),
                        ("Client Secret", client_secret.as_deref().unwrap_or(""), 4),
                    ];

                    v_flex()
                        .gap_3()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(muted)
                                .child("OAUTH 2.0"),
                        )
                        .children(fields.into_iter().map(|(label, val, f_idx)| {
                            let is_f = matches!(active_input, Some(ActiveInputTarget::AuthOAuth2Field(i)) if *i == f_idx);
                            v_flex()
                                .gap_1()
                                .child(div().text_xs().text_color(muted).child(label))
                                .child(Self::render_inline_input(
                                    ElementId::NamedInteger(
                                        "input-oauth-field".into(),
                                        f_idx as u64,
                                    ),
                                    inputs.get_input(&InputKey::AuthOAuth2Field(f_idx)),
                                    val,
                                    is_f,
                                    false,
                                    field_bg,
                                    border,
                                    normal,
                                ))
                        }))
                }
            })
    }
}
