//! Modular JMeter-style Load Testing & Collection Runner UI modal
//! organized into Setup & Scope, Requests & Flow, and Results & Charts tabs.

use super::AuraWorkspaceView;
use crate::app_state::{LoadModalTab, LoadThinkMode};
use crate::input_registry::InputKey;
use crate::theme::{icon, subtle_surface};
use aura_engine::LoadProfileMode;
use aura_sync::LoadTargetScope;
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
        let active_tab = self.state.load_modal_tab;

        el.child(
            Dialog::new(cx)
                .w(px(940.0))
                .title("Load Tester & Workspace Runner (JMeter Profiles)")
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
                        .child(self.render_load_header_and_tabs(border, muted, fg, radius, surface, running, cx))
                        .when_some(self.state.load_error.clone(), |d, err| {
                            d.child(div().text_xs().text_color(cx.theme().red).child(err))
                        })
                        .child(match active_tab {
                            LoadModalTab::SetupAndScope => v_flex()
                                .id("load-setup-scroll")
                                .gap_2()
                                .max_h(px(540.0))
                                .overflow_y_scroll()
                                .child(self.render_load_target_and_modes(border, muted, fg, radius, surface, running, cx))
                                .child(self.render_load_profile_fields(border, muted, fg, radius, surface, running, cx))
                                .child(self.render_load_throughput_bar(border, muted, fg, radius, surface, cx))
                                .child(self.render_load_timers_and_controllers(border, muted, fg, radius, surface, running, cx))
                                .child(self.render_load_data_and_sla_bar(border, muted, fg, radius, surface, running, cx))
                                .into_any_element(),
                            LoadModalTab::RequestsAndFlow => {
                                self.render_load_requests_panel(border, muted, fg, radius, running, cx)
                                    .into_any_element()
                            }
                            LoadModalTab::ResultsAndCharts => {
                                self.render_load_results_section(border, muted, fg, radius, cx)
                                    .into_any_element()
                            }
                        }),
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
    fn render_load_header_and_tabs(
        &self,
        border: Hsla,
        muted: Hsla,
        fg: Hsla,
        radius: Pixels,
        surface: Hsla,
        running: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let active_tab = self.state.load_modal_tab;
        let has_ws = self.state.workspace_store.workspace_root.is_some();

        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .children(LoadModalTab::ALL.into_iter().map(|tab| {
                                let active = active_tab == tab;
                                let label = if tab == LoadModalTab::RequestsAndFlow {
                                    let inc = self
                                        .state
                                        .load_discovered_requests
                                        .iter()
                                        .filter(|r| r.included)
                                        .count();
                                    format!("{} ({inc})", tab.label())
                                } else {
                                    tab.label().to_string()
                                };
                                let b = Button::new(ElementId::Name(format!("load-mtab-{}", tab.label()).into()))
                                    .label(label)
                                    .xsmall()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.state.load_modal_tab = tab;
                                        cx.notify();
                                    }));
                                if active { b.primary() } else { b.outline() }
                            })),
                    )
                    .child(
                        Tag::secondary().small().child(if running {
                            format!("Running ({} samples)...", self.state.load_live_records.len())
                        } else {
                            "Idle".to_string()
                        }),
                    ),
            )
            // Workspace Load Profile bar
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .p_2()
                    .rounded(radius)
                    .bg(surface)
                    .border_1()
                    .border_color(border)
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(div().text_xs().text_color(muted).child("Profile:"))
                            .child(div().w(px(180.0)).when_some(
                                self.inputs.get_input(&InputKey::LoadProfileName),
                                |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                            ))
                            .child(
                                Button::new("load-profile-save-btn")
                                    .icon(icon(IconName::Save))
                                    .label("Save")
                                    .xsmall()
                                    .secondary()
                                    .disabled(running || !has_ws)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.spawn_save_load_profile(cx);
                                    })),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .flex_wrap()
                            .child(div().text_xs().text_color(muted).child(if self.state.load_saved_profiles.is_empty() {
                                "No saved profiles".to_string()
                            } else {
                                "Saved:".to_string()
                            }))
                            .children(self.state.load_saved_profiles.iter().take(5).enumerate().map(|(i, p)| {
                                let prof = p.clone();
                                let del_name = p.name.clone();
                                let active = p.name.eq_ignore_ascii_case(self.state.load_profile_name_str.trim());
                                h_flex()
                                    .gap_0p5()
                                    .items_center()
                                    .child({
                                        let b = Button::new(ElementId::NamedInteger("load-prof-pick".into(), i as u64))
                                            .label(p.name.clone())
                                            .xsmall()
                                            .rounded(ButtonRounded::Small)
                                            .disabled(running)
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.state.apply_workspace_load_profile(&prof);
                                                this.spawn_refresh_load_workspace_data(cx);
                                            }));
                                        if active { b.primary() } else { b.outline() }
                                    })
                                    .child(
                                        Button::new(ElementId::NamedInteger("load-prof-del".into(), i as u64))
                                            .icon(icon(IconName::X))
                                            .xsmall()
                                            .ghost()
                                            .disabled(running)
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.spawn_delete_load_profile(del_name.clone(), cx);
                                            })),
                                    )
                            })),
                    )
                    .when(!has_ws, |el| {
                        el.child(div().text_xs().text_color(fg).child("Open a workspace to save profiles"))
                    }),
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
        let scope = &self.state.load_target_scope;
        let folders = self.state.workspace_folder_options();
        let has_ws = self.state.workspace_store.workspace_root.is_some();
        let env_label = self
            .state
            .load_env_override
            .clone()
            .unwrap_or_else(|| "Active Workspace Env".to_string());

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
                    .flex_wrap()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .flex_wrap()
                            .child(div().text_xs().text_color(muted).child("Target Scope:"))
                            .child({
                                let b = Button::new("load-scope-active")
                                    .label("Active Request")
                                    .xsmall()
                                    .rounded(ButtonRounded::Small)
                                    .disabled(running)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.state.load_target_scope = LoadTargetScope::ActiveRequest;
                                        this.state.load_use_collection = false;
                                        this.state.load_scope_dropdown_open = false;
                                        cx.notify();
                                    }));
                                if matches!(scope, LoadTargetScope::ActiveRequest) {
                                    b.primary()
                                } else {
                                    b.outline()
                                }
                            })
                            .child({
                                let b = Button::new("load-scope-workspace")
                                    .label("Entire Workspace")
                                    .xsmall()
                                    .rounded(ButtonRounded::Small)
                                    .disabled(running || !has_ws)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.state.load_target_scope = LoadTargetScope::Workspace;
                                        this.state.load_use_collection = true;
                                        this.state.load_scope_dropdown_open = false;
                                        this.spawn_refresh_load_workspace_data(cx);
                                    }));
                                if matches!(scope, LoadTargetScope::Workspace) {
                                    b.primary()
                                } else {
                                    b.outline()
                                }
                            })
                            .when(!folders.is_empty(), |el| {
                                let folder_btn_label = match scope {
                                    LoadTargetScope::Folder { relative_path } => {
                                        format!("Folder: {relative_path}")
                                    }
                                    _ => "Pick Folder...".to_string(),
                                };
                                let is_folder = matches!(scope, LoadTargetScope::Folder { .. });
                                el.child({
                                    let b = Button::new("load-scope-folder-btn")
                                        .label(folder_btn_label)
                                        .xsmall()
                                        .rounded(ButtonRounded::Small)
                                        .disabled(running)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.state.load_scope_dropdown_open =
                                                !this.state.load_scope_dropdown_open;
                                            cx.notify();
                                        }));
                                    if is_folder { b.primary() } else { b.outline() }
                                })
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(div().text_xs().text_color(muted).child("Env:"))
                            .child(
                                Button::new("load-env-pick-btn")
                                    .label(env_label)
                                    .xsmall()
                                    .outline()
                                    .disabled(running)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.state.load_env_dropdown_open =
                                            !this.state.load_env_dropdown_open;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .when(self.state.load_scope_dropdown_open && !folders.is_empty(), |el| {
                el.child(
                    h_flex()
                        .gap_1()
                        .flex_wrap()
                        .p_1p5()
                        .rounded(radius)
                        .bg(cx.theme().background)
                        .border_1()
                        .border_color(border)
                        .children(folders.iter().enumerate().map(|(i, (rel, label))| {
                            let rel_clone = rel.clone();
                            Button::new(ElementId::NamedInteger("load-folder-opt".into(), i as u64))
                                .label(label.clone())
                                .xsmall()
                                .outline()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.state.load_target_scope = LoadTargetScope::Folder {
                                        relative_path: rel_clone.clone(),
                                    };
                                    this.state.load_use_collection = true;
                                    this.state.load_scope_dropdown_open = false;
                                    this.spawn_refresh_load_workspace_data(cx);
                                }))
                        })),
                )
            })
            .when(self.state.load_env_dropdown_open, |el| {
                el.child(
                    h_flex()
                        .gap_1()
                        .flex_wrap()
                        .p_1p5()
                        .rounded(radius)
                        .bg(cx.theme().background)
                        .border_1()
                        .border_color(border)
                        .child(
                            Button::new("load-env-active-opt")
                                .label("Active Workspace Env")
                                .xsmall()
                                .outline()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.state.load_env_override = None;
                                    this.state.load_env_dropdown_open = false;
                                    cx.notify();
                                })),
                        )
                        .children(self.state.environments.iter().enumerate().map(|(i, env)| {
                            let name = env.1.name.clone();
                            Button::new(ElementId::NamedInteger("load-env-opt".into(), i as u64))
                                .label(name.clone())
                                .xsmall()
                                .outline()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.state.load_env_override = Some(name.clone());
                                    this.state.load_env_dropdown_open = false;
                                    cx.notify();
                                }))
                        })),
                )
            })
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

        let view_barrier = cx.entity().clone();
        let view_weighted = cx.entity().clone();

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
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
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
                            .child(div().text_xs().text_color(fg).child("Weighted Distribution (uses Wt% per request in Requests & Flow tab)")),
                    ),
            )
    }
}
