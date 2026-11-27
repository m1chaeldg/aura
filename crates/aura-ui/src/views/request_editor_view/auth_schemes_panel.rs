use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::ActiveInputTarget;
use crate::input_registry::{InputKey, InputRegistry};
use aura_core::model::AuthConfig;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{Sizable, h_flex, v_flex};

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_oauth2_panel<V: 'static>(
        auth: &AuthConfig,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        field_bg: Hsla,
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let AuthConfig::OAuth2 {
            grant_type,
            access_token,
            token_url,
            client_id,
            client_secret,
            auth_url,
            redirect_uri,
            scope,
            username,
            password,
            expires_at,
            use_pkce,
            auto_refresh,
            ..
        } = auth
        else {
            return v_flex();
        };

        let grants: [(&'static str, &'static str); 4] = [
            ("authorization_code", "Auth Code"),
            ("client_credentials", "Client Credentials"),
            ("password", "Password"),
            ("implicit", "Implicit"),
        ];

        let mut fields: Vec<(&'static str, String, usize, bool)> = vec![
            ("Access Token", access_token.clone(), 0, true),
            ("Grant Type", grant_type.clone(), 1, false),
            ("Token URL", token_url.clone().unwrap_or_default(), 2, false),
            ("Client ID", client_id.clone().unwrap_or_default(), 3, false),
            ("Client Secret", client_secret.clone().unwrap_or_default(), 4, true),
        ];

        let gt_lower = grant_type.trim().to_ascii_lowercase();
        if gt_lower == "authorization_code" || gt_lower == "implicit" || gt_lower.is_empty() {
            fields.push(("Authorize URL", auth_url.clone().unwrap_or_default(), 5, false));
            fields.push((
                "Redirect Callback URI",
                redirect_uri.clone().unwrap_or_default(),
                6,
                false,
            ));
        }
        fields.push(("Scope", scope.clone().unwrap_or_default(), 7, false));
        if gt_lower == "password" {
            fields.push(("Username", username.clone().unwrap_or_default(), 8, false));
            fields.push(("Password", password.clone().unwrap_or_default(), 9, true));
        }

        let now = chrono::Utc::now().timestamp();
        let expiry_tag = expires_at.map(|exp| {
            if exp <= now {
                Tag::danger().small().child("Token Expired")
            } else {
                let mins = ((exp - now) / 60).max(1);
                Tag::success()
                    .small()
                    .child(format!("Valid ({mins}m left)"))
            }
        });

        let view_pkce = cx.entity().clone();
        let view_refresh = cx.entity().clone();

        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child("OAUTH 2.0"),
                            )
                            .when_some(expiry_tag, |el, tag| el.child(tag)),
                    )
                    .child(
                        Button::new("oauth2-get-token-btn")
                            .label("Get New Access Token")
                            .xsmall()
                            .primary()
                            .rounded(ButtonRounded::Small)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::StartOAuth2Flow, window, cx);
                            })),
                    ),
            )
            .child(
                h_flex()
                    .gap_1p5()
                    .flex_wrap()
                    .children(grants.into_iter().map(|(gt, label)| {
                        let active = grant_type.eq_ignore_ascii_case(gt);
                        let btn = Button::new(ElementId::Name(format!("oauth-gt-{gt}").into()))
                            .label(label)
                            .xsmall()
                            .rounded(ButtonRounded::Small)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(
                                    this,
                                    RequestEditorAction::SetOAuth2GrantType(gt),
                                    window,
                                    cx,
                                );
                            }));
                        if active {
                            btn.primary()
                        } else {
                            btn.outline()
                        }
                    })),
            )
            .child(
                h_flex()
                    .gap_4()
                    .flex_wrap()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(
                                Switch::new("oauth2-pkce-switch")
                                    .checked(*use_pkce)
                                    .small()
                                    .on_change(move |_, window, app| {
                                        let _ = view_pkce.update(app, |this, cx| {
                                            on_action(
                                                this,
                                                RequestEditorAction::ToggleOAuth2Pkce,
                                                window,
                                                cx,
                                            );
                                        });
                                    }),
                            )
                            .child(div().text_xs().text_color(normal).child("Use PKCE (S256)")),
                    )
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(
                                Switch::new("oauth2-refresh-switch")
                                    .checked(*auto_refresh)
                                    .small()
                                    .on_change(move |_, window, app| {
                                        let _ = view_refresh.update(app, |this, cx| {
                                            on_action(
                                                this,
                                                RequestEditorAction::ToggleOAuth2AutoRefresh,
                                                window,
                                                cx,
                                            );
                                        });
                                    }),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(normal)
                                    .child("Auto-Refresh on Expiration"),
                            ),
                    ),
            )
            .children(fields.into_iter().map(|(label, val, f_idx, is_masked)| {
                let is_f = matches!(
                    active_input,
                    Some(ActiveInputTarget::AuthOAuth2Field(i)) if *i == f_idx
                );
                v_flex()
                    .gap_1()
                    .child(div().text_xs().text_color(muted).child(label))
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("input-oauth-field".into(), f_idx as u64),
                        inputs.get_input(&InputKey::AuthOAuth2Field(f_idx)),
                        &val,
                        is_f,
                        is_masked,
                        field_bg,
                        border,
                        normal,
                    ))
            }))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_extended_auth_scheme<V: 'static>(
        auth: &AuthConfig,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        field_bg: Hsla,
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let mut root = v_flex().gap_3();
        let mut fields: Vec<(&'static str, String, usize, bool)> = Vec::new();

        match auth {
            AuthConfig::Digest {
                username,
                password,
                realm,
                nonce,
                qop,
                algorithm,
            } => {
                root = root.child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(muted)
                        .child("DIGEST AUTH (AUTO-NEGOTIATES 401 CHALLENGE)"),
                );
                fields.push(("Username", username.clone(), 0, false));
                fields.push(("Password", password.clone(), 1, true));
                fields.push(("Realm (optional)", realm.clone().unwrap_or_default(), 2, false));
                fields.push(("Nonce (optional)", nonce.clone().unwrap_or_default(), 3, false));
                fields.push(("QoP (optional)", qop.clone().unwrap_or_default(), 4, false));
                fields.push((
                    "Algorithm (MD5 / MD5-sess / SHA-256)",
                    algorithm.clone().unwrap_or_default(),
                    5,
                    false,
                ));
            }
            AuthConfig::Ntlm {
                username,
                password,
                domain,
                workstation,
            } => {
                root = root.child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(muted)
                        .child("NTLM / NTLMv2 CHALLENGE-RESPONSE"),
                );
                fields.push(("Username", username.clone(), 0, false));
                fields.push(("Password", password.clone(), 1, true));
                fields.push(("Domain", domain.clone(), 2, false));
                fields.push(("Workstation", workstation.clone(), 3, false));
            }
            AuthConfig::OAuth1 {
                consumer_key,
                consumer_secret,
                token,
                token_secret,
                signature_method,
                callback_url,
                realm,
                add_to_query,
            } => {
                let view_q = cx.entity().clone();
                let methods: [&'static str; 3] = ["HMAC-SHA1", "HMAC-SHA256", "PLAINTEXT"];
                root = root
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(muted)
                            .child("OAUTH 1.0"),
                    )
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .flex_wrap()
                            .children(methods.into_iter().map(|m| {
                                let active = signature_method.eq_ignore_ascii_case(m);
                                let btn = Button::new(ElementId::Name(format!("oauth1-{m}").into()))
                                    .label(m)
                                    .xsmall()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            RequestEditorAction::SetOAuth1SigMethod(m),
                                            window,
                                            cx,
                                        );
                                    }));
                                if active { btn.primary() } else { btn.outline() }
                            }))
                            .child(
                                h_flex()
                                    .gap_1p5()
                                    .items_center()
                                    .ml_2()
                                    .child(
                                        Switch::new("oauth1-query-switch")
                                            .checked(*add_to_query)
                                            .small()
                                            .on_change(move |_, window, app| {
                                                let _ = view_q.update(app, |this, cx| {
                                                    on_action(
                                                        this,
                                                        RequestEditorAction::ToggleOAuth1Query,
                                                        window,
                                                        cx,
                                                    );
                                                });
                                            }),
                                    )
                                    .child(div().text_xs().text_color(normal).child("Add to Query")),
                            ),
                    );
                fields.push(("Consumer Key", consumer_key.clone(), 0, false));
                fields.push(("Consumer Secret", consumer_secret.clone(), 1, true));
                fields.push(("Access Token", token.clone(), 2, false));
                fields.push(("Token Secret", token_secret.clone(), 3, true));
                fields.push(("Callback URL", callback_url.clone().unwrap_or_default(), 4, false));
                fields.push(("Realm", realm.clone().unwrap_or_default(), 5, false));
            }
            AuthConfig::Hawk {
                id,
                key,
                algorithm,
                ext,
                app,
                dlg,
            } => {
                root = root
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(muted)
                            .child("HAWK AUTHENTICATION"),
                    )
                    .child(
                        h_flex()
                            .gap_1p5()
                            .children(["sha256", "sha1"].into_iter().map(|alg| {
                                let active = algorithm.eq_ignore_ascii_case(alg);
                                let btn = Button::new(ElementId::Name(format!("hawk-{alg}").into()))
                                    .label(alg)
                                    .xsmall()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            RequestEditorAction::SetHawkAlgorithm(alg),
                                            window,
                                            cx,
                                        );
                                    }));
                                if active { btn.primary() } else { btn.outline() }
                            })),
                    );
                fields.push(("Hawk ID", id.clone(), 0, false));
                fields.push(("Hawk Key", key.clone(), 1, true));
                fields.push(("Ext (optional)", ext.clone().unwrap_or_default(), 2, false));
                fields.push(("App (optional)", app.clone().unwrap_or_default(), 3, false));
                fields.push(("Dlg (optional)", dlg.clone().unwrap_or_default(), 4, false));
            }
            AuthConfig::AkamaiEdgeGrid {
                client_token,
                client_secret,
                access_token,
                max_body,
            } => {
                root = root.child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(muted)
                        .child("AKAMAI EDGEGRID (EG1-HMAC-SHA256)"),
                );
                fields.push(("Client Token", client_token.clone(), 0, false));
                fields.push(("Client Secret", client_secret.clone(), 1, true));
                fields.push(("Access Token", access_token.clone(), 2, false));
                fields.push(("Max Body Size", max_body.to_string(), 3, false));
            }
            AuthConfig::Wsse {
                username,
                password,
                digest_password,
            } => {
                let view_w = cx.entity().clone();
                root = root
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(muted)
                            .child("WSSE USERNAMETOKEN"),
                    )
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(
                                Switch::new("wsse-digest-switch")
                                    .checked(*digest_password)
                                    .small()
                                    .on_change(move |_, window, app| {
                                        let _ = view_w.update(app, |this, cx| {
                                            on_action(
                                                this,
                                                RequestEditorAction::ToggleWsseDigest,
                                                window,
                                                cx,
                                            );
                                        });
                                    }),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(normal)
                                    .child("SHA-1 PasswordDigest (recommended)"),
                            ),
                    );
                fields.push(("Username", username.clone(), 0, false));
                fields.push(("Password", password.clone(), 1, true));
            }
            _ => {}
        }

        root.children(fields.into_iter().map(|(label, val, f_idx, is_masked)| {
            let is_f = matches!(
                active_input,
                Some(ActiveInputTarget::AuthSchemeField(i)) if *i == f_idx
            );
            v_flex()
                .gap_1()
                .child(div().text_xs().text_color(muted).child(label))
                .child(Self::render_inline_input(
                    ElementId::NamedInteger("input-auth-scheme-field".into(), f_idx as u64),
                    inputs.get_input(&InputKey::AuthSchemeField(f_idx)),
                    &val,
                    is_f,
                    is_masked,
                    field_bg,
                    border,
                    normal,
                ))
        }))
    }
}
