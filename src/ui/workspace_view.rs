//! GPUI Desktop Workspace View implementation strictly adhering to DESIGN.md.

use crate::model::HttpMethod;
use crate::ui::AuraAppState;
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

        // Brand colors (Hex to rgb helper)
        let bg_color = rgb(0x1d1d1f); // #1d1d1f Apple Near-Black
        let surface_color = rgb(0x272729); // #272729 Surface Tile 1
        let text_color = rgb(0xf5f5f7); // #f5f5f7 Canvas Parchment
        let text_muted = rgb(0x7a7a7a); // #7a7a7a Muted
        let border_color = rgb(0x333333); // #333333 Hairline border
        let accent_blue = rgb(0x0066cc); // #0066cc Action Blue
        let green_color = rgb(0x28a745); // #28a745 Method Post / Send Success

        let method_color = match active_method {
            HttpMethod::GET => rgb(0x0066cc),
            HttpMethod::POST => rgb(0x28a745),
            HttpMethod::PUT => rgb(0xf5a623),
            HttpMethod::PATCH => rgb(0x8e44ad),
            HttpMethod::DELETE => rgb(0xe74c3c),
            _ => rgb(0x7a7a7a),
        };

        div()
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
                    .py_3()
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
            // Tab Bar
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
                            rgb(0x0066cc)
                        } else {
                            rgb(0x333333)
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
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(match tab.method {
                                        HttpMethod::GET => rgb(0x0066cc),
                                        HttpMethod::POST => rgb(0x28a745),
                                        HttpMethod::PUT => rgb(0xf5a623),
                                        HttpMethod::PATCH => rgb(0x8e44ad),
                                        HttpMethod::DELETE => rgb(0xe74c3c),
                                        _ => rgb(0x7a7a7a),
                                    })
                                    .child(tab.method.as_str()),
                            )
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
                    }))
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
            // Split Panes: Request on left/top, Response on right/bottom
            .child(
                div()
                    .flex()
                    .flex_1()
                    .child(
                        // Request Pane
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .p_4()
                            .border_r_1()
                            .border_color(border_color)
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
                    .child(
                        // Response Pane
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
            // Bottom Status Bar
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
                            .child("Ready")
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
                            .child("Layout: 50/50 Split")
                            .child("•")
                            .child("Aura v0.1.0"),
                    ),
            )
    }
}
