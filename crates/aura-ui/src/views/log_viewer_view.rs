//! LogViewer View: Interactive diagnostic and structured logging modal overlay.
//! Adheres strictly to AGENTS.md Layer C and DESIGN.md styling guidelines.

use crate::app_state::AuraAppState;
use crate::views::workspace_view::AuraWorkspaceView;
use aura_engine::LogLevel;
use gpui::*;

pub struct LogViewerView;

impl LogViewerView {
    pub fn render(state: &AuraAppState, cx: &mut Context<AuraWorkspaceView>) -> impl IntoElement {
        let accent_blue = rgb(0x0066cc);
        let surface_tile_1 = rgb(0x272729);
        let surface_tile_2 = rgb(0x2a2a2c);
        let surface_tile_3 = rgb(0x222224);
        let border_color = rgb(0x333333);
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

        div()
            .id("log-viewer-modal")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .p_8()
            .bg(rgb(0x0a0a0c))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .h_full()
                    .max_w(px(1100.0))
                    .max_h(px(720.0))
                    .rounded_lg()
                    .bg(surface_tile_1)
                    .border_1()
                    .border_color(accent_blue)
                    .shadow_lg()
                    // Modal Header
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_5()
                            .py_3()
                            .border_b_1()
                            .border_color(border_color)
                            .bg(surface_tile_2)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(div().text_base().text_color(accent_blue).child("🪵"))
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(text_color)
                                            .child("System Logs & Diagnostics"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(text_muted)
                                            .child(format!(
                                                "(Showing {} of {} entries)",
                                                filtered_logs.len(),
                                                total_count
                                            )),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    // Clear Logs Button
                                    .child(
                                        div()
                                            .id("clear-logs-btn")
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .px_2()
                                            .py_1()
                                            .rounded_md()
                                            .bg(surface_tile_3)
                                            .border_1()
                                            .border_color(border_color)
                                            .hover(|s| s.bg(rgb(0x3a3a3e)))
                                            .text_xs()
                                            .text_color(text_color)
                                            .child("🗑 Clear")
                                            .on_click(cx.listener(|this, _, _window, cx| {
                                                this.state.clear_logs();
                                                cx.notify();
                                            })),
                                    )
                                    // Close Button
                                    .child(
                                        div()
                                            .id("close-log-viewer-btn")
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .w_6()
                                            .h_6()
                                            .rounded_md()
                                            .bg(surface_tile_3)
                                            .border_1()
                                            .border_color(border_color)
                                            .hover(|s| s.bg(rgb(0xe74c3c)))
                                            .text_xs()
                                            .text_color(text_color)
                                            .child("✕")
                                            .on_click(cx.listener(|this, _, _window, cx| {
                                                this.state.toggle_log_viewer();
                                                cx.notify();
                                            })),
                                    ),
                            ),
                    )
                    // Filter Toolbar
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_5()
                            .py_2()
                            .border_b_1()
                            .border_color(border_color)
                            .bg(surface_tile_1)
                            // Level Filter Pills
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(div().text_xs().text_color(text_muted).child("Level:"))
                                    .children([
                                        (None, "ALL"),
                                        (Some(LogLevel::ERROR), "ERROR"),
                                        (Some(LogLevel::WARN), "WARN"),
                                        (Some(LogLevel::INFO), "INFO"),
                                        (Some(LogLevel::DEBUG), "DEBUG"),
                                        (Some(LogLevel::TRACE), "TRACE"),
                                    ].into_iter().map(|(lvl, label)| {
                                        let is_active = active_level == lvl;
                                        let bg = if is_active { accent_blue } else { surface_tile_2 };
                                        let txt = if is_active { rgb(0xffffff) } else { text_color };

                                        div()
                                            .id(ElementId::Name(format!("log-lvl-{label}").into()))
                                            .px_2()
                                            .py_0p5()
                                            .rounded_sm()
                                            .bg(bg)
                                            .border_1()
                                            .border_color(border_color)
                                            .hover(|s| s.bg(rgb(0x3a3a3e)))
                                            .text_xs()
                                            .text_color(txt)
                                            .child(label)
                                            .on_click(cx.listener(move |this, _, _window, cx| {
                                                this.state.set_log_filter_level(lvl);
                                                cx.notify();
                                            }))
                                    }))
                            )
                            // Subsystem Filter Pills
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(div().text_xs().text_color(text_muted).child("Subsystem:"))
                                    .children([
                                        (None, "All"),
                                        (Some("aura_engine".to_string()), "Engine"),
                                        (Some("aura_ui".to_string()), "UI"),
                                        (Some("aura_sync".to_string()), "Sync"),
                                        (Some("aura_parser".to_string()), "Parser"),
                                    ].into_iter().map(|(sub, label)| {
                                        let is_active = active_subsystem == sub;
                                        let bg = if is_active { accent_blue } else { surface_tile_2 };
                                        let txt = if is_active { rgb(0xffffff) } else { text_color };

                                        div()
                                            .id(ElementId::Name(format!("log-sub-{label}").into()))
                                            .px_2()
                                            .py_0p5()
                                            .rounded_sm()
                                            .bg(bg)
                                            .border_1()
                                            .border_color(border_color)
                                            .hover(|s| s.bg(rgb(0x3a3a3e)))
                                            .text_xs()
                                            .text_color(txt)
                                            .child(label)
                                            .on_click(cx.listener(move |this, _, _window, cx| {
                                                this.state.set_log_filter_subsystem(sub.clone());
                                                cx.notify();
                                            }))
                                    }))
                            ),
                    )
                    // Log Entries Table Header
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .px_5()
                            .py_1p5()
                            .border_b_1()
                            .border_color(border_color)
                            .bg(surface_tile_2)
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(text_muted)
                            .child(div().w(px(90.0)).child("Timestamp"))
                            .child(div().w_20().child("Level"))
                            .child(div().w(px(140.0)).child("Subsystem"))
                            .child(div().flex_1().child("Message")),
                    )
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
                                vec![
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .py_16()
                                        .text_xs()
                                        .text_color(text_muted)
                                        .child("No log entries matching criteria.")
                                        .into_any_element(),
                                ]
                            } else {
                                filtered_logs
                                    .iter()
                                    .rev()
                                    .take(200)
                                    .map(|entry| {
                                        let badge_color = match entry.level {
                                            LogLevel::TRACE => rgb(0x7a7a7a),
                                            LogLevel::DEBUG => rgb(0x55555a),
                                            LogLevel::INFO => rgb(0x0066cc),
                                            LogLevel::WARN => rgb(0xf5a623),
                                            LogLevel::ERROR => rgb(0xe74c3c),
                                        };
                                        let time_str = entry.timestamp.format("%H:%M:%S%.3f").to_string();

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
                                                div()
                                                    .w_20()
                                                    .flex()
                                                    .items_center()
                                                    .child(
                                                        div()
                                                            .px_1p5()
                                                            .py_0p5()
                                                            .rounded_sm()
                                                            .bg(badge_color)
                                                            .text_xs()
                                                            .text_color(rgb(0xffffff))
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
                                                    .child(entry.message.clone()),
                                            )
                                            .into_any_element()
                                    })
                                    .collect()
                            }),
                    )
                    // Modal Footer
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_5()
                            .py_2()
                            .border_t_1()
                            .border_color(border_color)
                            .bg(surface_tile_2)
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
                            .child(div().child("Esc or Ctrl+Shift+L to close • Zero Telemetry")),
                    ),
            )
    }
}
