use super::AuraWorkspaceView;
use crate::theme::icon;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::separator::Separator;
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::component::{Sizable, h_flex};

impl AuraWorkspaceView {
    pub(super) fn render_status_bar(
        &self,
        is_horizontal: bool,
        split_pct: u32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let git_status_label = match &self.state.git_snapshot {
            Some(s) if s.is_repo => {
                let count = s.total_changes();
                if count > 0 {
                    format!("Git: {} ({count} changed)", s.branch)
                } else {
                    format!("Git: {} (clean)", s.branch)
                }
            }
            _ => "Git: Not initialized".to_string(),
        };

        let mut bar = StatusBar::new()
            .left(
                self.state
                    .status_message
                    .clone()
                    .unwrap_or_else(|| "Ready".to_string()),
            )
            .left(Separator::vertical().h(px(12.)))
            .left(git_status_label)
            .left(Separator::vertical().h(px(12.)))
            .left("Workspace: Local Disk Synced")
            .left(Separator::vertical().h(px(12.)))
            .left("deno_core: Offline");

        if self.state.load_is_running {
            let samples = self.state.load_live_records.len();
            bar = bar
                .left(Separator::vertical().h(px(12.)))
                .left(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .child(
                            Button::new("status-load-reopen-btn")
                                .icon(icon(IconName::Zap))
                                .label(format!("Load: {samples} samples"))
                                .xsmall()
                                .primary()
                                .rounded(ButtonRounded::Small)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.state.load_modal_tab =
                                        crate::app_state::LoadModalTab::ResultsAndCharts;
                                    this.open_load_tester_modal(cx);
                                })),
                        )
                        .child(
                            Button::new("status-load-stop-btn")
                                .icon(icon(IconName::Square))
                                .label("Stop")
                                .xsmall()
                                .danger()
                                .rounded(ButtonRounded::Small)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.stop_load_test(cx);
                                })),
                        ),
                );
        }

        bar.right(format!(
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
