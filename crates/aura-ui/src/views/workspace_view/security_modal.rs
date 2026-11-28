use super::AuraWorkspaceView;
use crate::input_registry::InputKey;
use crate::theme::subtle_surface;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::dialog::{Dialog, DialogFooter};
use gpui_kit::component::input::Input;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};

impl AuraWorkspaceView {
    pub(super) fn render_security_vault_modal(&self, el: Div, cx: &mut Context<Self>) -> Div {
        if !self.state.is_security_vault_modal_open {
            return el;
        }

        let border = cx.theme().border;
        let muted = cx.theme().muted_foreground;
        let fg = cx.theme().foreground;
        let radius = cx.theme().radius;
        let surface = subtle_surface(cx);
        let certs = self.state.domain_client_certs.clone();
        let hc_profiles = self.state.vault_config.hashicorp_profiles.clone();
        let aws_profiles = self.state.vault_config.aws_profiles.clone();
        let kp_profiles = self.state.vault_config.keepass_profiles.clone();
        let cache_count = aura_engine::secret_cache_len();

        el.child(
            Dialog::new(cx)
                .w(px(740.0))
                .title("Security: OS Keychain, Secret Vaults, KeePass & Domain mTLS")
                .on_close({
                    let view = cx.entity().clone();
                    move |_, _, app| {
                        let _ = view.update(app, |this, cx| {
                            this.state.is_security_vault_modal_open = false;
                            cx.notify();
                        });
                    }
                })
                .child(
                    v_flex()
                        .id("security-vault-modal-scroll")
                        .gap_3()
                        .max_h(px(560.0))
                        .overflow_y_scroll()
                        // 0. Secret Cache Bar
                        .child(
                            h_flex()
                                .justify_between()
                                .px_3()
                                .py_2()
                                .rounded(radius)
                                .bg(surface)
                                .border_1()
                                .border_color(border)
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(muted)
                                        .child(format!(
                                            "In-memory secret cache: {cache_count} cached entry(s) (TTL: {}s)",
                                            self.state.vault_config.cache_ttl_secs
                                        )),
                                )
                                .child(
                                    Button::new("vault-clear-cache-btn")
                                        .label("Clear Secret Cache")
                                        .xsmall()
                                        .outline()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            aura_engine::clear_secret_cache();
                                            this.state.status_message = Some("Cleared in-memory secret cache".to_string());
                                            cx.notify();
                                        })),
                                ),
                        )
                        // 1. OS Keychain Manager
                        .child(
                            v_flex()
                                .gap_2()
                                .p_3()
                                .rounded(radius)
                                .bg(surface)
                                .border_1()
                                .border_color(border)
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(fg)
                                        .child("OS KEYCHAIN / CREDENTIAL MANAGER (keyring://service/key)"),
                                )
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .child(
                                            div().w(px(140.0)).when_some(
                                                self.inputs.get_input(&InputKey::KeychainService),
                                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                            ),
                                        )
                                        .child(
                                            div().flex_1().when_some(
                                                self.inputs.get_input(&InputKey::KeychainKey),
                                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                            ),
                                        )
                                        .child(
                                            div().flex_1().when_some(
                                                self.inputs.get_input(&InputKey::KeychainValue),
                                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                            ),
                                        ),
                                )
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .child(
                                            Button::new("kc-store-btn")
                                                .label("Save to Keychain")
                                                .xsmall()
                                                .primary()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    let svc = this.state.keychain_service.trim().to_string();
                                                    let key = this.state.keychain_key.trim().to_string();
                                                    let val = this.state.keychain_value.clone();
                                                    if key.is_empty() {
                                                        this.state.status_message = Some("Keychain key name is required".to_string());
                                                        cx.notify();
                                                        return;
                                                    }
                                                    cx.spawn(async move |view, cx| {
                                                        let res = cx
                                                            .background_executor()
                                                            .spawn(async move {
                                                                aura_engine::keychain_set(&svc, &key, &val)
                                                                    .map(|_| {
                                                                        let eff = if svc.is_empty() { "aura" } else { svc.as_str() };
                                                                        format!("Saved secret to OS Keychain (reference: keyring://{eff}/{key})")
                                                                    })
                                                                    .map_err(|e| format!("{e}"))
                                                            })
                                                            .await;
                                                        let _ = view.update(cx, |this, cx| {
                                                            this.state.status_message = Some(match res {
                                                                Ok(msg) => msg,
                                                                Err(e) => e,
                                                            });
                                                            cx.notify();
                                                        });
                                                    })
                                                    .detach();
                                                })),
                                        )
                                        .child(
                                            Button::new("kc-load-btn")
                                                .label("Load Secret")
                                                .xsmall()
                                                .outline()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    let svc = this.state.keychain_service.trim().to_string();
                                                    let key = this.state.keychain_key.trim().to_string();
                                                    if key.is_empty() {
                                                        this.state.status_message = Some("Keychain key name is required".to_string());
                                                        cx.notify();
                                                        return;
                                                    }
                                                    let key_for_bg = key.clone();
                                                    cx.spawn(async move |view, cx| {
                                                        let res = cx
                                                            .background_executor()
                                                            .spawn(async move {
                                                                aura_engine::keychain_get(&svc, &key_for_bg)
                                                                    .map_err(|e| format!("{e}"))
                                                            })
                                                            .await;
                                                        let _ = view.update(cx, |this, cx| {
                                                            match res {
                                                                Ok(Some(v)) => {
                                                                    this.state.keychain_value = v;
                                                                    this.state.status_message = Some("Loaded secret from OS Keychain".to_string());
                                                                }
                                                                Ok(None) => {
                                                                    this.state.status_message = Some(format!("No OS keychain secret found for '{key}'"));
                                                                }
                                                                Err(e) => {
                                                                    this.state.status_message = Some(e);
                                                                }
                                                            }
                                                            cx.notify();
                                                        });
                                                    })
                                                    .detach();
                                                })),
                                        )
                                        .child(
                                            Button::new("kc-del-btn")
                                                .label("Delete")
                                                .xsmall()
                                                .danger()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    let svc = this.state.keychain_service.trim().to_string();
                                                    let key = this.state.keychain_key.trim().to_string();
                                                    if key.is_empty() {
                                                        this.state.status_message = Some("Keychain key name is required".to_string());
                                                        cx.notify();
                                                        return;
                                                    }
                                                    cx.spawn(async move |view, cx| {
                                                        let res = cx
                                                            .background_executor()
                                                            .spawn(async move {
                                                                aura_engine::keychain_delete(&svc, &key)
                                                                    .map_err(|e| format!("{e}"))
                                                            })
                                                            .await;
                                                        let _ = view.update(cx, |this, cx| {
                                                            match res {
                                                                Ok(()) => {
                                                                    this.state.keychain_value.clear();
                                                                    this.state.status_message = Some("Deleted secret from OS Keychain".to_string());
                                                                }
                                                                Err(e) => {
                                                                    this.state.status_message = Some(e);
                                                                }
                                                            }
                                                            cx.notify();
                                                        });
                                                    })
                                                    .detach();
                                                })),
                                        ),
                                ),
                        )
                        // 2. HashiCorp Vault & AWS Secrets Manager Profiles
                        .child(
                            v_flex()
                                .gap_2()
                                .p_3()
                                .rounded(radius)
                                .bg(surface)
                                .border_1()
                                .border_color(border)
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(fg)
                                        .child("HASHICORP VAULT (vault://[profile@]path#field)"),
                                )
                                .children(hc_profiles.into_iter().enumerate().map(|(idx, p)| {
                                    let p_addr = p.addr.clone();
                                    let p_ns = p.namespace.clone();
                                    let p_id = p.id.clone();
                                    let p_verify = p.verify_ssl;
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
                                                .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(fg).child(p.name))
                                                .child(div().text_xs().text_color(muted).child(p.addr)),
                                        )
                                        .child(
                                            h_flex()
                                                .gap_1()
                                                .child(
                                                    Button::new(ElementId::NamedInteger("hc-test".into(), idx as u64))
                                                        .label("Test")
                                                        .xsmall()
                                                        .outline()
                                                        .on_click(cx.listener(move |this, _, _, cx| {
                                                            let addr = p_addr.clone();
                                                            let ns = p_ns.clone();
                                                            let id = p_id.clone();
                                                            let fallback_tok = this.state.vault_config.vault_token.clone();
                                                            cx.spawn(async move |view, cx| {
                                                                let res = cx.background_executor().spawn(async move {
                                                                    let tok = aura_engine::get_vault_profile_token(&id)
                                                                        .ok()
                                                                        .flatten()
                                                                        .unwrap_or(fallback_tok);
                                                                    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("{e}"))?;
                                                                    rt.block_on(aura_engine::test_hashicorp_vault_connection(
                                                                        &addr,
                                                                        &tok,
                                                                        ns.as_deref(),
                                                                        p_verify,
                                                                    ))
                                                                    .map_err(|e| format!("{e}"))
                                                                }).await;
                                                                let _ = view.update(cx, |this, cx| {
                                                                    this.state.status_message = Some(match res {
                                                                        Ok(msg) | Err(msg) => msg,
                                                                    });
                                                                    cx.notify();
                                                                });
                                                            }).detach();
                                                        })),
                                                )
                                                .child(
                                                    Button::new(ElementId::NamedInteger("hc-del".into(), idx as u64))
                                                        .label("Remove")
                                                        .xsmall()
                                                        .ghost()
                                                        .on_click(cx.listener(move |this, _, _, cx| {
                                                            this.state.remove_hashicorp_vault_profile(idx);
                                                            cx.notify();
                                                        })),
                                                ),
                                        )
                                }))
                                .child(
                                    h_flex()
                                        .gap_1p5()
                                        .child(
                                            div().w(px(120.0)).when_some(
                                                self.inputs.get_input(&InputKey::VaultProfileName),
                                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                            ),
                                        )
                                        .child(
                                            div().flex_1().when_some(
                                                self.inputs.get_input(&InputKey::VaultAddr),
                                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                            ),
                                        )
                                        .child(
                                            div().flex_1().when_some(
                                                self.inputs.get_input(&InputKey::VaultToken),
                                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                            ),
                                        )
                                        .child(
                                            div().w(px(110.0)).when_some(
                                                self.inputs.get_input(&InputKey::VaultNamespace),
                                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                            ),
                                        )
                                        .child(
                                            Button::new("hc-add-btn")
                                                .label("Add")
                                                .xsmall()
                                                .primary()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    match this.state.add_hashicorp_vault_profile() {
                                                        Ok(name) => {
                                                            this.state.status_message = Some(format!("Added HashiCorp Vault profile '{name}' (token stored in Keychain)"));
                                                        }
                                                        Err(e) => this.state.status_message = Some(e),
                                                    }
                                                    cx.notify();
                                                })),
                                        ),
                                )
                                .child(
                                    div()
                                        .pt_2()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(fg)
                                        .child("AWS SECRETS MANAGER (aws-sm://[profile@]secret_id#key)"),
                                )
                                .children(aws_profiles.into_iter().enumerate().map(|(idx, p)| {
                                    let reg = p.region.clone();
                                    let prof = p.profile.clone();
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
                                                .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(fg).child(p.name))
                                                .child(div().text_xs().text_color(muted).child(format!("{} ({})", p.region, p.profile.as_deref().unwrap_or("default/env")))),
                                        )
                                        .child(
                                            h_flex()
                                                .gap_1()
                                                .child(
                                                    Button::new(ElementId::NamedInteger("aws-test".into(), idx as u64))
                                                        .label("Test")
                                                        .xsmall()
                                                        .outline()
                                                        .on_click(cx.listener(move |_this, _, _, cx| {
                                                            let r = reg.clone();
                                                            let pr = prof.clone();
                                                            cx.spawn(async move |view, cx| {
                                                                let res = cx.background_executor().spawn(async move {
                                                                    aura_engine::test_aws_credentials(&r, pr.as_deref())
                                                                        .map_err(|e| format!("{e}"))
                                                                }).await;
                                                                let _ = view.update(cx, |this, cx| {
                                                                    this.state.status_message = Some(match res {
                                                                        Ok(msg) | Err(msg) => msg,
                                                                    });
                                                                    cx.notify();
                                                                });
                                                            }).detach();
                                                        })),
                                                )
                                                .child(
                                                    Button::new(ElementId::NamedInteger("aws-del".into(), idx as u64))
                                                        .label("Remove")
                                                        .xsmall()
                                                        .ghost()
                                                        .on_click(cx.listener(move |this, _, _, cx| {
                                                            this.state.remove_aws_vault_profile(idx);
                                                            cx.notify();
                                                        })),
                                                ),
                                        )
                                }))
                                .child(
                                    h_flex()
                                        .gap_1p5()
                                        .child(
                                            div().w(px(130.0)).when_some(
                                                self.inputs.get_input(&InputKey::AwsProfileName),
                                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                            ),
                                        )
                                        .child(
                                            div().flex_1().when_some(
                                                self.inputs.get_input(&InputKey::VaultAwsRegion),
                                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                            ),
                                        )
                                        .child(
                                            div().flex_1().when_some(
                                                self.inputs.get_input(&InputKey::VaultAwsProfile),
                                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                            ),
                                        )
                                        .child(
                                            Button::new("aws-add-btn")
                                                .label("Add")
                                                .xsmall()
                                                .primary()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    match this.state.add_aws_vault_profile() {
                                                        Ok(name) => {
                                                            this.state.status_message = Some(format!("Added AWS Secrets Manager profile '{name}'"));
                                                        }
                                                        Err(e) => this.state.status_message = Some(e),
                                                    }
                                                    cx.notify();
                                                })),
                                        ),
                                ),
                        )
                        // 2b. KeePass (.kdbx) Profiles
                        .child(
                            v_flex()
                                .gap_2()
                                .p_3()
                                .rounded(radius)
                                .bg(surface)
                                .border_1()
                                .border_color(border)
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(fg)
                                        .child("KEEPASS .KDBX DATABASES (keepass://[profile@]Group/Entry#Password)"),
                                )
                                .children(kp_profiles.into_iter().enumerate().map(|(idx, p)| {
                                    let p_id = p.id.clone();
                                    let p_db = p.db_path.clone();
                                    let p_kf = p.keyfile_path.clone();
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
                                                .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(fg).child(p.name))
                                                .child(div().text_xs().text_color(muted).child(p.db_path)),
                                        )
                                        .child(
                                            h_flex()
                                                .gap_1()
                                                .child(
                                                    Button::new(ElementId::NamedInteger("kp-test".into(), idx as u64))
                                                        .label("Test")
                                                        .xsmall()
                                                        .outline()
                                                        .on_click(cx.listener(move |_this, _, _, cx| {
                                                            let id = p_id.clone();
                                                            let db = p_db.clone();
                                                            let kf = p_kf.clone();
                                                            cx.spawn(async move |view, cx| {
                                                                let res = cx.background_executor().spawn(async move {
                                                                    let pw = aura_engine::get_keepass_profile_password(&id)
                                                                        .ok()
                                                                        .flatten();
                                                                    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("{e}"))?;
                                                                    rt.block_on(aura_engine::test_keepass_connection(
                                                                        &db,
                                                                        pw.as_deref(),
                                                                        kf.as_deref(),
                                                                    ))
                                                                    .map_err(|e| format!("{e}"))
                                                                }).await;
                                                                let _ = view.update(cx, |this, cx| {
                                                                    this.state.status_message = Some(match res {
                                                                        Ok(msg) | Err(msg) => msg,
                                                                    });
                                                                    cx.notify();
                                                                });
                                                            }).detach();
                                                        })),
                                                )
                                                .child(
                                                    Button::new(ElementId::NamedInteger("kp-del".into(), idx as u64))
                                                        .label("Remove")
                                                        .xsmall()
                                                        .ghost()
                                                        .on_click(cx.listener(move |this, _, _, cx| {
                                                            this.state.remove_keepass_profile(idx);
                                                            cx.notify();
                                                        })),
                                                ),
                                        )
                                }))
                                .child(
                                    h_flex()
                                        .gap_1p5()
                                        .child(
                                            div().w(px(110.0)).when_some(
                                                self.inputs.get_input(&InputKey::KeePassProfileName),
                                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                            ),
                                        )
                                        .child(
                                            div().flex_1().when_some(
                                                self.inputs.get_input(&InputKey::KeePassDbPath),
                                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                            ),
                                        )
                                        .child(
                                            div().w(px(130.0)).when_some(
                                                self.inputs.get_input(&InputKey::KeePassKeyfilePath),
                                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                            ),
                                        )
                                        .child(
                                            div().w(px(130.0)).when_some(
                                                self.inputs.get_input(&InputKey::KeePassPassword),
                                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                            ),
                                        )
                                        .child(
                                            Button::new("kp-add-btn")
                                                .label("Add")
                                                .xsmall()
                                                .primary()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    match this.state.add_keepass_profile() {
                                                        Ok(name) => {
                                                            this.state.status_message = Some(format!("Added KeePass profile '{name}' (password stored in Keychain)"));
                                                        }
                                                        Err(e) => this.state.status_message = Some(e),
                                                    }
                                                    cx.notify();
                                                })),
                                        ),
                                ),
                        )
                        // 3. Per-Domain mTLS Client Certificates
                        .child(
                            v_flex()
                                .gap_2()
                                .p_3()
                                .rounded(radius)
                                .bg(surface)
                                .border_1()
                                .border_color(border)
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(fg)
                                        .child("DOMAIN CLIENT CERTIFICATES (mTLS: .pem / .pfx / .p12)"),
                                )
                                .children(certs.into_iter().enumerate().map(|(idx, c)| {
                                    let dom = c.domain.unwrap_or_else(|| "*".to_string());
                                    let path = c.cert_path;
                                    let enabled = c.enabled;
                                    let view_tog = cx.entity().clone();
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
                                                .child(
                                                    Switch::new(ElementId::NamedInteger(
                                                        "dom-cert-sw".into(),
                                                        idx as u64,
                                                    ))
                                                    .checked(enabled)
                                                    .small()
                                                    .on_change(move |_, _, app| {
                                                        let _ = view_tog.update(app, |this, cx| {
                                                            this.state.toggle_domain_cert(idx);
                                                            cx.notify();
                                                        });
                                                    }),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .text_color(fg)
                                                        .child(dom),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(muted)
                                                        .child(path),
                                                ),
                                        )
                                        .child(
                                            Button::new(ElementId::NamedInteger(
                                                "dom-cert-del".into(),
                                                idx as u64,
                                            ))
                                            .label("Remove")
                                            .xsmall()
                                            .ghost()
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.state.remove_domain_cert(idx);
                                                cx.notify();
                                            })),
                                        )
                                }))
                                .child(
                                    h_flex()
                                        .gap_1p5()
                                        .child(
                                            div().w(px(140.0)).when_some(
                                                self.inputs.get_input(&InputKey::DomainCertDomain),
                                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                            ),
                                        )
                                        .child(
                                            div().flex_1().when_some(
                                                self.inputs.get_input(&InputKey::DomainCertPath),
                                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                            ),
                                        )
                                        .child(
                                            div().flex_1().when_some(
                                                self.inputs.get_input(&InputKey::DomainCertKeyPath),
                                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                            ),
                                        )
                                        .child(
                                            div().w(px(110.0)).when_some(
                                                self.inputs.get_input(&InputKey::DomainCertPassphrase),
                                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                            ),
                                        )
                                        .child(
                                            Button::new("dom-cert-add-btn")
                                                .label("Add")
                                                .xsmall()
                                                .primary()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    match this.state.add_domain_cert() {
                                                        Ok(()) => {
                                                            this.state.status_message = Some(
                                                                "Added domain client certificate"
                                                                    .to_string(),
                                                            );
                                                        }
                                                        Err(e) => {
                                                            this.state.status_message = Some(e);
                                                        }
                                                    }
                                                    cx.notify();
                                                })),
                                        ),
                                ),
                        ),
                )
                .footer(
                    DialogFooter::new()
                        .child(
                            Button::new("vault-save-close-btn")
                                .label("Save & Close")
                                .primary()
                                .small()
                                .rounded(ButtonRounded::Medium)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.state.persist_security_preferences();
                                    this.state.is_security_vault_modal_open = false;
                                    this.state.status_message =
                                        Some("Saved vault and certificate settings".to_string());
                                    cx.notify();
                                })),
                        ),
                ),
        )
    }
}
