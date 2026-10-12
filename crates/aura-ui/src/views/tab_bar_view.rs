//! TabBar View: Renders open request document tabs, badges, dirty dots, and close buttons.

use crate::app_state::AuraAppState;
use aura_core::model::HttpMethod;
use gpui::prelude::FluentBuilder;
use gpui::*;

pub struct TabBarView;

impl TabBarView {
    pub fn render<V: 'static>(
        state: &AuraAppState,
        cx: &mut Context<V>,
        on_select_tab: impl Fn(&mut V, usize, &mut Window, &mut Context<V>) + 'static + Copy,
        on_close_tab: impl Fn(&mut V, &str, &mut Window, &mut Context<V>) + 'static + Copy,
        on_new_request: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let border_color = rgb(0x333333);
        let accent_blue = rgb(0x0066cc);
        let amber_color = rgb(0xf5a623);
        let text_muted = rgb(0x7a7a7a);

        div()
            .flex()
            .items_center()
            .px_3()
            .py_1()
            .bg(rgb(0x222224))
            .border_b_1()
            .border_color(border_color)
            .gap_1()
            .children(state.tabs.iter().enumerate().map(|(idx, tab)| {
                let is_active = idx == state.active_tab_index;
                let tab_bg = if is_active {
                    rgb(0x272729)
                } else {
                    rgb(0x1d1d1f)
                };
                let tab_border = if is_active { accent_blue } else { border_color };

                let method_color = match tab.method {
                    HttpMethod::GET => rgb(0x0066cc),
                    HttpMethod::POST => rgb(0x28a745),
                    HttpMethod::PUT => rgb(0xf5a623),
                    HttpMethod::PATCH => rgb(0x8e44ad),
                    HttpMethod::DELETE => rgb(0xe74c3c),
                    _ => rgb(0x7a7a7a),
                };

                let tab_id_close = tab.id.clone();

                div()
                    .id(ElementId::NamedInteger("workspace-tab".into(), idx as u64))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .bg(tab_bg)
                    .border_1()
                    .border_color(tab_border)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        on_select_tab(this, idx, window, cx);
                    }))
                    .child(
                        div()
                            .px_1()
                            .rounded_sm()
                            .bg(rgb(0x1b1b1d))
                            .text_xs()
                            .text_color(method_color)
                            .child(tab.method.as_str()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(if is_active { rgb(0xffffff) } else { rgb(0xaaaaaa) })
                            .child(tab.title.clone()),
                    )
                    .when(tab.is_dirty, |el| {
                        el.child(
                            div()
                                .w_2()
                                .h_2()
                                .rounded_full()
                                .bg(amber_color),
                        )
                    })
                    .child(
                        div()
                            .id(ElementId::NamedInteger("close-tab".into(), idx as u64))
                            .flex()
                            .items_center()
                            .justify_center()
                            .w_4()
                            .h_4()
                            .rounded_full()
                            .text_xs()
                            .text_color(text_muted)
                            .child("×")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_close_tab(this, &tab_id_close, window, cx);
                            })),
                    )
            }))
            .child(
                div()
                    .id("new-request-btn")
                    .flex()
                    .items_center()
                    .justify_center()
                    .w_6()
                    .h_6()
                    .rounded_md()
                    .bg(rgb(0x333333))
                    .text_xs()
                    .text_color(rgb(0xffffff))
                    .child("+")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        on_new_request(this, window, cx);
                    })),
            )
    }
}
