use super::AuraWorkspaceView;
use gpui::*;
use gpui_kit::component::separator::Separator;
use gpui_kit::component::status_bar::StatusBar;

impl AuraWorkspaceView {
    pub(super) fn render_status_bar(
        &self,
        is_horizontal: bool,
        split_pct: u32,
    ) -> impl IntoElement {
        StatusBar::new()
            .left(
                self.state
                    .status_message
                    .clone()
                    .unwrap_or_else(|| "Ready".to_string()),
            )
            .left(Separator::vertical().h(px(12.)))
            .left("Workspace: Local Disk Synced")
            .left(Separator::vertical().h(px(12.)))
            .left("deno_core: Offline")
            .right(format!(
                "Layout: {} ({}%) [Reset: Ctrl+R]",
                if is_horizontal {
                    "Side-by-Side"
                } else {
                    "Stacked"
                },
                split_pct
            ))
            .right(Separator::vertical().h(px(12.)))
            .right(format!("Keymap: {}", self.state.keybinding_preset.name()))
            .right(Separator::vertical().h(px(12.)))
            .right("Aura v0.1.0")
    }
}
