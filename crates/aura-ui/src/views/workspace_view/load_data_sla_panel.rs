//! Modular UI section for CSV/JSON Data Feeder, Per-Sample Performance Bounds, and Global SLA Gates.

use super::AuraWorkspaceView;
use crate::input_registry::InputKey;
use crate::theme::icon;
use aura_engine::{DataEofBehavior, DataSharingMode};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::base::Disableable as _;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::input::Input;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{Sizable, h_flex, v_flex};

impl AuraWorkspaceView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_load_data_and_sla_bar(
        &self,
        border: Hsla,
        muted: Hsla,
        fg: Hsla,
        radius: Pixels,
        surface: Hsla,
        running: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let data_on = self.state.load_data_enabled;
        let sharing = self.state.load_data_sharing;
        let eof = self.state.load_data_eof;
        let view_data = cx.entity().clone();

        v_flex()
            .gap_2()
            .p_2()
            .rounded(radius)
            .bg(surface)
            .border_1()
            .border_color(border)
            // CSV / JSON Data Feeder Row
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .flex_1()
                            .child(
                                Switch::new("load-data-feeder-sw")
                                    .checked(data_on)
                                    .small()
                                    .on_change(move |_, _, app| {
                                        let _ = view_data.update(app, |this, cx| {
                                            this.state.load_data_enabled = !this.state.load_data_enabled;
                                            cx.notify();
                                        });
                                    }),
                            )
                            .child(div().text_xs().text_color(fg).child("Data Feeder:"))
                            .child(div().flex_1().min_w(px(160.0)).when_some(
                                self.inputs.get_input(&InputKey::LoadDataFilePath),
                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                            ))
                            .child(
                                Button::new("load-data-browse-btn")
                                    .icon(icon(IconName::FolderOpen))
                                    .label("Browse")
                                    .xsmall()
                                    .outline()
                                    .rounded(ButtonRounded::Small)
                                    .disabled(running)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.spawn_pick_load_data_file(cx);
                                    })),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child({
                                let is_all = sharing == DataSharingMode::AllVUs;
                                let b = Button::new("load-data-share-btn")
                                    .label(sharing.label())
                                    .xsmall()
                                    .rounded(ButtonRounded::Small)
                                    .disabled(running)
                                    .tooltip("Toggle row sharing across All VUs vs Per VU")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.state.load_data_sharing = match this.state.load_data_sharing {
                                            DataSharingMode::AllVUs => DataSharingMode::PerVU,
                                            DataSharingMode::PerVU => DataSharingMode::AllVUs,
                                        };
                                        cx.notify();
                                    }));
                                if is_all { b.outline() } else { b.secondary() }
                            })
                            .child({
                                let is_recycle = eof == DataEofBehavior::Recycle;
                                let b = Button::new("load-data-eof-btn")
                                    .label(eof.label())
                                    .xsmall()
                                    .rounded(ButtonRounded::Small)
                                    .disabled(running)
                                    .tooltip("Toggle EOF behavior: Recycle from Start vs Stop VU")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.state.load_data_eof = match this.state.load_data_eof {
                                            DataEofBehavior::Recycle => DataEofBehavior::StopVuOnEof,
                                            DataEofBehavior::StopVuOnEof => DataEofBehavior::Recycle,
                                        };
                                        cx.notify();
                                    }));
                                if is_recycle { b.outline() } else { b.secondary() }
                            }),
                    ),
            )
            // Per-Sample Performance Assertions + Global CI SLA Thresholds Row
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(div().text_xs().text_color(fg).child("Sample Bounds:"))
                            .child(div().w(px(74.0)).when_some(
                                self.inputs.get_input(&InputKey::LoadMaxDurationMs),
                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                            ))
                            .child(div().text_xs().text_color(muted).child("ms"))
                            .child(div().w(px(64.0)).when_some(
                                self.inputs.get_input(&InputKey::LoadMinSizeBytes),
                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                            ))
                            .child(div().text_xs().text_color(muted).child(".."))
                            .child(div().w(px(64.0)).when_some(
                                self.inputs.get_input(&InputKey::LoadMaxSizeBytes),
                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                            ))
                            .child(div().text_xs().text_color(muted).child("bytes")),
                    )
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(div().text_xs().text_color(fg).child("SLA Gates:"))
                            .child(div().w(px(68.0)).when_some(
                                self.inputs.get_input(&InputKey::LoadSlaP95Ms),
                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                            ))
                            .child(div().w(px(68.0)).when_some(
                                self.inputs.get_input(&InputKey::LoadSlaP99Ms),
                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                            ))
                            .child(div().w(px(64.0)).when_some(
                                self.inputs.get_input(&InputKey::LoadSlaErrorPct),
                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                            )),
                    ),
            )
    }
}
