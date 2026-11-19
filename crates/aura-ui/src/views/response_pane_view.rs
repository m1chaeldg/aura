//! ResponsePane View: Status pills, metrics, tabs, tests, console logs, and formatted response body.

use crate::stores::{ResponseBodyMode, ResponseTab};
use aura_core::model::{AuraResponse, TestAssertion};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::Selectable;

pub struct ResponsePaneView;

impl ResponsePaneView {
    pub fn format_body(body: &str, mode: ResponseBodyMode) -> String {
        match mode {
            ResponseBodyMode::Pretty => {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(body) {
                    serde_json::to_string_pretty(&val).unwrap_or_else(|_| body.to_string())
                } else {
                    body.to_string()
                }
            }
            ResponseBodyMode::Raw => body.to_string(),
        }
    }

    pub fn assertion_counts(assertions: &[TestAssertion]) -> (usize, usize) {
        let total = assertions.len();
        let passed = assertions.iter().filter(|a| a.passed).count();
        (passed, total)
    }

    pub fn status_color(status: u16) -> Rgba {
        match status {
            200..=299 => rgb(0x28a745),
            300..=399 => rgb(0x0066cc),
            400..=499 => rgb(0xf5a623),
            500..=599 => rgb(0xe74c3c),
            _ => rgb(0x7a7a7a),
        }
    }

    pub fn format_size(bytes: usize) -> String {
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.2} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render<V: 'static>(
        split_ratio: f32,
        is_horizontal: bool,
        response: Option<&AuraResponse>,
        active_tab: ResponseTab,
        body_mode: ResponseBodyMode,
        cx: &mut Context<V>,
        on_select_tab: impl Fn(&mut V, ResponseTab, &mut Window, &mut Context<V>) + 'static + Copy,
        on_toggle_body_mode: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let border = rgb(0x333333);
        let surface = rgb(0x272729);
        let text_muted = rgb(0x7a7a7a);

        div()
            .flex()
            .flex_col()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .when(is_horizontal, |el| el.w(gpui::relative(1.0 - split_ratio)).flex_shrink_0())
            .when(!is_horizontal, |el| el.h(gpui::relative(1.0 - split_ratio)).flex_shrink_0())
            .bg(rgb(0x1b1b1d))
            .child(match response {
                None => div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(border)
                            .py_1()
                            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(rgb(0x28a745)).child("RESPONSE PANE"))
                            .child(div().text_xs().text_color(text_muted).child("Status: -- • Time: --ms • Size: --B")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .py_12()
                            .gap_2()
                            .child(div().text_sm().text_color(text_muted).child("Ready to dispatch request"))
                            .child(div().text_xs().text_color(rgb(0x555555)).child("Send a request with 'Send ↵' or Ctrl+Enter to inspect headers, raw body, and formatting.")),
                    ),
                Some(resp) => Self::render_active_response(resp, active_tab, body_mode, border, surface, text_muted, cx, on_select_tab, on_toggle_body_mode),
            })
    }

    #[allow(clippy::too_many_arguments)]
    fn render_active_response<V: 'static>(
        resp: &AuraResponse,
        active_tab: ResponseTab,
        body_mode: ResponseBodyMode,
        border: Rgba,
        surface: Rgba,
        muted: Rgba,
        cx: &mut Context<V>,
        on_select_tab: impl Fn(&mut V, ResponseTab, &mut Window, &mut Context<V>) + 'static + Copy,
        on_toggle_body_mode: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let (passed_tests, total_tests) = Self::assertion_counts(&resp.assertions);
        let tabs = [
            (ResponseTab::Body, "Body".to_string()),
            (
                ResponseTab::Headers,
                format!("Headers ({})", resp.headers.len()),
            ),
            (ResponseTab::Raw, "Raw".to_string()),
            (ResponseTab::Json, "JSON".to_string()),
            (
                ResponseTab::Tests,
                format!("Tests ({}/{})", passed_tests, total_tests),
            ),
            (
                ResponseTab::Console,
                format!("Console ({})", resp.console_logs.len()),
            ),
        ];

        div()
            .flex()
            .flex_col()
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            // Metrics Header Bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .bg(surface)
                    .border_b_1()
                    .border_color(border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            // Status Pill
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(Self::status_color(resp.status))
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xffffff))
                                    .child(format!("{} {}", resp.status, resp.status_text)),
                            )
                            // Duration Metric
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(rgb(0x1d1d1f))
                                    .border_1()
                                    .border_color(border)
                                    .text_xs()
                                    .text_color(muted)
                                    .child(format!("⏱ {} ms", resp.duration_ms)),
                            )
                            // Size Metric
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(rgb(0x1d1d1f))
                                    .border_1()
                                    .border_color(border)
                                    .text_xs()
                                    .text_color(muted)
                                    .child(format!("📦 {}", Self::format_size(resp.size_bytes))),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            // Body Format Mode Toggle
                            .child(
                                div()
                                    .id("body-mode-toggle-btn")
                                    .flex()
                                    .items_center()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(rgb(0x1d1d1f))
                                    .border_1()
                                    .border_color(border)
                                    .hover(|s| s.bg(rgb(0x333333)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(rgb(0x2997ff))
                                    .child(match body_mode {
                                        ResponseBodyMode::Pretty => "Format: Pretty ▾",
                                        ResponseBodyMode::Raw => "Format: Raw ▾",
                                    })
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_toggle_body_mode(this, window, cx);
                                    })),
                            ),
                    ),
            )
            // Tabs Bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .px_3()
                    .py_1()
                    .bg(rgb(0x222224))
                    .border_b_1()
                    .border_color(border)
                    .child({
                        let mut bar = TabBar::new("response-tabs");
                        for (tab, label) in tabs {
                            let is_active = tab == active_tab;
                            let item = Tab::new()
                                .label(label)
                                .selected(is_active)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_select_tab(this, tab, window, cx);
                                }));
                            bar = bar.child(item);
                        }
                        bar
                    }),
            )
            // Panel Body
            .child(
                div()
                    .id("response-panel-scroll")
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .p_4()
                    .overflow_y_scroll()
                    .overflow_x_hidden()
                    .child(Self::render_tab_content(
                        resp, active_tab, body_mode, border, muted,
                    )),
            )
    }

    fn render_tab_content(
        resp: &AuraResponse,
        tab: ResponseTab,
        body_mode: ResponseBodyMode,
        border: Rgba,
        muted: Rgba,
    ) -> Div {
        match tab {
            ResponseTab::Body => {
                let formatted = Self::format_body(&resp.body, body_mode);
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .min_w_0()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child(match body_mode {
                                        ResponseBodyMode::Pretty => "PRETTY FORMATTED BODY",
                                        ResponseBodyMode::Raw => "RAW BODY CONTENT",
                                    }),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted)
                                    .child(format!("{} characters", resp.body.len())),
                            ),
                    )
                    .child(
                        div()
                            .p_3()
                            .rounded_md()
                            .bg(rgb(0x141416))
                            .border_1()
                            .border_color(border)
                            .font_family("monospace")
                            .text_xs()
                            .text_color(rgb(0xcccccc))
                            .min_w_0()
                            .overflow_x_hidden()
                            .child(if formatted.is_empty() {
                                "// Response body is empty".to_string()
                            } else {
                                formatted
                            }),
                    )
            }
            ResponseTab::Raw => {
                // Postman & Bruno style Raw HTTP Response (Status line + Headers + Blank line + Body)
                let mut raw_http = format!("HTTP/1.1 {} {}\n", resp.status, resp.status_text);
                let mut header_entries: Vec<_> = resp.headers.iter().collect();
                header_entries.sort_by_key(|(k, _)| (*k).clone());
                for (k, v) in header_entries {
                    raw_http.push_str(&format!("{}: {}\n", k, v));
                }
                raw_http.push('\n');
                raw_http.push_str(&resp.body);

                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .min_w_0()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(muted)
                            .child("FULL RAW HTTP PAYLOAD"),
                    )
                    .child(
                        div()
                            .p_3()
                            .rounded_md()
                            .bg(rgb(0x141416))
                            .border_1()
                            .border_color(border)
                            .font_family("monospace")
                            .text_xs()
                            .text_color(rgb(0xcccccc))
                            .min_w_0()
                            .overflow_x_hidden()
                            .child(raw_http),
                    )
            }
            ResponseTab::Json => {
                let json_pretty = match serde_json::from_str::<serde_json::Value>(&resp.body) {
                    Ok(val) => {
                        serde_json::to_string_pretty(&val).unwrap_or_else(|_| resp.body.clone())
                    }
                    Err(_) => "/* Response body is not valid JSON */\n\n".to_string() + &resp.body,
                };
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .min_w_0()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(muted)
                            .child("JSON VIEWER"),
                    )
                    .child(
                        div()
                            .p_3()
                            .rounded_md()
                            .bg(rgb(0x141416))
                            .border_1()
                            .border_color(border)
                            .font_family("monospace")
                            .text_xs()
                            .text_color(rgb(0xcccccc))
                            .min_w_0()
                            .overflow_x_hidden()
                            .child(json_pretty),
                    )
            }
            ResponseTab::Headers => {
                let mut header_entries: Vec<_> = resp.headers.iter().collect();
                header_entries.sort_by_key(|(k, _)| (*k).clone());

                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .min_w_0()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child(format!("RESPONSE HEADERS ({})", header_entries.len())),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted)
                                    .child("Case-insensitive RFC 7230 headers"),
                            ),
                    )
                    .child(div().flex().flex_col().gap_1().min_w_0().children(
                        header_entries.into_iter().map(|(k, v)| {
                            div()
                                .flex()
                                .items_start()
                                .justify_between()
                                .p_2()
                                .rounded_md()
                                .bg(rgb(0x272729))
                                .border_1()
                                .border_color(border)
                                .text_xs()
                                .min_w_0()
                                .child(
                                    div()
                                        .flex()
                                        .items_start()
                                        .gap_2()
                                        .min_w_0()
                                        .flex_1()
                                        .child(
                                            div()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(rgb(0x2997ff))
                                                .flex_shrink_0()
                                                .child(format!("{}:", k)),
                                        )
                                        .child(
                                            div()
                                                .font_family("monospace")
                                                .text_color(rgb(0xffffff))
                                                .min_w_0()
                                                .flex_1()
                                                .whitespace_normal()
                                                .child(v.clone()),
                                        ),
                                )
                        }),
                    ))
            }
            ResponseTab::Tests => div()
                .flex()
                .flex_col()
                .gap_2()
                .min_w_0()
                .when(resp.assertions.is_empty(), |el| {
                    el.child(
                        div()
                            .py_3()
                            .text_xs()
                            .text_color(muted)
                            .child("No test assertions executed."),
                    )
                })
                .children(resp.assertions.iter().map(|a| {
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .p_2()
                        .rounded_md()
                        .bg(rgb(0x272729))
                        .border_1()
                        .border_color(border)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .text_xs()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .text_color(if a.passed {
                                                    rgb(0x28a745)
                                                } else {
                                                    rgb(0xe74c3c)
                                                })
                                                .child(if a.passed {
                                                    "✓ PASS"
                                                } else {
                                                    "✗ FAIL"
                                                }),
                                        )
                                        .child(
                                            div().text_color(rgb(0xffffff)).child(a.name.clone()),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_color(muted)
                                        .child(format!("{} ms", a.duration_ms)),
                                ),
                        )
                        .when(!a.passed && a.message.is_some(), |el| {
                            el.child(
                                div()
                                    .p_2()
                                    .rounded_sm()
                                    .bg(rgb(0x3a1d1d))
                                    .font_family("monospace")
                                    .text_xs()
                                    .text_color(rgb(0xff8888))
                                    .child(a.message.clone().unwrap_or_default()),
                            )
                        })
                })),
            ResponseTab::Console => div()
                .flex()
                .flex_col()
                .gap_1()
                .min_w_0()
                .when(resp.console_logs.is_empty(), |el| {
                    el.child(
                        div()
                            .py_3()
                            .text_xs()
                            .text_color(muted)
                            .child("No console output recorded."),
                    )
                })
                .children(resp.console_logs.iter().map(|log| {
                    let level_color = match log.level.to_uppercase().as_str() {
                        "WARN" => rgb(0xf5a623),
                        "ERROR" => rgb(0xe74c3c),
                        _ => rgb(0x2997ff),
                    };
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .p_2()
                        .rounded_md()
                        .bg(rgb(0x272729))
                        .border_1()
                        .border_color(border)
                        .text_xs()
                        .min_w_0()
                        .child(
                            div()
                                .px_1()
                                .rounded_sm()
                                .bg(rgb(0x1d1d1f))
                                .text_color(level_color)
                                .child(log.level.to_uppercase()),
                        )
                        .child(
                            div()
                                .text_color(muted)
                                .child(log.timestamp.format("%H:%M:%S%.3f").to_string()),
                        )
                        .child(
                            div()
                                .text_color(rgb(0xcccccc))
                                .min_w_0()
                                .overflow_x_hidden()
                                .child(log.message.clone()),
                        )
                })),
            _ => div()
                .text_xs()
                .text_color(muted)
                .child("Tab not implemented"),
        }
    }
}
