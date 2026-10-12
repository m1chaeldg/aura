//! CommandPalette View: Keyboard-driven search modal overlay.

use crate::app_state::AuraAppState;
use aura_core::model::HttpMethod;
use gpui::*;

pub struct CommandPaletteView;

impl CommandPaletteView {
    pub fn render(state: &AuraAppState) -> impl IntoElement {
        let accent_blue = rgb(0x0066cc);
        let surface_color = rgb(0x272729);
        let border_color = rgb(0x333333);
        let text_color = rgb(0xf5f5f7);
        let text_muted = rgb(0x7a7a7a);

        let filtered = state.filtered_palette_items();
        let sel_idx = state.selected_palette_index;

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_start()
            .justify_center()
            .pt_16()
            .bg(rgb(0x0a0a0c))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w_96()
                    .rounded_lg()
                    .bg(surface_color)
                    .border_1()
                    .border_color(accent_blue)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .px_4()
                            .py_3()
                            .border_b_1()
                            .border_color(border_color)
                            .gap_2()
                            .child(div().text_sm().text_color(accent_blue).child("🔍"))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(if state.command_palette_search.is_empty() {
                                        text_muted
                                    } else {
                                        text_color
                                    })
                                    .child(if state.command_palette_search.is_empty() {
                                        "Search requests or type command... (Esc to dismiss)".to_string()
                                    } else {
                                        state.command_palette_search.clone()
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .py_2()
                            .children(filtered.iter().enumerate().map(|(idx, item)| {
                                let is_sel = idx == sel_idx;
                                let row_bg = if is_sel { rgb(0x333338) } else { surface_color };
                                let item_method = item.method.unwrap_or(HttpMethod::GET);
                                let badge_color = match item_method {
                                    HttpMethod::GET => rgb(0x0066cc),
                                    HttpMethod::POST => rgb(0x28a745),
                                    HttpMethod::PUT => rgb(0xf5a623),
                                    HttpMethod::PATCH => rgb(0x8e44ad),
                                    HttpMethod::DELETE => rgb(0xe74c3c),
                                    _ => rgb(0x7a7a7a),
                                };

                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px_4()
                                    .py_2()
                                    .bg(row_bg)
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .px_1()
                                                    .rounded_sm()
                                                    .bg(rgb(0x1d1d1f))
                                                    .text_xs()
                                                    .text_color(badge_color)
                                                    .child(item_method.as_str()),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(text_color)
                                                    .child(item.title.clone()),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(text_muted)
                                            .child(item.subtitle.clone()),
                                    )
                            })),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_4()
                            .py_2()
                            .border_t_1()
                            .border_color(border_color)
                            .text_xs()
                            .text_color(text_muted)
                            .child("↑ ↓ to navigate • ↵ to select • Esc to dismiss"),
                    ),
            )
    }
}
