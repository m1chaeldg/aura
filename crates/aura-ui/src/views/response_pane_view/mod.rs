//! ResponsePane View: Status pills, metrics, tabs, tests, console logs, and custom viewers.

mod table_panel;
mod tree_panel;
mod visualize_panel;

use crate::services::{
    AggregationOp, ChartType, FormatService, ResponseLanguage,
};
use crate::stores::{ResponseBodyMode, ResponseTab};
use aura_core::model::{AuraResponse, TestAssertion};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::base::SelectableText;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyTitle};
use gpui_kit::component::input::{Editor, EditorState, InputState};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::tag::{Tag, TagVariant};
use gpui_kit::component::{Selectable, Sizable};
use std::collections::HashSet;

/// Actions emitted by `ResponsePaneView`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponsePaneAction {
    SelectTab(ResponseTab),
    SetBodyMode(ResponseBodyMode),
    ToggleBodyMode,
    CycleLanguage(ResponseLanguage),
    ToggleLanguageDropdown,
    DismissLanguageDropdown,
    SetLanguage(ResponseLanguage),
    ToggleSoftWrap,
    CopyBody,
    CopyText { label: &'static str, text: String },
    ToggleTreeNode(String),
    ExpandAllTree,
    ToggleTableSort(usize),
    SetBiChartType(ChartType),
    CycleBiAggOp,
    CycleBiXCol(Vec<String>),
    CycleBiYCol(Vec<String>),
    CycleBiPivotCol(Vec<String>),
}

pub struct ResponsePaneView;

impl ResponsePaneView {
    pub fn format_body(body: &str, mode: ResponseBodyMode) -> String {
        let lang = FormatService::detect_language(body, None);
        FormatService::format_response_body(body, lang, mode)
    }

    pub fn assertion_counts(assertions: &[TestAssertion]) -> (usize, usize) {
        let total = assertions.len();
        let passed = assertions.iter().filter(|a| a.passed).count();
        (passed, total)
    }

    pub fn format_tests_text(resp: &AuraResponse) -> String {
        if resp.assertions.is_empty() {
            return "// No test assertions executed.".to_string();
        }
        let (passed, total) = Self::assertion_counts(&resp.assertions);
        let failed = total.saturating_sub(passed);
        let mut lines = vec![format!(
            "# Test Summary: {passed}/{total} passed, {failed} failed"
        )];
        for (idx, a) in resp.assertions.iter().enumerate() {
            let badge = if a.passed { "[PASS]" } else { "[FAIL]" };
            lines.push(format!(
                "{:>2}. {} {} ({} ms)",
                idx + 1,
                badge,
                a.name,
                a.duration_ms
            ));
            if !a.passed {
                if let Some(msg) = &a.message {
                    for mline in msg.lines() {
                        lines.push(format!("    Error: {mline}"));
                    }
                }
            }
        }
        lines.join("\n")
    }

    pub fn format_console_text(resp: &AuraResponse) -> String {
        if resp.console_logs.is_empty() {
            return "// No console output recorded.".to_string();
        }
        resp.console_logs
            .iter()
            .map(|log| {
                format!(
                    "[{}] [{:<5}] {}",
                    log.timestamp.format("%H:%M:%S%.3f"),
                    log.level.to_uppercase(),
                    log.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
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
        _split_ratio: f32,
        _is_horizontal: bool,
        response: Option<&AuraResponse>,
        active_tab: ResponseTab,
        body_mode: ResponseBodyMode,
        language_override: Option<ResponseLanguage>,
        is_lang_dropdown_open: bool,
        soft_wrap: bool,
        tree_collapsed: &HashSet<String>,
        table_search: &str,
        table_sort_col: Option<usize>,
        table_sort_asc: bool,
        bi_chart_type: ChartType,
        bi_agg_op: AggregationOp,
        bi_x_col: Option<&str>,
        bi_y_col: Option<&str>,
        bi_pivot_col: Option<&str>,
        body_editor: Option<&Entity<EditorState>>,
        raw_editor: Option<&Entity<EditorState>>,
        tests_editor: Option<&Entity<EditorState>>,
        console_editor: Option<&Entity<EditorState>>,
        table_search_input: Option<&Entity<InputState>>,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, ResponsePaneAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let border = rgb(0x333333);
        let surface = rgb(0x272729);
        let text_muted = rgb(0x7a7a7a);

        div()
            .flex()
            .flex_col()
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .bg(rgb(0x1b1b1d))
            .child(match response {
                None => Self::render_empty_state(border, text_muted),
                Some(resp) => Self::render_active_response(
                    resp,
                    active_tab,
                    body_mode,
                    language_override,
                    is_lang_dropdown_open,
                    soft_wrap,
                    tree_collapsed,
                    table_search,
                    table_sort_col,
                    table_sort_asc,
                    bi_chart_type,
                    bi_agg_op,
                    bi_x_col,
                    bi_y_col,
                    bi_pivot_col,
                    body_editor,
                    raw_editor,
                    tests_editor,
                    console_editor,
                    table_search_input,
                    border,
                    surface,
                    text_muted,
                    cx,
                    on_action,
                ),
            })
    }

    fn render_empty_state(border: Rgba, text_muted: Rgba) -> Div {
        div()
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
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0x28a745))
                            .child("RESPONSE PANE"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(text_muted)
                            .child("Status: -- • Time: --ms • Size: --B"),
                    ),
            )
            .child(
                Empty::new().header(
                    EmptyHeader::new()
                        .title(EmptyTitle::new().child("Ready to dispatch request"))
                        .description(EmptyDescription::new().child(
                            "Send a request with 'Send ↵' or Ctrl+Enter to inspect headers, raw body, and formatting.",
                        )),
                ),
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_active_response<V: 'static>(
        resp: &AuraResponse,
        active_tab: ResponseTab,
        body_mode: ResponseBodyMode,
        language_override: Option<ResponseLanguage>,
        is_lang_dropdown_open: bool,
        soft_wrap: bool,
        tree_collapsed: &HashSet<String>,
        table_search: &str,
        table_sort_col: Option<usize>,
        table_sort_asc: bool,
        bi_chart_type: ChartType,
        bi_agg_op: AggregationOp,
        bi_x_col: Option<&str>,
        bi_y_col: Option<&str>,
        bi_pivot_col: Option<&str>,
        body_editor: Option<&Entity<EditorState>>,
        raw_editor: Option<&Entity<EditorState>>,
        tests_editor: Option<&Entity<EditorState>>,
        console_editor: Option<&Entity<EditorState>>,
        table_search_input: Option<&Entity<InputState>>,
        border: Rgba,
        surface: Rgba,
        muted: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, ResponsePaneAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let effective_tab = if active_tab == ResponseTab::Json {
            ResponseTab::Body
        } else {
            active_tab
        };
        let (passed_tests, total_tests) = Self::assertion_counts(&resp.assertions);
        let tabs = [
            (ResponseTab::Body, "Body".to_string()),
            (
                ResponseTab::Headers,
                format!("Headers ({})", resp.headers.len()),
            ),
            (ResponseTab::Table, "Table".to_string()),
            (ResponseTab::Tree, "Tree".to_string()),
            (ResponseTab::Visualize, "Visualize".to_string()),
            (ResponseTab::Raw, "Raw".to_string()),
            (
                ResponseTab::Tests,
                format!("Tests ({}/{})", passed_tests, total_tests),
            ),
            (
                ResponseTab::Console,
                format!("Console ({})", resp.console_logs.len()),
            ),
        ];

        let status_variant = match resp.status {
            200..=299 => TagVariant::Success,
            300..=399 => TagVariant::Primary,
            400..=499 => TagVariant::Warning,
            500..=599 => TagVariant::Danger,
            _ => TagVariant::Secondary,
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
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
                            .child(
                                Tag::new()
                                    .with_variant(status_variant)
                                    .small()
                                    .child(format!("{} {}", resp.status, resp.status_text)),
                            )
                            .child(
                                Tag::secondary()
                                    .outline()
                                    .small()
                                    .child(format!("⏱ {} ms", resp.duration_ms)),
                            )
                            .child(
                                Tag::secondary()
                                    .outline()
                                    .small()
                                    .child(format!("📦 {}", Self::format_size(resp.size_bytes))),
                            )
                            .when(!resp.script_errors.is_empty(), |el| {
                                el.child(
                                    Tag::new()
                                        .with_variant(TagVariant::Danger)
                                        .small()
                                        .child(format!(
                                            "⚠ {} Script Error{}",
                                            resp.script_errors.len(),
                                            if resp.script_errors.len() == 1 { "" } else { "s" }
                                        )),
                                )
                            }),
                    )
                    .child(
                        Button::new("body-mode-toggle-btn")
                            .label(match body_mode {
                                ResponseBodyMode::Pretty => "Format: Pretty ▾",
                                ResponseBodyMode::Compress => "Format: Compress ▾",
                                ResponseBodyMode::Raw => "Format: Raw ▾",
                            })
                            .xsmall()
                            .secondary()
                            .rounded(ButtonRounded::Small)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, ResponsePaneAction::ToggleBodyMode, window, cx);
                            })),
                    ),
            )
            .when(!resp.script_errors.is_empty(), |el| {
                el.child(Self::render_script_errors_banner(&resp.script_errors))
            })
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
                            let is_active = tab == effective_tab;
                            let item = Tab::new()
                                .label(label)
                                .selected(is_active)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_action(
                                        this,
                                        ResponsePaneAction::SelectTab(tab),
                                        window,
                                        cx,
                                    );
                                }));
                            bar = bar.child(item);
                        }
                        bar
                    }),
            )
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
                        resp,
                        effective_tab,
                        body_mode,
                        language_override,
                        is_lang_dropdown_open,
                        soft_wrap,
                        tree_collapsed,
                        table_search,
                        table_sort_col,
                        table_sort_asc,
                        bi_chart_type,
                        bi_agg_op,
                        bi_x_col,
                        bi_y_col,
                        bi_pivot_col,
                        body_editor,
                        raw_editor,
                        tests_editor,
                        console_editor,
                        table_search_input,
                        border,
                        muted,
                        cx,
                        on_action,
                    )),
            )
    }

    fn render_script_errors_banner(errors: &[aura_core::model::ScriptError]) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_1p5()
            .px_3()
            .py_2()
            .bg(rgb(0x3a1818))
            .border_b_1()
            .border_color(rgb(0xe74c3c))
            .children(errors.iter().enumerate().map(|(idx, err)| {
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        Tag::new()
                            .with_variant(TagVariant::Danger)
                            .xsmall()
                            .child(format!("⚠ {} Error", err.stage)),
                    )
                    .child(
                        div()
                            .p_2()
                            .rounded_sm()
                            .bg(rgb(0x261212))
                            .border_1()
                            .border_color(rgb(0x7a2b2b))
                            .font_family("monospace")
                            .text_xs()
                            .text_color(rgb(0xff9b9b))
                            .child(SelectableText::new(
                                ElementId::NamedInteger("resp-script-err".into(), idx as u64),
                                err.message.clone(),
                            )),
                    )
            }))
    }

    #[allow(clippy::too_many_arguments)]
    fn render_tab_content<V: 'static>(
        resp: &AuraResponse,
        tab: ResponseTab,
        body_mode: ResponseBodyMode,
        language_override: Option<ResponseLanguage>,
        is_lang_dropdown_open: bool,
        soft_wrap: bool,
        tree_collapsed: &HashSet<String>,
        table_search: &str,
        table_sort_col: Option<usize>,
        table_sort_asc: bool,
        bi_chart_type: ChartType,
        bi_agg_op: AggregationOp,
        bi_x_col: Option<&str>,
        bi_y_col: Option<&str>,
        bi_pivot_col: Option<&str>,
        body_editor: Option<&Entity<EditorState>>,
        raw_editor: Option<&Entity<EditorState>>,
        tests_editor: Option<&Entity<EditorState>>,
        console_editor: Option<&Entity<EditorState>>,
        table_search_input: Option<&Entity<InputState>>,
        border: Rgba,
        muted: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, ResponsePaneAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        match tab {
            ResponseTab::Body | ResponseTab::Json => Self::render_body_tab(
                resp,
                body_mode,
                language_override,
                is_lang_dropdown_open,
                soft_wrap,
                body_editor,
                border,
                muted,
                cx,
                on_action,
            ),
            ResponseTab::Raw => Self::render_raw_tab(resp, raw_editor, border, muted),
            ResponseTab::Table => Self::render_table_panel(
                resp,
                table_search,
                table_sort_col,
                table_sort_asc,
                table_search_input,
                border,
                muted,
                cx,
                on_action,
            ),
            ResponseTab::Tree => {
                Self::render_tree_panel(resp, tree_collapsed, border, muted, cx, on_action)
            }
            ResponseTab::Headers => Self::render_headers_tab(resp, border, muted),
            ResponseTab::Tests => {
                Self::render_tests_tab(resp, tests_editor, border, muted, cx, on_action)
            }
            ResponseTab::Console => {
                Self::render_console_tab(resp, console_editor, border, muted, cx, on_action)
            }
            ResponseTab::Visualize => Self::render_visualize_panel(
                resp,
                bi_chart_type,
                bi_agg_op,
                bi_x_col,
                bi_y_col,
                bi_pivot_col,
                border,
                muted,
                cx,
                on_action,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_body_tab<V: 'static>(
        resp: &AuraResponse,
        body_mode: ResponseBodyMode,
        language_override: Option<ResponseLanguage>,
        is_lang_dropdown_open: bool,
        soft_wrap: bool,
        body_editor: Option<&Entity<EditorState>>,
        border: Rgba,
        muted: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, ResponsePaneAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let content_type = resp
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.as_str());
        let detected_lang = FormatService::detect_language(&resp.body, content_type);
        let effective_lang = language_override.unwrap_or(detected_lang);
        let formatted = FormatService::format_response_body(&resp.body, effective_lang, body_mode);

        let mode_btn = |id: &'static str, label: &'static str, target: ResponseBodyMode| {
            let btn = Button::new(id)
                .label(label)
                .xsmall()
                .rounded(ButtonRounded::Small)
                .on_click(cx.listener(move |this, _, window, cx| {
                    on_action(this, ResponsePaneAction::SetBodyMode(target), window, cx);
                }));
            if body_mode == target {
                btn.primary()
            } else {
                btn.secondary()
            }
        };

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
                    .gap_2()
                    .flex_wrap()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(mode_btn("resp-mode-pretty", "Pretty", ResponseBodyMode::Pretty))
                            .child(mode_btn(
                                "resp-mode-compress",
                                "Compress",
                                ResponseBodyMode::Compress,
                            ))
                            .child(mode_btn("resp-mode-raw", "Raw", ResponseBodyMode::Raw))
                            .child({
                                const ALL_LANGS: [ResponseLanguage; 4] = [
                                    ResponseLanguage::Json,
                                    ResponseLanguage::Xml,
                                    ResponseLanguage::Html,
                                    ResponseLanguage::Text,
                                ];
                                div()
                                    .relative()
                                    .child(
                                        Button::new("resp-lang-dropdown-btn")
                                            .label(format!("Lang: {} ▾", effective_lang.label()))
                                            .xsmall()
                                            .secondary()
                                            .rounded(ButtonRounded::Small)
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                on_action(
                                                    this,
                                                    ResponsePaneAction::ToggleLanguageDropdown,
                                                    window,
                                                    cx,
                                                );
                                            })),
                                    )
                                    .when(is_lang_dropdown_open, |el| {
                                        let menu = div()
                                            .id("resp-lang-dropdown-menu")
                                            .absolute()
                                            .top_full()
                                            .left_0()
                                            .mt_1()
                                            .w(px(120.0))
                                            .bg(rgb(0x252528))
                                            .border_1()
                                            .border_color(border)
                                            .rounded_md()
                                            .p_1()
                                            .shadow_lg()
                                            .flex()
                                            .flex_col()
                                            .gap_0p5()
                                            .on_mouse_down_out(cx.listener(
                                                move |this, _, window, cx| {
                                                    on_action(
                                                        this,
                                                        ResponsePaneAction::DismissLanguageDropdown,
                                                        window,
                                                        cx,
                                                    );
                                                },
                                            ))
                                            .children(ALL_LANGS.iter().enumerate().map(
                                                |(idx, &lang)| {
                                                    let is_selected = lang == effective_lang;
                                                    ListItem::new(ElementId::NamedInteger(
                                                        "resp-lang-opt".into(),
                                                        idx as u64,
                                                    ))
                                                    .selected(is_selected)
                                                    .px_2()
                                                    .py_1()
                                                    .rounded_sm()
                                                    .on_click(cx.listener(
                                                        move |this, _, window, cx| {
                                                            on_action(
                                                                this,
                                                                ResponsePaneAction::SetLanguage(
                                                                    lang,
                                                                ),
                                                                window,
                                                                cx,
                                                            );
                                                        },
                                                    ))
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .justify_between()
                                                            .w_full()
                                                            .text_xs()
                                                            .child(
                                                                div()
                                                                    .text_color(if is_selected {
                                                                        rgb(0xffffff)
                                                                    } else {
                                                                        rgb(0xcccccc)
                                                                    })
                                                                    .child(lang.label()),
                                                            )
                                                            .when(is_selected, |el| {
                                                                el.child(
                                                                    div()
                                                                        .text_color(rgb(0x2997ff))
                                                                        .child("✓"),
                                                                )
                                                            }),
                                                    )
                                                },
                                            ));
                                        el.child(deferred(menu).with_priority(1))
                                    })
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted)
                                    .child(format!("{} chars", formatted.len())),
                            )
                            .child({
                                let wrap_btn = Button::new("resp-soft-wrap-btn")
                                    .label(if soft_wrap { "Wrap: On" } else { "Wrap: Off" })
                                    .xsmall()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            ResponsePaneAction::ToggleSoftWrap,
                                            window,
                                            cx,
                                        );
                                    }));
                                if soft_wrap {
                                    wrap_btn.primary()
                                } else {
                                    wrap_btn.secondary()
                                }
                            })
                            .child(
                                Button::new("resp-copy-body-btn")
                                    .label("Copy")
                                    .xsmall()
                                    .secondary()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            ResponsePaneAction::CopyBody,
                                            window,
                                            cx,
                                        );
                                    })),
                            ),
                    ),
            )
            .child(if formatted.is_empty() {
                div()
                    .p_2()
                    .text_xs()
                    .text_color(muted)
                    .child("// Response body is empty")
                    .into_any_element()
            } else if let Some(ed) = body_editor {
                div()
                    .h(px(360.0))
                    .w_full()
                    .rounded_md()
                    .border_1()
                    .border_color(border)
                    .overflow_hidden()
                    .child(Editor::new(ed).readonly(true).h(px(360.0)).w_full())
                    .into_any_element()
            } else {
                div()
                    .p_2()
                    .font_family("monospace")
                    .text_xs()
                    .text_color(rgb(0xcccccc))
                    .child(formatted)
                    .into_any_element()
            })
    }

    fn render_raw_tab(
        resp: &AuraResponse,
        raw_editor: Option<&Entity<EditorState>>,
        border: Rgba,
        muted: Rgba,
    ) -> Div {
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
            .child(if let Some(ed) = raw_editor {
                div()
                    .h(px(360.0))
                    .w_full()
                    .rounded_md()
                    .border_1()
                    .border_color(border)
                    .overflow_hidden()
                    .child(Editor::new(ed).readonly(true).h(px(360.0)).w_full())
                    .into_any_element()
            } else {
                div()
                    .p_2()
                    .font_family("monospace")
                    .text_xs()
                    .text_color(rgb(0xcccccc))
                    .child(raw_http)
                    .into_any_element()
            })
    }

    fn render_headers_tab(resp: &AuraResponse, border: Rgba, muted: Rgba) -> Div {
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
                header_entries.into_iter().enumerate().map(|(idx, (k, v))| {
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
                                        .child(SelectableText::new(
                                            ElementId::NamedInteger(
                                                "resp-hdr-k".into(),
                                                idx as u64,
                                            ),
                                            format!("{}:", k),
                                        )),
                                )
                                .child(
                                    div()
                                        .font_family("monospace")
                                        .text_color(rgb(0xffffff))
                                        .min_w_0()
                                        .flex_1()
                                        .whitespace_normal()
                                        .child(SelectableText::new(
                                            ElementId::NamedInteger(
                                                "resp-hdr-v".into(),
                                                idx as u64,
                                            ),
                                            v.clone(),
                                        )),
                                ),
                        )
                }),
            ))
    }

    fn render_tests_tab<V: 'static>(
        resp: &AuraResponse,
        tests_editor: Option<&Entity<EditorState>>,
        border: Rgba,
        muted: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, ResponsePaneAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let (passed, total) = Self::assertion_counts(&resp.assertions);
        let failed = total.saturating_sub(passed);
        let formatted_text = Self::format_tests_text(resp);

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
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child(format!("TEST RESULTS ({passed}/{total})")),
                            )
                            .when(total > 0, |el| {
                                el.child(
                                    Tag::new()
                                        .with_variant(TagVariant::Success)
                                        .xsmall()
                                        .child(format!("✓ {passed} Passed")),
                                )
                                .when(failed > 0, |el| {
                                    el.child(
                                        Tag::new()
                                            .with_variant(TagVariant::Danger)
                                            .xsmall()
                                            .child(format!("✗ {failed} Failed")),
                                    )
                                })
                            }),
                    )
                    .when(total > 0, |el| {
                        let copy_text = formatted_text.clone();
                        el.child(
                            Button::new("resp-copy-tests-btn")
                                .label("Copy All")
                                .xsmall()
                                .secondary()
                                .rounded(ButtonRounded::Small)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_action(
                                        this,
                                        ResponsePaneAction::CopyText {
                                            label: "Test Results",
                                            text: copy_text.clone(),
                                        },
                                        window,
                                        cx,
                                    );
                                })),
                        )
                    }),
            )
            .child(if resp.assertions.is_empty() {
                Empty::new()
                    .header(
                        EmptyHeader::new()
                            .description(EmptyDescription::new().child("No test assertions executed.")),
                    )
                    .into_any_element()
            } else if let Some(ed) = tests_editor {
                div()
                    .h(px(360.0))
                    .w_full()
                    .rounded_md()
                    .border_1()
                    .border_color(border)
                    .overflow_hidden()
                    .child(Editor::new(ed).readonly(true).h(px(360.0)).w_full())
                    .into_any_element()
            } else {
                div()
                    .p_2()
                    .font_family("monospace")
                    .text_xs()
                    .text_color(rgb(0xcccccc))
                    .child(formatted_text)
                    .into_any_element()
            })
    }

    fn render_console_tab<V: 'static>(
        resp: &AuraResponse,
        console_editor: Option<&Entity<EditorState>>,
        border: Rgba,
        muted: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, ResponsePaneAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let total = resp.console_logs.len();
        let warn_count = resp
            .console_logs
            .iter()
            .filter(|l| l.level.eq_ignore_ascii_case("WARN"))
            .count();
        let err_count = resp
            .console_logs
            .iter()
            .filter(|l| l.level.eq_ignore_ascii_case("ERROR"))
            .count();
        let formatted_text = Self::format_console_text(resp);

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
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child(format!("CONSOLE OUTPUT ({total})")),
                            )
                            .when(warn_count > 0, |el| {
                                el.child(
                                    Tag::new()
                                        .with_variant(TagVariant::Warning)
                                        .xsmall()
                                        .child(format!("{warn_count} Warn")),
                                )
                            })
                            .when(err_count > 0, |el| {
                                el.child(
                                    Tag::new()
                                        .with_variant(TagVariant::Danger)
                                        .xsmall()
                                        .child(format!("{err_count} Error")),
                                )
                            }),
                    )
                    .when(total > 0, |el| {
                        let copy_text = formatted_text.clone();
                        el.child(
                            Button::new("resp-copy-console-btn")
                                .label("Copy All")
                                .xsmall()
                                .secondary()
                                .rounded(ButtonRounded::Small)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_action(
                                        this,
                                        ResponsePaneAction::CopyText {
                                            label: "Console Logs",
                                            text: copy_text.clone(),
                                        },
                                        window,
                                        cx,
                                    );
                                })),
                        )
                    }),
            )
            .child(if resp.console_logs.is_empty() {
                Empty::new()
                    .header(
                        EmptyHeader::new()
                            .description(EmptyDescription::new().child("No console output recorded.")),
                    )
                    .into_any_element()
            } else if let Some(ed) = console_editor {
                div()
                    .h(px(360.0))
                    .w_full()
                    .rounded_md()
                    .border_1()
                    .border_color(border)
                    .overflow_hidden()
                    .child(Editor::new(ed).readonly(true).h(px(360.0)).w_full())
                    .into_any_element()
            } else {
                div()
                    .p_2()
                    .font_family("monospace")
                    .text_xs()
                    .text_color(rgb(0xcccccc))
                    .child(formatted_text)
                    .into_any_element()
            })
    }
}
