use super::AuraWorkspaceView;
use crate::input_registry::InputKey;
use crate::theme::subtle_surface;
use aura_core::codegen::CodegenLanguage;
use aura_core::model::ProxyProtocol;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::base::SelectableText;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::dialog::{Dialog, DialogFooter};
use gpui_kit::component::input::Input;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};

impl AuraWorkspaceView {
    pub(super) fn render_network_modals(&self, el: Div, cx: &mut Context<Self>) -> Div {
        let el = self.render_cookie_jar_modal(el, cx);
        let el = self.render_global_proxy_modal(el, cx);
        let el = self.render_capture_proxy_modal(el, cx);
        self.render_codegen_modal(el, cx)
    }

    fn render_cookie_jar_modal(&self, el: Div, cx: &mut Context<Self>) -> Div {
        if !self.state.is_cookie_modal_open {
            return el;
        }
        let border = cx.theme().border;
        let muted = cx.theme().muted_foreground;
        let fg = cx.theme().foreground;
        let radius = cx.theme().radius;
        let surface = subtle_surface(cx);
        let filtered: Vec<_> = self
            .state
            .cookie_jar
            .cookies_for_domain(&self.state.cookie_domain_filter)
            .into_iter()
            .cloned()
            .collect();

        el.child(
            Dialog::new(cx)
                .w(px(700.0))
                .title("Cookie Jar & Cookie Manager")
                .on_close({
                    let view = cx.entity().clone();
                    move |_, _, app| {
                        let _ = view.update(app, |this, cx| {
                            this.state.is_cookie_modal_open = false;
                            cx.notify();
                        });
                    }
                })
                .child(
                    v_flex()
                        .gap_3()
                        .max_h(px(520.0))
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(div().flex_1().when_some(
                                    self.inputs.get_input(&InputKey::CookieFilter),
                                    |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                ))
                                .child(
                                    Button::new("cookie-clear-filtered-btn")
                                        .label(if self.state.cookie_domain_filter.trim().is_empty() {
                                            "Clear All Cookies"
                                        } else {
                                            "Clear Domain"
                                        })
                                        .xsmall()
                                        .danger()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.state.clear_filtered_cookies();
                                            this.state.status_message = Some("Cleared cookies".into());
                                            cx.notify();
                                        })),
                                ),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .p_2()
                                .rounded(radius)
                                .bg(surface)
                                .border_1()
                                .border_color(border)
                                .when(filtered.is_empty(), |box_el| {
                                    box_el.child(
                                        div()
                                            .text_xs()
                                            .text_color(muted)
                                            .p_2()
                                            .child("No stored cookies match filter. Set-Cookie headers from responses appear here automatically."),
                                    )
                                })
                                .children(filtered.into_iter().enumerate().map(|(idx, c)| {
                                    let dom = c.domain.clone();
                                    let path = c.path.clone();
                                    let name = c.name.clone();
                                    let dom_del = dom.clone();
                                    let path_del = path.clone();
                                    let name_del = name.clone();
                                    h_flex()
                                        .justify_between()
                                        .gap_2()
                                        .px_2()
                                        .py_1()
                                        .rounded(radius)
                                        .border_1()
                                        .border_color(border)
                                        .child(
                                            h_flex()
                                                .gap_2()
                                                .items_center()
                                                .flex_1()
                                                .overflow_hidden()
                                                .child(
                                                    Tag::secondary()
                                                        .small()
                                                        .child(format!("{dom}{path}")),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .text_color(fg)
                                                        .child(format!("{name}={}", c.value)),
                                                )
                                                .when(c.secure, |h| h.child(Tag::info().xsmall().child("Secure")))
                                                .when(c.http_only, |h| h.child(Tag::secondary().xsmall().child("HttpOnly"))),
                                        )
                                        .child(
                                            Button::new(ElementId::NamedInteger(
                                                "cookie-del-btn".into(),
                                                idx as u64,
                                            ))
                                            .label("Delete")
                                            .xsmall()
                                            .ghost()
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.state.remove_cookie_entry(
                                                    &dom_del,
                                                    &path_del,
                                                    &name_del,
                                                );
                                                cx.notify();
                                            })),
                                        )
                                })),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .p_2p5()
                                .rounded(radius)
                                .bg(surface)
                                .border_1()
                                .border_color(border)
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(fg)
                                        .child("ADD / UPDATE COOKIE"),
                                )
                                .child(
                                    h_flex()
                                        .gap_1p5()
                                        .child(div().w(px(140.0)).when_some(
                                            self.inputs.get_input(&InputKey::CookieDraftDomain),
                                            |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                        ))
                                        .child(div().w(px(80.0)).when_some(
                                            self.inputs.get_input(&InputKey::CookieDraftPath),
                                            |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                        ))
                                        .child(div().flex_1().when_some(
                                            self.inputs.get_input(&InputKey::CookieDraftName),
                                            |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                        ))
                                        .child(div().flex_1().when_some(
                                            self.inputs.get_input(&InputKey::CookieDraftValue),
                                            |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                        ))
                                        .child(
                                            Button::new("cookie-add-save-btn")
                                                .label("Save")
                                                .xsmall()
                                                .primary()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    if let Err(e) = this.state.save_draft_cookie() {
                                                        this.state.status_message = Some(e);
                                                    } else {
                                                        this.state.status_message =
                                                            Some("Saved cookie to jar".into());
                                                    }
                                                    cx.notify();
                                                })),
                                        ),
                                ),
                        ),
                )
                .footer(
                    DialogFooter::new().child(
                        Button::new("cookie-close-btn")
                            .label("Done")
                            .primary()
                            .small()
                            .rounded(ButtonRounded::Medium)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.is_cookie_modal_open = false;
                                cx.notify();
                            })),
                    ),
                ),
        )
    }

    fn render_global_proxy_modal(&self, el: Div, cx: &mut Context<Self>) -> Div {
        if !self.state.is_proxy_modal_open {
            return el;
        }
        let border = cx.theme().border;
        let muted = cx.theme().muted_foreground;
        let fg = cx.theme().foreground;
        let radius = cx.theme().radius;
        let surface = subtle_surface(cx);
        let protos = [
            ProxyProtocol::Http,
            ProxyProtocol::Https,
            ProxyProtocol::Socks5,
            ProxyProtocol::Socks5h,
        ];
        let view_en = cx.entity().clone();
        let view_auth = cx.entity().clone();

        el.child(
            Dialog::new(cx)
                .w(px(560.0))
                .title("Global HTTP / SOCKS Proxy Settings")
                .on_close({
                    let view = cx.entity().clone();
                    move |_, _, app| {
                        let _ = view.update(app, |this, cx| {
                            this.state.is_proxy_modal_open = false;
                            cx.notify();
                        });
                    }
                })
                .child(
                    v_flex()
                        .gap_3()
                        .child(
                            h_flex()
                                .justify_between()
                                .p_2p5()
                                .rounded(radius)
                                .bg(surface)
                                .border_1()
                                .border_color(border)
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(fg)
                                        .child("Enable Global Outbound Proxy"),
                                )
                                .child(
                                    Switch::new("global-proxy-en-sw")
                                        .checked(self.state.global_proxy.enabled)
                                        .small()
                                        .on_change(move |_, _, app| {
                                            let _ = view_en.update(app, |this, cx| {
                                                this.state.toggle_global_proxy_enabled();
                                                cx.notify();
                                            });
                                        }),
                                ),
                        )
                        .child(
                            h_flex().gap_1p5().children(protos.into_iter().map(|p| {
                                let active = self.state.global_proxy.protocol == p;
                                let btn = Button::new(ElementId::Name(
                                    format!("gproxy-proto-{}", p.scheme()).into(),
                                ))
                                .label(p.as_str())
                                .xsmall()
                                .rounded(ButtonRounded::Small)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.state.set_global_proxy_protocol(p);
                                    cx.notify();
                                }));
                                if active { btn.primary() } else { btn.outline() }
                            })),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .child(div().flex_1().when_some(
                                    self.inputs.get_input(&InputKey::GlobalProxyHost),
                                    |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                ))
                                .child(div().w(px(110.0)).when_some(
                                    self.inputs.get_input(&InputKey::GlobalProxyPort),
                                    |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                )),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .child(div().text_xs().text_color(muted).child("Bypass List (comma-separated, e.g. localhost, 127.0.0.1, *.internal)"))
                                .when_some(
                                    self.inputs.get_input(&InputKey::GlobalProxyBypass),
                                    |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                ),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    Switch::new("global-proxy-auth-sw")
                                        .checked(self.state.global_proxy.auth_enabled)
                                        .small()
                                        .on_change(move |_, _, app| {
                                            let _ = view_auth.update(app, |this, cx| {
                                                this.state.toggle_global_proxy_auth();
                                                cx.notify();
                                            });
                                        }),
                                )
                                .child(div().text_xs().text_color(fg).child("Proxy Authentication")),
                        )
                        .when(self.state.global_proxy.auth_enabled, |d| {
                            d.child(
                                h_flex()
                                    .gap_2()
                                    .child(div().flex_1().when_some(
                                        self.inputs.get_input(&InputKey::GlobalProxyUsername),
                                        |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                    ))
                                    .child(div().flex_1().when_some(
                                        self.inputs.get_input(&InputKey::GlobalProxyPassword),
                                        |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                    )),
                            )
                        }),
                )
                .footer(
                    DialogFooter::new().child(
                        Button::new("gproxy-save-btn")
                            .label("Save & Close")
                            .primary()
                            .small()
                            .rounded(ButtonRounded::Medium)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.persist_network_preferences();
                                this.state.is_proxy_modal_open = false;
                                this.state.status_message = Some("Saved global proxy settings".into());
                                cx.notify();
                            })),
                    ),
                ),
        )
    }

    fn render_capture_proxy_modal(&self, el: Div, cx: &mut Context<Self>) -> Div {
        if !self.state.is_interceptor_modal_open {
            return el;
        }
        let border = cx.theme().border;
        let muted = cx.theme().muted_foreground;
        let fg = cx.theme().foreground;
        let radius = cx.theme().radius;
        let surface = subtle_surface(cx);
        let running = self.state.capture_proxy_handle.is_some();
        let listen_addr = self.state.interceptor_manager.listen_addr();
        let ca_pem = self.state.interceptor_manager.ca_cert_pem();
        let rules = self.state.interceptor_manager.list_rules();
        let captures = self.state.interceptor_manager.list_captures();

        el.child(
            Dialog::new(cx)
                .w(px(740.0))
                .title("Live HTTP/HTTPS MITM Capture Proxy & Interceptor")
                .on_close({
                    let view = cx.entity().clone();
                    move |_, _, app| {
                        let _ = view.update(app, |this, cx| {
                            this.state.is_interceptor_modal_open = false;
                            cx.notify();
                        });
                    }
                })
                .child(
                    v_flex()
                        .gap_3()
                        .max_h(px(560.0))
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(div().w(px(190.0)).when_some(
                                    self.inputs.get_input(&InputKey::InterceptorBindAddr),
                                    |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                ))
                                .child({
                                    let btn = Button::new("capture-start-stop-btn")
                                        .label(if running { "Stop Proxy" } else { "Start Capture Proxy" })
                                        .xsmall();
                                    let btn = if running { btn.danger() } else { btn.primary() };
                                    btn.on_click(cx.listener(|this, _, _, cx| {
                                        this.dispatch_toggle_capture_proxy(cx);
                                    }))
                                })
                                .when(running, |h| {
                                    h.child(Tag::success().small().child(format!("Listening: {listen_addr}")))
                                })
                                .when(!ca_pem.is_empty(), |h| {
                                    let pem_copy = ca_pem.clone();
                                    h.child(
                                        Button::new("capture-copy-ca-btn")
                                            .label("Copy Root CA PEM")
                                            .xsmall()
                                            .outline()
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    pem_copy.clone(),
                                                ));
                                                this.state.status_message = Some(
                                                    "Copied MITM Proxy Root CA certificate PEM to clipboard"
                                                        .into(),
                                                );
                                                cx.notify();
                                            })),
                                    )
                                }),
                        )
                        .child(
                            v_flex()
                                .gap_1p5()
                                .p_2p5()
                                .rounded(radius)
                                .bg(surface)
                                .border_1()
                                .border_color(border)
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(fg)
                                        .child(format!("URL REWRITE RULES ({})", rules.len())),
                                )
                                .child(
                                    h_flex()
                                        .gap_1p5()
                                        .child(div().flex_1().when_some(
                                            self.inputs.get_input(&InputKey::InterceptorRuleMatch),
                                            |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                        ))
                                        .child(div().flex_1().when_some(
                                            self.inputs.get_input(&InputKey::InterceptorRuleTarget),
                                            |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                        ))
                                        .child(
                                            Button::new("interceptor-add-rule-btn")
                                                .label("Add Rule")
                                                .xsmall()
                                                .outline()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    if let Err(e) =
                                                        this.state.add_interceptor_rule_from_draft()
                                                    {
                                                        this.state.status_message = Some(e);
                                                    }
                                                    cx.notify();
                                                })),
                                        ),
                                )
                                .children(rules.into_iter().enumerate().map(|(idx, r)| {
                                    h_flex()
                                        .justify_between()
                                        .text_xs()
                                        .child(div().text_color(muted).child(format!(
                                            "{} -> {}",
                                            r.match_url_pattern,
                                            r.rewrite_url_target.as_deref().unwrap_or("(pass)")
                                        )))
                                        .child(
                                            Button::new(ElementId::NamedInteger(
                                                "rule-del-btn".into(),
                                                idx as u64,
                                            ))
                                            .label("Remove")
                                            .xsmall()
                                            .ghost()
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.state.interceptor_manager.remove_rule(idx);
                                                cx.notify();
                                            })),
                                        )
                                })),
                        )
                        .child(
                            v_flex()
                                .gap_1p5()
                                .p_2p5()
                                .rounded(radius)
                                .bg(surface)
                                .border_1()
                                .border_color(border)
                                .child(
                                    h_flex()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(fg)
                                                .child(format!("CAPTURED TRAFFIC ({})", captures.len())),
                                        )
                                        .child(
                                            Button::new("capture-clear-btn")
                                                .label("Clear")
                                                .xsmall()
                                                .ghost()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.state.interceptor_manager.clear_captures();
                                                    cx.notify();
                                                })),
                                        ),
                                )
                                .when(captures.is_empty(), |box_el| {
                                    box_el.child(
                                        div()
                                            .text_xs()
                                            .text_color(muted)
                                            .child("Point your browser or mobile proxy settings to the listen address above to capture HTTP/HTTPS traffic."),
                                    )
                                })
                                .children(captures.into_iter().enumerate().rev().take(25).map(|(idx, tx)| {
                                    h_flex()
                                        .justify_between()
                                        .gap_2()
                                        .px_2()
                                        .py_1()
                                        .rounded(radius)
                                        .border_1()
                                        .border_color(border)
                                        .child(
                                            h_flex()
                                                .gap_2()
                                                .items_center()
                                                .flex_1()
                                                .overflow_hidden()
                                                .child(Tag::info().small().child(tx.method.as_str()))
                                                .child(Tag::secondary().small().child(format!("{}", tx.status)))
                                                .when(tx.is_https, |h| h.child(Tag::success().xsmall().child("HTTPS")))
                                                .child(div().text_xs().text_color(fg).truncate().child(tx.url)),
                                        )
                                        .child(
                                            Button::new(ElementId::NamedInteger(
                                                "cap-open-btn".into(),
                                                idx as u64,
                                            ))
                                            .label("Open in Tab")
                                            .xsmall()
                                            .outline()
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.state.open_captured_transaction_in_tab(idx);
                                                cx.notify();
                                            })),
                                        )
                                })),
                        ),
                )
                .footer(
                    DialogFooter::new().child(
                        Button::new("capture-close-btn")
                            .label("Close")
                            .small()
                            .rounded(ButtonRounded::Medium)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.is_interceptor_modal_open = false;
                                cx.notify();
                            })),
                    ),
                ),
        )
    }

    fn render_codegen_modal(&self, el: Div, cx: &mut Context<Self>) -> Div {
        if !self.state.is_codegen_modal_open {
            return el;
        }
        let border = cx.theme().border;
        let fg = cx.theme().foreground;
        let radius = cx.theme().radius;
        let surface = subtle_surface(cx);
        let snippet = self.state.active_code_snippet();
        let snippet_copy = snippet.clone();
        let view_vars = cx.entity().clone();

        el.child(
            Dialog::new(cx)
                .w(px(720.0))
                .title("Export Code Snippet")
                .on_close({
                    let view = cx.entity().clone();
                    move |_, _, app| {
                        let _ = view.update(app, |this, cx| {
                            this.state.is_codegen_modal_open = false;
                            cx.notify();
                        });
                    }
                })
                .child(
                    v_flex()
                        .gap_3()
                        .child(
                            h_flex()
                                .justify_between()
                                .items_center()
                                .flex_wrap()
                                .gap_2()
                                .child(
                                    h_flex().gap_1p5().flex_wrap().children(
                                        CodegenLanguage::ALL.into_iter().map(|lang| {
                                            let active = self.state.codegen_language == lang;
                                            let btn = Button::new(ElementId::Name(
                                                format!("cg-lang-{}", lang.label()).into(),
                                            ))
                                            .label(lang.label())
                                            .xsmall()
                                            .rounded(ButtonRounded::Small)
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.state.set_codegen_language(lang);
                                                cx.notify();
                                            }));
                                            if active { btn.primary() } else { btn.outline() }
                                        }),
                                    ),
                                )
                                .child(
                                    h_flex()
                                        .gap_1p5()
                                        .items_center()
                                        .child(
                                            Switch::new("cg-resolve-vars-sw")
                                                .checked(self.state.codegen_resolve_vars)
                                                .small()
                                                .on_change(move |_, _, app| {
                                                    let _ = view_vars.update(app, |this, cx| {
                                                        this.state.codegen_resolve_vars =
                                                            !this.state.codegen_resolve_vars;
                                                        cx.notify();
                                                    });
                                                }),
                                        )
                                        .child(div().text_xs().text_color(fg).child("Resolve {{vars}}")),
                                ),
                        )
                        .child(
                            div()
                                .id("codegen-snippet-box")
                                .p_3()
                                .max_h(px(360.0))
                                .rounded(radius)
                                .bg(surface)
                                .border_1()
                                .border_color(border)
                                .font_family("monospace")
                                .text_xs()
                                .text_color(fg)
                                .overflow_y_scroll()
                                .child(SelectableText::new("codegen-snippet-text", snippet)),
                        ),
                )
                .footer(
                    DialogFooter::new()
                        .child(
                            Button::new("cg-copy-btn")
                                .label("Copy Snippet")
                                .primary()
                                .small()
                                .rounded(ButtonRounded::Medium)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        snippet_copy.clone(),
                                    ));
                                    this.state.status_message =
                                        Some("Copied code snippet to clipboard".into());
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("cg-close-btn")
                                .label("Close")
                                .small()
                                .rounded(ButtonRounded::Medium)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.state.is_codegen_modal_open = false;
                                    cx.notify();
                                })),
                        ),
                ),
        )
    }
}
