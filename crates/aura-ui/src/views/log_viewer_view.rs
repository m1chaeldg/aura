//! LogViewer View: Interactive diagnostic and structured logging modal overlay using gpui_kit.

use crate::app_state::AuraAppState;
use crate::theme::icon;
use crate::views::workspace_view::AuraWorkspaceView;
use aura_engine::LogLevel;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::base::SelectableText;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader};
use gpui_kit::component::input::{Textarea, TextareaState};
use gpui_kit::component::separator::Separator;
use gpui_kit::component::sheet::Sheet;
use gpui_kit::component::tag::{Tag, TagVariant};
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex};

pub struct LogViewerView;

impl LogViewerView {
    pub fn render(
        state: &AuraAppState,
        log_textarea: Option<&Entity<TextareaState>>,
        window: &mut Window,
        cx: &mut Context<AuraWorkspaceView>,
    ) -> impl IntoElement {
        let accent_blue = cx.theme().primary;
        let surface_tile_1 = cx.theme().background;
        let surface_tile_2 = cx.theme().list_head;
        let surface_tile_3 = cx.theme().secondary.opacity(0.35);
        let text_color = cx.theme().foreground;
        let text_muted = cx.theme().muted_foreground;
        let border_color = cx.theme().border;
        let row_hover = cx.theme().list_hover;

        let filtered_logs = state.get_filtered_logs();
        let total_count = aura_engine::LogBroadcaster::global().get_entries().len();
        let active_level = state.log_filter_level;
        let active_subsystem = state.log_filter_subsystem.clone();
        let log_dir_str = aura_engine::LogBroadcaster::global()
            .log_dir()
            .display()
            .to_string();

        let view_for_close = cx.entity().clone();
        let title_el = div()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .pr_4()
            .child(
                h_flex()
                    .gap_2()
                    .child(icon(IconName::ScrollText).text_color(accent_blue))
                    .child("System Logs & Diagnostics")
                    .child(div().text_xs().text_color(text_muted).child(format!(
                        "({} / {})",
                        filtered_logs.len(),
                        total_count
                    ))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child({
                                let btn = Button::new("log-mode-table")
                                    .label("Table")
                                    .xsmall()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.state.set_log_viewer_text_mode(false);
                                        cx.notify();
                                    }));
                                if !state.is_log_viewer_text_mode {
                                    btn.primary()
                                } else {
                                    btn.secondary()
                                }
                            })
                            .child({
                                let btn = Button::new("log-mode-text")
                                    .label("Text")
                                    .xsmall()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.state.set_log_viewer_text_mode(true);
                                        cx.notify();
                                    }));
                                if state.is_log_viewer_text_mode {
                                    btn.primary()
                                } else {
                                    btn.secondary()
                                }
                            }),
                    )
                    .child(
                        Button::new("copy-logs-btn")
                            .icon(icon(IconName::Copy))
                            .label("Copy")
                            .small()
                            .rounded(ButtonRounded::Medium)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                let text = this.state.format_filtered_logs_text();
                                cx.write_to_clipboard(ClipboardItem::new_string(text));
                                this.state.status_message =
                                    Some("Copied logs to clipboard".to_string());
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("clear-logs-btn")
                            .icon(icon(IconName::Trash))
                            .label("Clear")
                            .small()
                            .rounded(ButtonRounded::Medium)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.state.clear_logs();
                                cx.notify();
                            })),
                    ),
            );

        let footer_el = div()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .text_xs()
            .text_color(text_muted)
            .child(
                h_flex()
                    .gap_2()
                    .child(icon(IconName::FolderOpen).xsmall())
                    .child("Log Directory:")
                    .child(div().text_color(text_color).child(log_dir_str)),
            )
            .child("Esc or Ctrl+Shift+L to close");

        Sheet::new(window, cx)
            .size(px(640.0))
            .resizable(true)
            .title(title_el)
            .footer(footer_el)
            .on_close(move |_, _window, app| {
                let _ = view_for_close.update(app, |this, cx| {
                    this.state.is_log_viewer_open = false;
                    cx.notify();
                });
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .size_full()
                    .bg(surface_tile_1)
                    // Filter Toolbar
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .justify_between()
                            .gap_x_4()
                            .gap_y_1p5()
                            .px_3()
                            .py_2()
                            .bg(surface_tile_1)
                            // Level Filter Pills
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(div().text_xs().text_color(text_muted).child("Level:"))
                                    .children(
                                        [
                                            (None, "ALL"),
                                            (Some(LogLevel::ERROR), "ERROR"),
                                            (Some(LogLevel::WARN), "WARN"),
                                            (Some(LogLevel::INFO), "INFO"),
                                            (Some(LogLevel::DEBUG), "DEBUG"),
                                            (Some(LogLevel::TRACE), "TRACE"),
                                        ]
                                        .into_iter()
                                        .map(|(lvl, label)| {
                                            let is_active = active_level == lvl;
                                            let btn = Button::new(ElementId::Name(
                                                format!("log-lvl-{label}").into(),
                                            ))
                                            .label(label)
                                            .xsmall()
                                            .rounded(ButtonRounded::Small)
                                            .on_click(cx.listener(
                                                move |this, _, _window, cx| {
                                                    this.state.set_log_filter_level(lvl);
                                                    cx.notify();
                                                },
                                            ));
                                            if is_active {
                                                btn.primary()
                                            } else {
                                                btn.secondary()
                                            }
                                        }),
                                    ),
                            )
                            // Subsystem Filter Pills
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div().text_xs().text_color(text_muted).child("Subsystem:"),
                                    )
                                    .children(
                                        [
                                            (None, "All"),
                                            (Some("aura_engine".to_string()), "Engine"),
                                            (Some("aura_ui".to_string()), "UI"),
                                            (Some("aura_sync".to_string()), "Sync"),
                                            (Some("aura_parser".to_string()), "Parser"),
                                        ]
                                        .into_iter()
                                        .map(|(sub, label)| {
                                            let is_active = active_subsystem == sub;
                                            let btn = Button::new(ElementId::Name(
                                                format!("log-sub-{label}").into(),
                                            ))
                                            .label(label)
                                            .xsmall()
                                            .rounded(ButtonRounded::Small)
                                            .on_click(cx.listener(
                                                move |this, _, _window, cx| {
                                                    this.state.set_log_filter_subsystem(sub.clone());
                                                    cx.notify();
                                                },
                                            ));
                                            if is_active {
                                                btn.primary()
                                            } else {
                                                btn.secondary()
                                            }
                                        }),
                                    ),
                            ),
                    )
                    .child(Separator::horizontal())
                    .child(if state.is_log_viewer_text_mode {
                        Self::render_text_body(
                            state,
                            &filtered_logs,
                            log_textarea,
                            surface_tile_3,
                            text_color,
                        )
                    } else {
                        Self::render_table_body(
                            &filtered_logs,
                            surface_tile_2,
                            surface_tile_3,
                            accent_blue,
                            text_color,
                            text_muted,
                            border_color,
                            row_hover,
                        )
                    }),
            )
    }

    fn render_text_body(
        state: &AuraAppState,
        filtered_logs: &[aura_engine::LogEntry],
        log_textarea: Option<&Entity<TextareaState>>,
        surface_tile_3: Hsla,
        text_color: Hsla,
    ) -> AnyElement {
        if filtered_logs.is_empty() {
            return div()
                .flex()
                .flex_col()
                .flex_1()
                .bg(surface_tile_3)
                .child(
                    Empty::new().header(EmptyHeader::new().description(
                        EmptyDescription::new().child("No log entries matching criteria."),
                    )),
                )
                .into_any_element();
        }

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .p_3()
            .bg(surface_tile_3)
            .child(if let Some(ta) = log_textarea {
                Textarea::new(ta)
                    .readonly(true)
                    .size_full()
                    .font_family("monospace")
                    .text_xs()
                    .into_any_element()
            } else {
                div()
                    .p_2()
                    .font_family("monospace")
                    .text_xs()
                    .text_color(text_color)
                    .child(state.format_filtered_logs_text())
                    .into_any_element()
            })
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_table_body(
        filtered_logs: &[aura_engine::LogEntry],
        surface_tile_2: Hsla,
        surface_tile_3: Hsla,
        accent_blue: Hsla,
        text_color: Hsla,
        text_muted: Hsla,
        border_color: Hsla,
        row_hover: Hsla,
    ) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .min_w_0()
            // Log Entries Table Header
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .w_full()
                    .flex_shrink_0()
                    .px_3()
                    .py_1()
                    .bg(surface_tile_2)
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(text_muted)
                    .child(div().w(px(76.0)).flex_shrink_0().child("Timestamp"))
                    .child(div().w(px(54.0)).flex_shrink_0().child("Level"))
                    .child(div().w(px(88.0)).flex_shrink_0().child("Subsystem"))
                    .child(div().flex_1().min_w_0().child("Message")),
            )
            .child(Separator::horizontal())
            // Log Entries List
            .child(
                div()
                    .id("log-entries-scroll")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_y_scroll()
                    .bg(surface_tile_3)
                    .children(if filtered_logs.is_empty() {
                        vec![Empty::new()
                            .header(EmptyHeader::new().description(
                                EmptyDescription::new().child("No log entries matching criteria."),
                            ))
                            .into_any_element()]
                    } else {
                        filtered_logs
                            .iter()
                            .rev()
                            .take(200)
                            .enumerate()
                            .map(|(idx, entry)| {
                                let tag_variant = match entry.level {
                                    LogLevel::TRACE => TagVariant::Secondary,
                                    LogLevel::DEBUG => TagVariant::Secondary,
                                    LogLevel::INFO => TagVariant::Primary,
                                    LogLevel::WARN => TagVariant::Warning,
                                    LogLevel::ERROR => TagVariant::Danger,
                                };
                                let time_str =
                                    entry.timestamp.format("%H:%M:%S%.3f").to_string();
                                let msg = entry.message.trim_end().to_string();

                                div()
                                    .flex()
                                    .items_start()
                                    .gap_3()
                                    .w_full()
                                    .flex_shrink_0()
                                    .px_3()
                                    .py_1()
                                    .border_b_1()
                                    .border_color(border_color)
                                    .hover(move |s| s.bg(row_hover))
                                    .text_xs()
                                    .child(
                                        div()
                                            .w(px(76.0))
                                            .flex_shrink_0()
                                            .font_family("monospace")
                                            .text_color(text_muted)
                                            .child(time_str),
                                    )
                                    .child(
                                        div()
                                            .w(px(54.0))
                                            .flex_shrink_0()
                                            .flex()
                                            .items_center()
                                            .child(
                                                Tag::new()
                                                    .with_variant(tag_variant)
                                                    .xsmall()
                                                    .child(entry.level.as_str()),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .w(px(88.0))
                                            .flex_shrink_0()
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .text_color(accent_blue)
                                            .child(entry.subsystem.clone()),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .text_color(text_color)
                                            .child(SelectableText::new(
                                                ElementId::NamedInteger(
                                                    "log-entry-msg".into(),
                                                    idx as u64,
                                                ),
                                                msg,
                                            )),
                                    )
                                    .into_any_element()
                            })
                            .collect()
                    }),
            )
            .into_any_element()
    }
}
