//! Modular JMeter-style Load Testing & Collection Runner UI modal.

use super::AuraWorkspaceView;
use crate::app_state::LoadThinkMode;
use crate::input_registry::InputKey;
use crate::theme::{icon, subtle_surface};
use aura_engine::LoadProfileMode;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::base::Disableable as _;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::dialog::{Dialog, DialogFooter};
use gpui_kit::component::input::Input;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};

impl AuraWorkspaceView {
    pub(super) fn render_load_tester_modal(&self, el: Div, cx: &mut Context<Self>) -> Div {
        if !self.state.is_load_tester_modal_open {
            return el;
        }

        let border = cx.theme().border;
        let muted = cx.theme().muted_foreground;
        let fg = cx.theme().foreground;
        let radius = cx.theme().radius;
        let surface = subtle_surface(cx);
        let running = self.state.load_is_running;

        el.child(
            Dialog::new(cx)
                .w(px(900.0))
                .title("Load Tester & Collection Runner (JMeter Profiles)")
                .on_close({
                    let view = cx.entity().clone();
                    move |_, _, app| {
                        let _ = view.update(app, |this, cx| {
                            this.state.is_load_tester_modal_open = false;
                            cx.notify();
                        });
                    }
                })
                .child(
                    v_flex()
                        .gap_2()
                        .max_h(px(660.0))
                        .child(self.render_load_target_and_modes(border, muted, fg, radius, surface, running, cx))
                        .child(self.render_load_profile_fields(border, muted, fg, radius, surface, running, cx))
                        .child(self.render_load_throughput_bar(border, muted, fg, radius, surface, cx))
                        .child(self.render_load_timers_and_controllers(border, muted, fg, radius, surface, running, cx))
                        .child(self.render_load_data_and_sla_bar(border, muted, fg, radius, surface, running, cx))
                        .when_some(self.state.load_error.clone(), |d, err| {
                            d.child(div().text_xs().text_color(cx.theme().red).child(err))
                        })
                        .child(self.render_load_results_section(border, muted, fg, radius, cx)),
                )
                .footer(
                    DialogFooter::new()
                        .child(if running {
                            Button::new("load-stop-btn")
                                .icon(icon(IconName::Square))
                                .label("Stop Load Test")
                                .danger()
                                .small()
                                .rounded(ButtonRounded::Medium)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.stop_load_test(cx);
                                }))
                        } else {
                            Button::new("load-start-btn")
                                .icon(icon(IconName::Play))
                                .label("Start Load Test")
                                .success()
                                .small()
                                .rounded(ButtonRounded::Medium)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.spawn_start_load_test(cx);
                                }))
                        })
                        .child(
                            Button::new("load-close-btn")
                                .label("Close")
                                .outline()
                                .small()
                                .rounded(ButtonRounded::Medium)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.state.is_load_tester_modal_open = false;
                                    cx.notify();
                                })),
                        ),
                ),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_load_target_and_modes(
        &self,
        border: Hsla,
        muted: Hsla,
        fg: Hsla,
        radius: Pixels,
        surface: Hsla,
        running: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let mode = self.state.load_profile_mode;
        let use_col = self.state.load_use_collection;
        let col_req_count: usize = self.state.collections.iter().map(|c| c.requests.len()).sum();

        v_flex()
            .gap_2()
            .p_2()
            .rounded(radius)
            .bg(surface)
            .border_1()
            .border_color(border)
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(div().text_xs().text_color(muted).child("Target Scope:"))
                            .child({
                                let b = Button::new("load-scope-active")
                                    .label("Active Request")
                                    .xsmall()
                                    .rounded(ButtonRounded::Small)
                                    .disabled(running)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.state.load_use_collection = false;
                                        cx.notify();
                                    }));
                                if !use_col { b.primary() } else { b.outline() }
                            })
                            .child({
                                let b = Button::new("load-scope-collection")
                                    .label(format!("Full Collection ({col_req_count})"))
                                    .xsmall()
                                    .rounded(ButtonRounded::Small)
                                    .disabled(running || col_req_count == 0)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.state.load_use_collection = true;
                                        cx.notify();
                                    }));
                                if use_col { b.primary() } else { b.outline() }
                            }),
                    )
                    .child(
                        Tag::secondary().small().child(if running {
                            format!("Running ({} samples)...", self.state.load_live_records.len())
                        } else {
                            "Idle".to_string()
                        }),
                    ),
            )
            .child(
                h_flex()
                    .gap_1p5()
                    .flex_wrap()
                    .items_center()
                    .child(div().text_xs().text_color(fg).child("Thread Group:"))
                    .children(LoadProfileMode::ALL.into_iter().map(|m| {
                        let active = mode == m;
                        let b = Button::new(ElementId::Name(format!("load-mode-{}", m.label()).into()))
                            .label(m.label())
                            .xsmall()
                            .rounded(ButtonRounded::Small)
                            .disabled(running)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.state.load_profile_mode = m;
                                cx.notify();
                            }));
                        if active { b.primary() } else { b.outline() }
                    })),
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_load_profile_fields(
        &self,
        border: Hsla,
        muted: Hsla,
        fg: Hsla,
        radius: Pixels,
        surface: Hsla,
        running: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let box_el = v_flex()
            .gap_2()
            .p_2p5()
            .rounded(radius)
            .bg(surface)
            .border_1()
            .border_color(border);

        match self.state.load_profile_mode {
            LoadProfileMode::FixedIterations => box_el.child(
                h_flex()
                    .gap_2()
                    .child(self.render_labeled_input("Total Iterations", &InputKey::LoadIterations, muted, fg))
                    .child(self.render_labeled_input("Virtual Users (VUs)", &InputKey::LoadConcurrency, muted, fg))
                    .child(self.render_labeled_input("Ramp-Up (secs)", &InputKey::LoadRampUpSecs, muted, fg))
                    .child(self.render_labeled_input("Delay / Req (ms)", &InputKey::LoadDelayMs, muted, fg)),
            ),
            LoadProfileMode::Duration => box_el.child(
                h_flex()
                    .gap_2()
                    .child(self.render_labeled_input("Virtual Users (VUs)", &InputKey::LoadConcurrency, muted, fg))
                    .child(self.render_labeled_input("Duration (secs)", &InputKey::LoadDurationSecs, muted, fg))
                    .child(self.render_labeled_input("Delay / Req (ms)", &InputKey::LoadDelayMs, muted, fg)),
            ),
            LoadProfileMode::RampUpDown => box_el.child(
                h_flex()
                    .gap_2()
                    .child(self.render_labeled_input("Target VUs", &InputKey::LoadConcurrency, muted, fg))
                    .child(self.render_labeled_input("Ramp-Up (secs)", &InputKey::LoadRampUpSecs, muted, fg))
                    .child(self.render_labeled_input("Total Duration (s)", &InputKey::LoadDurationSecs, muted, fg))
                    .child(self.render_labeled_input("Ramp-Down (secs)", &InputKey::LoadRampDownSecs, muted, fg)),
            ),
            LoadProfileMode::SteppedStages => box_el
                .child(
                    h_flex()
                        .justify_between()
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted)
                                .child("Multi-Stage Spike Schedule (executed sequentially):"),
                        )
                        .child(
                            h_flex()
                                .gap_1p5()
                                .child(
                                    Button::new("load-spike-preset-btn")
                                        .label("Spike Preset (5 -> 25 -> 5)")
                                        .xsmall()
                                        .outline()
                                        .disabled(running)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.state.load_stages = vec![
                                                ("5".into(), "5".into()),
                                                ("25".into(), "10".into()),
                                                ("5".into(), "5".into()),
                                            ];
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("load-add-stage-btn")
                                        .icon(icon(IconName::Plus))
                                        .label("Add Stage")
                                        .xsmall()
                                        .secondary()
                                        .disabled(running)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.state.load_stages.push(("10".into(), "10".into()));
                                            cx.notify();
                                        })),
                                ),
                        ),
                )
                .children(self.state.load_stages.iter().enumerate().map(|(idx, _)| {
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(div().w(px(64.0)).text_xs().text_color(muted).child(format!("Stage {}:", idx + 1)))
                        .child(div().w(px(110.0)).when_some(
                            self.inputs.get_input(&InputKey::LoadStageVus(idx)),
                            |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                        ))
                        .child(div().text_xs().text_color(muted).child("VUs for"))
                        .child(div().w(px(110.0)).when_some(
                            self.inputs.get_input(&InputKey::LoadStageDurationSecs(idx)),
                            |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                        ))
                        .child(div().text_xs().text_color(muted).child("secs"))
                        .child(
                            Button::new(ElementId::NamedInteger("load-del-stage".into(), idx as u64))
                                .icon(icon(IconName::X))
                                .xsmall()
                                .ghost()
                                .disabled(running || self.state.load_stages.len() <= 1)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if this.state.load_stages.len() > 1 && idx < this.state.load_stages.len() {
                                        this.state.load_stages.remove(idx);
                                        cx.notify();
                                    }
                                })),
                        )
                })),
        }
    }

    fn render_labeled_input(
        &self,
        label: &'static str,
        key: &InputKey,
        muted: Hsla,
        _fg: Hsla,
    ) -> Div {
        v_flex()
            .flex_1()
            .gap_1()
            .child(div().text_xs().text_color(muted).child(label))
            .when_some(self.inputs.get_input(key), |d, inp| {
                d.child(Input::new(inp).xsmall().w_full())
            })
    }

    fn render_load_throughput_bar(
        &self,
        border: Hsla,
        muted: Hsla,
        fg: Hsla,
        radius: Pixels,
        surface: Hsla,
        cx: &mut Context<Self>,
    ) -> Div {
        let enabled = self.state.load_target_rps_enabled;
        let view = cx.entity().clone();

        h_flex()
            .justify_between()
            .gap_3()
            .p_2()
            .rounded(radius)
            .bg(surface)
            .border_1()
            .border_color(border)
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Switch::new("load-target-rps-sw")
                            .checked(enabled)
                            .small()
                            .on_change(move |_, _, app| {
                                let _ = view.update(app, |this, cx| {
                                    this.state.load_target_rps_enabled = !this.state.load_target_rps_enabled;
                                    cx.notify();
                                });
                            }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(fg)
                            .child("Target Throughput Shaping (Constant Throughput Timer)"),
                    ),
            )
            .child(
                h_flex()
                    .gap_1p5()
                    .items_center()
                    .child(div().w(px(100.0)).when_some(
                        self.inputs.get_input(&InputKey::LoadTargetRps),
                        |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                    ))
                    .child(div().text_xs().text_color(muted).child("req / sec")),
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_load_timers_and_controllers(
        &self,
        border: Hsla,
        muted: Hsla,
        fg: Hsla,
        radius: Pixels,
        surface: Hsla,
        running: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let think_mode = self.state.load_think_mode;
        let barrier_on = self.state.load_barrier_enabled;
        let weighted_on = self.state.load_weighted_routing;
        let active_flow = self.state.active_request().and_then(|r| r.flow.as_ref());
        let once_on = active_flow.map(|f| f.once_per_run).unwrap_or(false);

        let view_barrier = cx.entity().clone();
        let view_weighted = cx.entity().clone();
        let view_once = cx.entity().clone();

        v_flex()
            .gap_2()
            .p_2()
            .rounded(radius)
            .bg(surface)
            .border_1()
            .border_color(border)
            // Think-Time + Synchronizing Barrier row
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(div().text_xs().text_color(fg).child("Think-Time:"))
                            .children(LoadThinkMode::ALL.into_iter().map(|tm| {
                                let active = think_mode == tm;
                                let b = Button::new(ElementId::Name(format!("think-{}", tm.label()).into()))
                                    .label(tm.label())
                                    .xsmall()
                                    .rounded(ButtonRounded::Small)
                                    .disabled(running)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.state.load_think_mode = tm;
                                        cx.notify();
                                    }));
                                if active { b.primary() } else { b.outline() }
                            }))
                            .when(think_mode != LoadThinkMode::None, |el| {
                                el.child(div().w(px(72.0)).when_some(
                                    self.inputs.get_input(&InputKey::LoadThinkA),
                                    |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                ))
                                .child(div().text_xs().text_color(muted).child(
                                    if think_mode == LoadThinkMode::Uniform { ".." } else { "±" },
                                ))
                                .child(div().w(px(72.0)).when_some(
                                    self.inputs.get_input(&InputKey::LoadThinkB),
                                    |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                ))
                                .child(div().text_xs().text_color(muted).child("ms"))
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(
                                Switch::new("load-barrier-sw")
                                    .checked(barrier_on)
                                    .small()
                                    .on_change(move |_, _, app| {
                                        let _ = view_barrier.update(app, |this, cx| {
                                            this.state.load_barrier_enabled = !this.state.load_barrier_enabled;
                                            cx.notify();
                                        });
                                    }),
                            )
                            .child(div().text_xs().text_color(fg).child("Sync Barrier VUs:"))
                            .child(div().w(px(62.0)).when_some(
                                self.inputs.get_input(&InputKey::LoadBarrierGroupSize),
                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                            ))
                            .child(div().text_xs().text_color(muted).child("Timeout ms:"))
                            .child(div().w(px(72.0)).when_some(
                                self.inputs.get_input(&InputKey::LoadBarrierTimeoutMs),
                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                            )),
                    ),
            )
            // Logic Controllers row (Weighted Distribution + Active Request Once-Only / Loop / If / While)
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(
                                Switch::new("load-weighted-sw")
                                    .checked(weighted_on)
                                    .small()
                                    .on_change(move |_, _, app| {
                                        let _ = view_weighted.update(app, |this, cx| {
                                            this.state.load_weighted_routing = !this.state.load_weighted_routing;
                                            cx.notify();
                                        });
                                    }),
                            )
                            .child(div().text_xs().text_color(fg).child("Weighted Distribution"))
                            .child(
                                Switch::new("flow-once-sw")
                                    .checked(once_on)
                                    .small()
                                    .on_change(move |_, _, app| {
                                        let _ = view_once.update(app, |this, cx| {
                                            this.state.toggle_active_flow_once_per_run();
                                            this.schedule_auto_save(cx);
                                            cx.notify();
                                        });
                                    }),
                            )
                            .child(div().text_xs().text_color(fg).child("Once-Only"))
                            .child(div().text_xs().text_color(muted).child("Wt %:"))
                            .child(div().w(px(56.0)).when_some(
                                self.inputs.get_input(&InputKey::FlowWeightPercent),
                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                            ))
                            .child(div().text_xs().text_color(muted).child("Loop:"))
                            .child(div().w(px(56.0)).when_some(
                                self.inputs.get_input(&InputKey::FlowLoopCount),
                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                            )),
                    )
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(div().text_xs().text_color(muted).child("If:"))
                            .child(div().w(px(140.0)).when_some(
                                self.inputs.get_input(&InputKey::FlowConditionIf),
                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                            ))
                            .child(div().text_xs().text_color(muted).child("While:"))
                            .child(div().w(px(140.0)).when_some(
                                self.inputs.get_input(&InputKey::FlowLoopWhile),
                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                            )),
                    ),
            )
    }
}
