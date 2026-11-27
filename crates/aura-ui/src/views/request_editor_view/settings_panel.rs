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
        let client_cert = request.and_then(|r| r.settings.client_cert.as_ref());
        let cert_enabled = client_cert.map(|c| c.enabled).unwrap_or(false);
        let cert_path = client_cert.map(|c| c.cert_path.as_str()).unwrap_or("");
        let key_path = client_cert
            .and_then(|c| c.key_path.as_deref())
            .unwrap_or("");
        let cert_pass = client_cert
            .and_then(|c| c.passphrase.as_deref())
            .unwrap_or("");
        let req_proxy = request.and_then(|r| r.settings.proxy.as_ref());
        let proxy_enabled = req_proxy.map(|p| p.enabled).unwrap_or(false);
        let proxy_proto = req_proxy
            .map(|p| p.protocol)
            .unwrap_or(aura_core::model::ProxyProtocol::Http);
        let proxy_host = req_proxy.map(|p| p.host.as_str()).unwrap_or("");
        let proxy_port = req_proxy.map(|p| p.port.to_string()).unwrap_or_else(|| "8080".to_string());
        let proxy_auth = req_proxy.map(|p| p.auth_enabled).unwrap_or(false);
        let proxy_user = req_proxy.map(|p| p.username.as_str()).unwrap_or("");
        let proxy_pass = req_proxy.map(|p| p.password.as_str()).unwrap_or("");
        let proxy_bypass = req_proxy
            .map(|p| p.bypass_list.join(", "))
            .unwrap_or_default();

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
            .child(
                v_flex()
                    .gap_2()
                    .p_3()
                    .rounded(radius)
                    .bg(card_bg)
                    .border_1()
                    .border_color(border)
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                v_flex()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(normal)
                                            .child("Client Certificate (mTLS)"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(muted)
                                            .child("Per-request .pem, .pfx, or .p12 certificate and passphrase"),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        gpui_kit::component::button::Button::new("open-vault-certs-btn")
                                            .label("Domain Certs & Vaults")
                                            .xsmall()
                                            .outline()
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                on_action(
                                                    this,
                                                    RequestEditorAction::OpenSecurityVaultModal,
                                                    window,
                                                    cx,
                                                );
                                            })),
                                    )
                                    .child({
                                        let ent = cx.entity().clone();
                                        Switch::new("setting-cert-enable-switch")
                                            .checked(cert_enabled)
                                            .small()
                                            .on_change(move |_, window, app| {
                                                let _ = ent.update(app, |this, cx| {
                                                    on_action(
                                                        this,
                                                        RequestEditorAction::ToggleRequestCertEnabled,
                                                        window,
                                                        cx,
                                                    );
                                                });
                                            })
                                    }),
                            ),
                    )
                    .when(cert_enabled, |panel| {
                        let cert_fields = [
                            ("Certificate Path (.pem, .pfx, .p12)", cert_path, 0usize, false),
                            ("Private Key Path (optional for PEM)", key_path, 1usize, false),
                            ("Passphrase (optional)", cert_pass, 2usize, true),
                        ];
                        panel.children(cert_fields.into_iter().map(
                            |(lbl, val, idx, is_masked)| {
                                let is_f = matches!(
                                    active_input,
                                    Some(ActiveInputTarget::SettingCertField(i)) if *i == idx
                                );
                                v_flex()
                                    .gap_1()
                                    .child(div().text_xs().text_color(muted).child(lbl))
                                    .child(Self::render_inline_input(
                                        ElementId::NamedInteger(
                                            "setting-cert-field".into(),
                                            idx as u64,
                                        ),
                                        inputs.get_input(&InputKey::SettingCertField(idx)),
                                        val,
                                        is_f,
                                        is_masked,
                                        field_bg,
                                        border,
                                        normal,
                                    ))
                            },
                        ))
                    }),
            )
            .child(
                v_flex()
                    .gap_2()
                    .p_3()
                    .rounded(radius)
                    .bg(card_bg)
                    .border_1()
                    .border_color(border)
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                v_flex()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(normal)
                                            .child("Request Proxy Override (HTTP / SOCKS5)"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(muted)
                                            .child("Override global proxy for this request"),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        gpui_kit::component::button::Button::new(
                                            "open-global-proxy-btn",
                                        )
                                        .label("Global Proxy")
                                        .xsmall()
                                        .outline()
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(
                                                this,
                                                RequestEditorAction::OpenGlobalProxyModal,
                                                window,
                                                cx,
                                            );
                                        })),
                                    )
                                    .child({
                                        let ent = cx.entity().clone();
                                        Switch::new("setting-proxy-enable-switch")
                                            .checked(proxy_enabled)
                                            .small()
                                            .on_change(move |_, window, app| {
                                                let _ = ent.update(app, |this, cx| {
                                                    on_action(
                                                        this,
                                                        RequestEditorAction::ToggleRequestProxyEnabled,
                                                        window,
                                                        cx,
                                                    );
                                                });
                                            })
                                    }),
                            ),
                    )
                    .when(proxy_enabled, |panel| {
                        use aura_core::model::ProxyProtocol;
                        use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
                        let ent_auth = cx.entity().clone();
                        let protos = [
                            ProxyProtocol::Http,
                            ProxyProtocol::Https,
                            ProxyProtocol::Socks5,
                            ProxyProtocol::Socks5h,
                        ];
                        let proxy_fields: [(&'static str, &str, usize, bool); 5] = [
                            ("Proxy Host", proxy_host, 0, false),
                            ("Proxy Port", proxy_port.as_str(), 1, false),
                            ("Bypass Hosts (comma-separated)", proxy_bypass.as_str(), 4, false),
                            ("Username (if auth enabled)", proxy_user, 2, false),
                            ("Password (if auth enabled)", proxy_pass, 3, true),
                        ];
                        panel
                            .child(
                                h_flex()
                                    .gap_1p5()
                                    .items_center()
                                    .children(protos.into_iter().map(|proto| {
                                        let active = proxy_proto == proto;
                                        let btn = Button::new(ElementId::Name(
                                            format!("req-proxy-proto-{}", proto.scheme()).into(),
                                        ))
                                        .label(proto.as_str())
                                        .xsmall()
                                        .rounded(ButtonRounded::Small)
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(
                                                this,
                                                RequestEditorAction::SetRequestProxyProtocol(proto),
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
                                                Switch::new("setting-proxy-auth-switch")
                                                    .checked(proxy_auth)
                                                    .small()
                                                    .on_change(move |_, window, app| {
                                                        let _ = ent_auth.update(app, |this, cx| {
                                                            on_action(
                                                                this,
                                                                RequestEditorAction::ToggleRequestProxyAuth,
                                                                window,
                                                                cx,
                                                            );
                                                        });
                                                    }),
                                            )
                                            .child(div().text_xs().text_color(normal).child("Proxy Auth")),
                                    ),
                            )
                            .children(proxy_fields.into_iter().filter(|(_, _, idx, _)| {
                                proxy_auth || (*idx != 2 && *idx != 3)
                            }).map(|(lbl, val, idx, is_masked)| {
                                let is_f = matches!(
                                    active_input,
                                    Some(ActiveInputTarget::SettingProxyField(i)) if *i == idx
                                );
                                v_flex()
                                    .gap_1()
                                    .child(div().text_xs().text_color(muted).child(lbl))
                                    .child(Self::render_inline_input(
                                        ElementId::NamedInteger(
                                            "setting-proxy-field".into(),
                                            idx as u64,
                                        ),
                                        inputs.get_input(&InputKey::SettingProxyField(idx)),
                                        val,
                                        is_f,
                                        is_masked,
                                        field_bg,
                                        border,
                                        normal,
                                    ))
                            }))
                    }),
            )
    }
}
