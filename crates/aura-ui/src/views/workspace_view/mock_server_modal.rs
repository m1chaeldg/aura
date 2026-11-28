use super::AuraWorkspaceView;
use crate::input_registry::InputKey;
use crate::theme::subtle_surface;
use aura_core::mock::MockUnmatchedPolicy;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::dialog::{Dialog, DialogFooter};
use gpui_kit::component::input::Input;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};

impl AuraWorkspaceView {
    pub(super) fn render_mock_server_modal(&self, el: Div, cx: &mut Context<Self>) -> Div {
        if !self.state.is_mock_modal_open {
            return el;
        }

        let border = cx.theme().border;
        let muted = cx.theme().muted_foreground;
        let fg = cx.theme().foreground;
        let running = self.state.mock_server_handle.is_some();
        let listen_addr = self
            .state
            .mock_server_handle
            .as_ref()
            .map(|h| format!("http://{}", h.listen_addr))
            .unwrap_or_else(|| format!("http://{}", self.state.mock_host_port));
        let hit_logs = self
            .state
            .mock_server_handle
            .as_ref()
            .map(|h| h.hit_logs())
            .unwrap_or_default();
        let routes = &self.state.mock_config.routes;
        let is_passthrough = matches!(
            self.state.mock_config.unmatched,
            MockUnmatchedPolicy::ProxyPassthrough { .. }
        );
        let view_close = cx.entity().clone();

        el.child(
            Dialog::new(cx)
                .w(px(760.0))
                .title("Local & Production Mock Server")
                .on_close(move |_, _window, app| {
                    let _ = view_close.update(app, |this, cx| {
                        this.state.is_mock_modal_open = false;
                        cx.notify();
                    });
                })
                .child(
                    v_flex()
                        .id("mock-modal-body")
                        .gap_3()
                        .max_h(px(540.0))
                        .overflow_y_scroll()
                        // Server Controls Row
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(div().w(px(180.0)).when_some(
                                    self.inputs.get_input(&InputKey::MockHostPort),
                                    |d, inp| d.child(Input::new(inp).small().w_full()),
                                ))
                                .child(
                                    Button::new("mock-toggle-btn")
                                        .label(if running { "Stop Mock Server" } else { "Start Mock Server" })
                                        .small()
                                        .primary()
                                        .rounded(ButtonRounded::Medium)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.dispatch_toggle_mock_server(cx);
                                        })),
                                )
                                .child(
                                    Button::new("mock-gen-tabs-btn")
                                        .label("Generate from Open Tabs")
                                        .small()
                                        .outline()
                                        .rounded(ButtonRounded::Medium)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.state.generate_mock_routes_from_tabs();
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Tag::new()
                                        .xsmall()
                                        .rounded(px(4.0))
                                        .child(if running { "LIVE" } else { "STOPPED" }),
                                )
                                .child(div().text_xs().text_color(muted).child(listen_addr)),
                        )
                        // User Dataset & Unmatched Policy Row
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(div().flex_1().when_some(
                                    self.inputs.get_input(&InputKey::MockDraftDatasetPath),
                                    |d, inp| d.child(Input::new(inp).small().w_full()),
                                ))
                                .child(
                                    Button::new("mock-unmatched-btn")
                                        .label(if is_passthrough {
                                            "Unmatched: Passthrough"
                                        } else {
                                            "Unmatched: 404 JSON"
                                        })
                                        .small()
                                        .outline()
                                        .rounded(ButtonRounded::Medium)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.state.mock_config.unmatched =
                                                if matches!(
                                                    this.state.mock_config.unmatched,
                                                    MockUnmatchedPolicy::NotFound
                                                ) {
                                                    MockUnmatchedPolicy::ProxyPassthrough {
                                                        target_base_url: "https://postman-echo.com".to_string(),
                                                    }
                                                } else {
                                                    MockUnmatchedPolicy::NotFound
                                                };
                                            cx.notify();
                                        })),
                                ),
                        )
                        // Add Route / Rule Builder
                        .child(
                            v_flex()
                                .gap_1p5()
                                .p_2()
                                .rounded(px(6.0))
                                .border_1()
                                .border_color(border)
                                .bg(subtle_surface(cx))
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(fg)
                                        .child("Add Route & Request Matching Rule"),
                                )
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .child(div().w(px(76.0)).when_some(
                                            self.inputs.get_input(&InputKey::MockDraftMethod),
                                            |d, inp| d.child(Input::new(inp).small().w_full()),
                                        ))
                                        .child(div().flex_1().when_some(
                                            self.inputs.get_input(&InputKey::MockDraftPath),
                                            |d, inp| d.child(Input::new(inp).small().w_full()),
                                        ))
                                        .child(div().w(px(64.0)).when_some(
                                            self.inputs.get_input(&InputKey::MockDraftStatus),
                                            |d, inp| d.child(Input::new(inp).small().w_full()),
                                        ))
                                        .child(
                                            Button::new("mock-mode-cycle-btn")
                                                .label(self.state.mock_draft_mode.label())
                                                .small()
                                                .outline()
                                                .rounded(ButtonRounded::Medium)
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.state.cycle_mock_draft_mode();
                                                    cx.notify();
                                                })),
                                        ),
                                )
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .child(div().flex_1().when_some(
                                            self.inputs.get_input(&InputKey::MockDraftHeaderRule),
                                            |d, inp| d.child(Input::new(inp).small().w_full()),
                                        ))
                                        .child(div().flex_1().when_some(
                                            self.inputs.get_input(&InputKey::MockDraftQueryRule),
                                            |d, inp| d.child(Input::new(inp).small().w_full()),
                                        ))
                                        .child(div().flex_1().when_some(
                                            self.inputs.get_input(&InputKey::MockDraftBodyRule),
                                            |d, inp| d.child(Input::new(inp).small().w_full()),
                                        )),
                                )
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .child(div().flex_1().when_some(
                                            self.inputs.get_input(&InputKey::MockDraftBody),
                                            |d, inp| d.child(Input::new(inp).small().w_full()),
                                        ))
                                        .child(
                                            Button::new("mock-add-route-btn")
                                                .label("+ Add Rule")
                                                .small()
                                                .primary()
                                                .rounded(ButtonRounded::Medium)
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.state.add_mock_route_from_draft();
                                                    cx.notify();
                                                })),
                                        ),
                                ),
                        )
                        // Configured Routes List
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(fg)
                                        .child(format!("Configured Routes ({})", routes.len())),
                                )
                                .children(routes.iter().enumerate().map(|(idx, r)| {
                                    let method = r.rule.methods.join(",");
                                    let path = r.rule.path.display_value().to_string();
                                    let mode = r.response.mode.label();
                                    let status = r.response.status;
                                    let enabled = r.enabled;
                                    h_flex()
                                        .gap_2()
                                        .items_center()
                                        .p_1p5()
                                        .rounded(px(4.0))
                                        .border_1()
                                        .border_color(border)
                                        .child(Tag::new().xsmall().child(method))
                                        .child(div().flex_1().text_xs().text_color(fg).child(path))
                                        .child(div().text_xs().text_color(muted).child(format!("{status} · {mode}")))
                                        .child(
                                            Button::new(("mock-toggle-route", idx))
                                                .label(if enabled { "On" } else { "Off" })
                                                .xsmall()
                                                .outline()
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.state.toggle_mock_route(idx);
                                                    cx.notify();
                                                })),
                                        )
                                        .child(
                                            Button::new(("mock-del-route", idx))
                                                .label("Del")
                                                .xsmall()
                                                .danger()
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.state.remove_mock_route(idx);
                                                    cx.notify();
                                                })),
                                        )
                                })),
                        )
                        // Live Hit Logs
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    h_flex()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(fg)
                                                .child(format!("Recent Mock Requests ({})", hit_logs.len())),
                                        )
                                        .child(
                                            Button::new("mock-clear-logs-btn")
                                                .label("Clear")
                                                .xsmall()
                                                .ghost()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    if let Some(h) = &this.state.mock_server_handle {
                                                        h.clear_hit_logs();
                                                    }
                                                    cx.notify();
                                                })),
                                        ),
                                )
                                .children(hit_logs.iter().rev().take(8).map(|log| {
                                    h_flex()
                                        .gap_2()
                                        .text_xs()
                                        .text_color(muted)
                                        .child(Tag::new().xsmall().child(log.method.clone()))
                                        .child(div().flex_1().text_color(fg).child(log.path.clone()))
                                        .child(format!("{} ({} ms)", log.status, log.duration_ms))
                                })),
                        ),
                )
                .footer(
                    DialogFooter::new().child(
                        Button::new("mock-close-btn")
                            .label("Close")
                            .small()
                            .rounded(ButtonRounded::Medium)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.is_mock_modal_open = false;
                                cx.notify();
                            })),
                    ),
                ),
        )
    }
}
