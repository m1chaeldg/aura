//! Requests & Flow checklist panel inside the Load Tester modal.
//! Shows discovered workspace requests with inline Include, Once-Only, Weight %, Loop, If, and While controls.

use super::AuraWorkspaceView;
use crate::input_registry::InputKey;
use crate::theme::{icon, method_color, subtle_surface};
use aura_sync::LoadTargetScope;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::base::Disableable as _;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::input::Input;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};

impl AuraWorkspaceView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_load_requests_panel(
        &self,
        border: Hsla,
        muted: Hsla,
        fg: Hsla,
        radius: Pixels,
        running: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let surface = subtle_surface(cx);
        let is_active_only = matches!(self.state.load_target_scope, LoadTargetScope::ActiveRequest);
        let total = self.state.load_discovered_requests.len();
        let included = self
            .state
            .load_discovered_requests
            .iter()
            .filter(|r| r.included)
            .count();

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
                            .gap_2()
                            .items_center()
                            .child(div().text_xs().text_color(fg).child(if is_active_only {
                                "Target Scope is 'Active Request' — switch to Workspace or Folder to run multiple requests.".to_string()
                            } else {
                                format!("Discovered Requests: {included} / {total} included")
                            }))
                            .when(is_active_only, |el| {
                                el.child(
                                    Button::new("load-req-switch-ws")
                                        .label("Use Entire Workspace")
                                        .xsmall()
                                        .primary()
                                        .disabled(running)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.state.load_target_scope = LoadTargetScope::Workspace;
                                            this.state.load_use_collection = true;
                                            this.spawn_refresh_load_workspace_data(cx);
                                        })),
                                )
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("load-req-all-btn")
                                    .label("Select All")
                                    .xsmall()
                                    .outline()
                                    .disabled(running || total == 0)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        for idx in 0..this.state.load_discovered_requests.len() {
                                            this.state.load_discovered_requests[idx].included = true;
                                            this.state.sync_override_for_request_idx(idx);
                                        }
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("load-req-none-btn")
                                    .label("Deselect All")
                                    .xsmall()
                                    .outline()
                                    .disabled(running || total == 0)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        for idx in 0..this.state.load_discovered_requests.len() {
                                            this.state.load_discovered_requests[idx].included = false;
                                            this.state.sync_override_for_request_idx(idx);
                                        }
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("load-req-refresh-btn")
                                    .icon(icon(IconName::RefreshCw))
                                    .label("Refresh")
                                    .xsmall()
                                    .outline()
                                    .disabled(running)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.spawn_refresh_load_workspace_data(cx);
                                    })),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .id("load-requests-scroll")
                    .max_h(px(410.0))
                    .overflow_y_scroll()
                    .gap_1p5()
                    .when(total == 0, |el| {
                        el.child(
                            div()
                                .p_4()
                                .text_xs()
                                .text_color(muted)
                                .child("No requests discovered in the active workspace. Open a workspace folder containing .aura or .toml request files."),
                        )
                    })
                    .children(self.state.load_discovered_requests.iter().enumerate().map(|(idx, entry)| {
                        let once_on = entry.request.flow.as_ref().map(|f| f.once_per_run).unwrap_or(false);
                        let view_inc = cx.entity().clone();
                        let method_col = method_color(&entry.method, cx);

                        v_flex()
                            .gap_1()
                            .p_2()
                            .rounded(radius)
                            .bg(cx.theme().background)
                            .border_1()
                            .border_color(border)
                            .child(
                                h_flex()
                                    .justify_between()
                                    .gap_2()
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .items_center()
                                            .flex_1()
                                            .child(
                                                Switch::new(ElementId::NamedInteger("load-req-inc".into(), idx as u64))
                                                    .checked(entry.included)
                                                    .small()
                                                    .on_change(move |_, _, app| {
                                                        if running {
                                                            return;
                                                        }
                                                        let _ = view_inc.update(app, |this, cx| {
                                                            if let Some(e) = this.state.load_discovered_requests.get_mut(idx) {
                                                                e.included = !e.included;
                                                                this.state.sync_override_for_request_idx(idx);
                                                                cx.notify();
                                                            }
                                                        });
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(method_col)
                                                    .w(px(52.0))
                                                    .child(entry.method.as_str().to_string()),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(fg)
                                                    .child(entry.name.clone()),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(muted)
                                                    .child(entry.relative_path.clone()),
                                            )
                                            .when(entry.is_dirty_tab, |el| {
                                                el.child(Tag::warning().small().child("unsaved"))
                                            }),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_1p5()
                                    .items_center()
                                    .pl(px(36.0))
                                    .flex_wrap()
                                    .child({
                                        let b = Button::new(ElementId::NamedInteger("load-req-once".into(), idx as u64))
                                            .label("Once-Only")
                                            .xsmall()
                                            .rounded(ButtonRounded::Small)
                                            .disabled(running)
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                if let Some(e) = this.state.load_discovered_requests.get_mut(idx) {
                                                    let flow = e.request.flow.get_or_insert_with(Default::default);
                                                    flow.once_per_run = !flow.once_per_run;
                                                    this.state.sync_override_for_request_idx(idx);
                                                    cx.notify();
                                                }
                                            }));
                                        if once_on { b.primary() } else { b.outline() }
                                    })
                                    .child(div().text_xs().text_color(muted).child("Wt%:"))
                                    .child(div().w(px(52.0)).when_some(
                                        self.inputs.get_input(&InputKey::LoadReqWeight(idx)),
                                        |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                    ))
                                    .child(div().text_xs().text_color(muted).child("Loop:"))
                                    .child(div().w(px(46.0)).when_some(
                                        self.inputs.get_input(&InputKey::LoadReqLoop(idx)),
                                        |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                    ))
                                    .child(div().text_xs().text_color(muted).child("If:"))
                                    .child(div().w(px(140.0)).when_some(
                                        self.inputs.get_input(&InputKey::LoadReqIf(idx)),
                                        |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                    ))
                                    .child(div().text_xs().text_color(muted).child("While:"))
                                    .child(div().w(px(140.0)).when_some(
                                        self.inputs.get_input(&InputKey::LoadReqWhile(idx)),
                                        |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                                    )),
                            )
                    })),
            )
    }
}
