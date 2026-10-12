//! GPUI Desktop Workspace View implementation strictly adhering to DESIGN.md.

use crate::model::HttpMethod;
use crate::ui::{AuraAppState, LayoutOrientation};
use gpui::prelude::FluentBuilder;
use gpui::*;

pub struct AuraWorkspaceView {
    pub state: AuraAppState,
}

impl AuraWorkspaceView {
    pub fn new() -> Self {
        let mut state = AuraAppState::new();
        state.new_request();
        Self { state }
    }
}

impl Default for AuraWorkspaceView {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for AuraWorkspaceView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let active_tab = self.state.tabs.get(self.state.active_tab_index);
        let active_title = active_tab
            .map(|t| t.title.clone())
            .unwrap_or_else(|| "Untitled Request".to_string());
        let active_method = active_tab.map(|t| t.method).unwrap_or(HttpMethod::GET);
        let active_url = active_tab
            .map(|t| t.request.url.clone())
            .unwrap_or_else(|| "https://postman-echo.com/get".to_string());

        // Brand colors adhering strictly to DESIGN.md tokens
        let bg_color = rgb(0x1d1d1f); // #1d1d1f Apple Near-Black
        let surface_color = rgb(0x272729); // #272729 Surface Tile 1
        let surface_tile_2 = rgb(0x2a2a2c); // #2a2a2c Surface Tile 2
        let text_color = rgb(0xf5f5f7); // #f5f5f7 Canvas Parchment
        let text_muted = rgb(0x7a7a7a); // #7a7a7a Muted
        let border_color = rgb(0x333333); // #333333 Hairline border
        let accent_blue = rgb(0x0066cc); // #0066cc Action Blue
        let green_color = rgb(0x28a745); // #28a745 Method Post / Send Success
        let amber_color = rgb(0xf5a623); // #f5a623 Amber for dirty dot / PUT
        let divider_hover_color = rgb(0x0071e3); // #0071e3 Focus Blue

        let method_color = match active_method {
            HttpMethod::GET => rgb(0x0066cc),
            HttpMethod::POST => rgb(0x28a745),
            HttpMethod::PUT => rgb(0xf5a623),
            HttpMethod::PATCH => rgb(0x8e44ad),
            HttpMethod::DELETE => rgb(0xe74c3c),
            _ => rgb(0x7a7a7a),
        };

        let is_horizontal = self.state.orientation == LayoutOrientation::Horizontal;
        let split_pct = (self.state.split_ratio * 100.0).round() as u32;

        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(bg_color)
            .text_color(text_color)
            // Top Navigation Bar / Title Bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_2()
                    .bg(surface_color)
                    .border_b_1()
                    .border_color(border_color)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w_6()
                                    .h_6()
                                    .rounded_md()
                                    .bg(accent_blue)
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(0xffffff))
                                            .child("A"),
                                    ),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .child("Aura REST API Client v0.1.0"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_muted)
                                    .child("(GPUI + deno_core • Hot Reload Active)"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            // Command Palette Quick Action button
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .bg(surface_tile_2)
                                    .border_1()
                                    .border_color(border_color)
                                    .text_xs()
                                    .text_color(text_color)
                                    .child("⌘ Command Palette")
                                    .child(
                                        div()
                                            .px_1()
                                            .rounded_sm()
                                            .bg(rgb(0x333333))
                                            .text_xs()
                                            .text_color(rgb(0xaaaaaa))
                                            .child(self.state.keybinding_preset.primary_shortcut(crate::ui::keybindings::Action::OpenCommandPalette)),
                                    ),
                            )
                            // Layout orientation toggle
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .bg(surface_tile_2)
                                    .border_1()
                                    .border_color(border_color)
                                    .text_xs()
                                    .text_color(text_color)
                                    .child(if is_horizontal { "◫ Side-by-Side" } else { "⬒ Stacked" }),
                            )
                            // Privacy & Sandbox Badge
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .bg(rgb(0x333333))
                                    .text_xs()
                                    .text_color(rgb(0xcccccc))
                                    .child("Zero Telemetry • Offline Sandboxed"),
                            ),
                    ),
            )
            // Multi-Tab Document Workspace Bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .px_3()
                    .py_1()
                    .bg(rgb(0x222224))
                    .border_b_1()
                    .border_color(border_color)
                    .gap_1()
                    .children(self.state.tabs.iter().enumerate().map(|(idx, tab)| {
                        let is_active = idx == self.state.active_tab_index;
                        let tab_bg = if is_active {
                            rgb(0x272729)
                        } else {
                            rgb(0x1d1d1f)
                        };
                        let tab_border = if is_active {
                            accent_blue
                        } else {
                            border_color
                        };

                        let method_color = match tab.method {
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
                            .gap_2()
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(tab_bg)
                            .border_1()
                            .border_color(tab_border)
                            // HTTP Method Badge
                            .child(
                                div()
                                    .px_1()
                                    .rounded_sm()
                                    .bg(rgb(0x1b1b1d))
                                    .text_xs()
                                    .text_color(method_color)
                                    .child(tab.method.as_str()),
                            )
                            // Tab Title
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(if is_active {
                                        rgb(0xffffff)
                                    } else {
                                        rgb(0xaaaaaa)
                                    })
                                    .child(tab.title.clone()),
                            )
                            // Dirty State Tracking: Visual Dot Indicator
                            .when(tab.is_dirty, |el| {
                                el.child(
                                    div()
                                        .w_2()
                                        .h_2()
                                        .rounded_full()
                                        .bg(amber_color),
                                )
                            })
                            // Dedicated Close Button
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w_4()
                                    .h_4()
                                    .rounded_full()
                                    .text_xs()
                                    .text_color(text_muted)
                                    .child("×"),
                            )
                    }))
                    // Persistent `+` New Request Quick Action
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w_6()
                            .h_6()
                            .rounded_md()
                            .bg(rgb(0x333333))
                            .text_xs()
                            .text_color(rgb(0xffffff))
                            .child("+"),
                    ),
            )
            // Main Request URL Bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_3()
                    .bg(surface_color)
                    .border_b_1()
                    .border_color(border_color)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(method_color)
                            .text_xs()
                            .text_color(rgb(0xffffff))
                            .child(active_method.as_str()),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .items_center()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(rgb(0x1d1d1f))
                            .border_1()
                            .border_color(border_color)
                            .text_sm()
                            .text_color(rgb(0xffffff))
                            .child(active_url),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .px_5()
                            .py_2()
                            .rounded_md()
                            .bg(accent_blue)
                            .text_sm()
                            .text_color(rgb(0xffffff))
                            .child("Send ↵"),
                    ),
            )
            // Responsive Pane Layout: Adjustable Dividers & Orientation Toggle
            .child(
                div()
                    .flex()
                    .flex_1()
                    .when(is_horizontal, |el| el.flex_row())
                    .when(!is_horizontal, |el| el.flex_col())
                    // Request Pane
                    .child(
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
                                    .border_color(border_color)
                                    .py_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(accent_blue)
                                            .child("REQUEST PANE (Dual-Mode: Visual / TOML)"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(text_muted)
                                            .child("Params • Headers • Body • Scripting"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .py_3()
                                    .text_sm()
                                    .text_color(rgb(0xcccccc))
                                    .child(format!(
                                        "Editing: {} [{}]",
                                        active_title,
                                        active_method.as_str()
                                    )),
                            ),
                    )
                    // Smooth Drag-to-Resize Divider with Double-Click Reset
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(is_horizontal, |el| {
                                el.w_2()
                                    .border_l_1()
                                    .border_r_1()
                                    .border_color(border_color)
                                    .bg(surface_color)
                            })
                            .when(!is_horizontal, |el| {
                                el.h_2()
                                    .border_t_1()
                                    .border_b_1()
                                    .border_color(border_color)
                                    .bg(surface_color)
                            })
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(divider_hover_color)
                                    .child(if is_horizontal { "⋮" } else { "···" }),
                            ),
                    )
                    // Response Pane
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .p_4()
                            .bg(rgb(0x1b1b1d))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .border_b_1()
                                    .border_color(border_color)
                                    .py_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(green_color)
                                            .child("RESPONSE PANE"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(text_muted)
                                            .child("Status: 200 OK • Time: --ms • Size: --B"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .py_3()
                                    .text_xs()
                                    .text_color(text_muted)
                                    .child("Ready to dispatch request to live Postman Echo endpoints."),
                            ),
                    ),
            )
            // Bottom Status Bar with Ergonomics & Keymap Preset Info
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_1()
                    .bg(surface_color)
                    .border_t_1()
                    .border_color(border_color)
                    .text_xs()
                    .text_color(text_muted)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                self.state
                                    .status_message
                                    .clone()
                                    .unwrap_or_else(|| "Ready".to_string()),
                            )
                            .child("•")
                            .child("Workspace: Local Disk Synced")
                            .child("•")
                            .child("deno_core: Offline"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(format!(
                                "Layout: {} ({}%) [Reset: Ctrl+R]",
                                if is_horizontal { "Side-by-Side" } else { "Stacked" },
                                split_pct
                            ))
                            .child("•")
                            .child(format!(
                                "Keymap: {}",
                                self.state.keybinding_preset.name()
                            ))
                            .child("•")
                            .child("Aura v0.1.0"),
                    ),
            )
            // Command Palette Modal Overlay
            .when(self.state.is_command_palette_open, |el| {
                let filtered = self.state.filtered_palette_items();
                let sel_idx = self.state.selected_palette_index;

                el.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_start()
                        .justify_center()
                        .pt_16()
                        .bg(rgb(0x0a0a0c)) // Dark modal backdrop
                        // Modal Card
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
                                    // Search Input Header
                                    div()
                                        .flex()
                                        .items_center()
                                        .px_4()
                                        .py_3()
                                        .border_b_1()
                                        .border_color(border_color)
                                        .gap_2()
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(accent_blue)
                                                .child("🔍"),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(if self.state.command_palette_search.is_empty() {
                                                    text_muted
                                                } else {
                                                    text_color
                                                })
                                                .child(if self.state.command_palette_search.is_empty() {
                                                    "Search requests or type command... (Esc to dismiss)".to_string()
                                                } else {
                                                    self.state.command_palette_search.clone()
                                                }),
                                        ),
                                )
                                // Results List
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .py_2()
                                        .children(filtered.iter().enumerate().map(|(idx, item)| {
                                            let is_sel = idx == sel_idx;
                                            let row_bg = if is_sel {
                                                rgb(0x333338)
                                            } else {
                                                surface_color
                                            };
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
                                // Footer navigation hints
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
                        ),
                )
            })
    }
}
