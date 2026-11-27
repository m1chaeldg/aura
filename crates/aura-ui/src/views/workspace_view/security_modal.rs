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

        el.child(
            Dialog::new(cx)
                .w(px(680.0))
                .title("Security: OS Keychain, Secret Vaults & Domain mTLS")
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
                        .gap_4()
                        .max_h(px(560.0))
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
                                                    match this.state.store_keychain_entry() {
                                                        Ok(uri) => {
                                                            this.state.status_message = Some(format!(
                                                                "Saved secret to OS Keychain (reference: {uri})"
                                                            ));
                                                        }
                                                        Err(e) => {
                                                            this.state.status_message = Some(e);
                                                        }
                                                    }
                                                    cx.notify();
                                                })),
                                        )
                                        .child(
                                            Button::new("kc-load-btn")
                                                .label("Load Secret")
                                                .xsmall()
                                                .outline()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    match this.state.load_keychain_entry() {
                                                        Ok(()) => {
                                                            this.state.status_message = Some(
                                                                "Loaded secret from OS Keychain"
                                                                    .to_string(),
                                                            );
                                                        }
                                                        Err(e) => {
                                                            this.state.status_message = Some(e);
                                                        }
                                                    }
                                                    cx.notify();
                                                })),
                                        )
                                        .child(
                                            Button::new("kc-del-btn")
                                                .label("Delete")
                                                .xsmall()
                                                .danger()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    match this.state.delete_keychain_entry() {
                                                        Ok(()) => {
                                                            this.state.status_message = Some(
                                                                "Deleted secret from OS Keychain"
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
                        )
                        // 2. HashiCorp Vault & AWS Secrets Manager Config
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
                                        .child("HASHICORP VAULT (vault://path#field) & AWS SECRETS MANAGER (aws-sm://id#key)"),
                                )
                                .child(
                                    h_flex()
                                        .gap_2()
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
                                            div().w(px(140.0)).when_some(
                                                self.inputs.get_input(&InputKey::VaultNamespace),
                                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                            ),
                                        ),
                                )
                                .child(
                                    h_flex()
                                        .gap_2()
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
