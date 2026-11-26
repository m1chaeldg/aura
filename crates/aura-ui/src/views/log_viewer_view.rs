//! LogViewer View: Interactive diagnostic and structured logging modal overlay using gpui_kit.

use crate::app_state::AuraAppState;
use crate::views::workspace_view::AuraWorkspaceView;
use aura_engine::LogLevel;
use gpui::*;
use gpui_kit::base::SelectableText;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader};
use gpui_kit::component::separator::Separator;
use gpui_kit::component::sheet::Sheet;
use gpui_kit::component::tag::{Tag, TagVariant};
use gpui_kit::component::Sizable;

pub struct LogViewerView;

impl LogViewerView {
    pub fn render(
        state: &AuraAppState,
        window: &mut Window,
        cx: &mut Context<AuraWorkspaceView>,
    ) -> impl IntoElement {
        let accent_blue = rgb(0x0066cc);
        let surface_tile_1 = rgb(0x272729);
        let surface_tile_2 = rgb(0x2a2a2c);
        let surface_tile_3 = rgb(0x222224);
        let text_color = rgb(0xf5f5f7);
        let text_muted = rgb(0x7a7a7a);

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
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_base().text_color(accent_blue).child("🪵"))
                    .child("System Logs & Diagnostics")
                    .child(div().text_xs().text_color(text_muted).child(format!(
                        "({} / {})",
                        filtered_logs.len(),
                        total_count
                    ))),
            )
            .child(
                Button::new("clear-logs-btn")
                    .label("🗑 Clear")
                    .small()
                    .rounded(ButtonRounded::Medium)
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.state.clear_logs();
                        cx.notify();
                    })),
            );

        let footer_el = div()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .text_xs()
            .text_color(text_muted)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child("📁 Log Directory:")
                    .child(div().text_color(text_color).child(log_dir_str)),
            )
            .child("Esc or Ctrl+Shift+L to close • Zero Telemetry");

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
                            .items_center()
                            .justify_between()
                            .px_5()
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
                    // Log Entries Table Header
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .px_5()
                            .py_1p5()
                            .bg(surface_tile_2)
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(text_muted)
                            .child(div().w(px(90.0)).child("Timestamp"))
                            .child(div().w_20().child("Level"))
                            .child(div().w(px(140.0)).child("Subsystem"))
                            .child(div().flex_1().child("Message")),
                    )
                    .child(Separator::horizontal())
                    // Log Entries List
                    .child(
                        div()
                            .id("log-entries-scroll")
                            .flex()
                            .flex_col()
                            .flex_1()
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

                                        div()
                                            .flex()
                                            .items_start()
                                            .px_5()
                                            .py_1p5()
                                            .border_b_1()
                                            .border_color(rgb(0x2a2a2d))
                                            .hover(|s| s.bg(rgb(0x27272b)))
                                            .text_xs()
                                            .child(
                                                div()
                                                    .w(px(90.0))
                                                    .text_color(text_muted)
                                                    .child(time_str),
                                            )
                                            .child(
                                                div().w_20().flex().items_center().child(
                                                    Tag::new()
                                                        .with_variant(tag_variant)
                                                        .xsmall()
                                                        .child(entry.level.as_str()),
                                                ),
                                            )
                                            .child(
                                                div()
                                                    .w(px(140.0))
                                                    .text_color(accent_blue)
                                                    .overflow_hidden()
                                                    .child(entry.subsystem.clone()),
                                            )
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .text_color(text_color)
                                                    .child(SelectableText::new(
                                                        ElementId::NamedInteger(
                                                            "log-entry-msg".into(),
                                                            idx as u64,
                                                        ),
                                                        entry.message.clone(),
                                                    )),
                                            )
                                            .into_any_element()
                                    })
                                    .collect()
                            }),
                    ),
            )
    }
}
