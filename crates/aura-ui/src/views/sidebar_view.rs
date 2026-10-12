//! Sidebar View: Displays workspace collections, folders, and request items.

use crate::app_state::AuraAppState;
use aura_core::model::HttpMethod;
use gpui::*;

pub struct SidebarView;

impl SidebarView {
    pub fn render<V: 'static>(
        state: &AuraAppState,
        cx: &mut Context<V>,
        on_select: impl Fn(&mut V, &str, &str, &mut Window, &mut Context<V>) + 'static + Copy,
        on_new: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let border_color = rgb(0x333333);
        let text_muted = rgb(0x7a7a7a);
        let accent_blue = rgb(0x0066cc);
        let green_color = rgb(0x28a745);

        let mut req_counter = 0u64;
        let mut col_children: Vec<AnyElement> = Vec::new();

        if state.collections.is_empty() {
            col_children.push(
                div()
                    .px_3()
                    .py_4()
                    .text_xs()
                    .text_color(text_muted)
                    .child("No collections in workspace.")
                    .into_any_element(),
            );
        } else {
            for col in &state.collections {
                let col_id = col.id.clone();
                let mut col_items: Vec<AnyElement> = Vec::new();

                col_items.push(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_2()
                        .py_1()
                        .rounded_sm()
                        .text_xs()
                        .text_color(rgb(0xffffff))
                        .child("📁")
                        .child(col.name.clone())
                        .into_any_element(),
                );

                for req in &col.requests {
                    let r_id = req.id.clone();
                    let c_id = col_id.clone();
                    let badge_col = match req.method {
                        HttpMethod::GET => rgb(0x0066cc),
                        HttpMethod::POST => rgb(0x28a745),
                        HttpMethod::PUT => rgb(0xf5a623),
                        HttpMethod::PATCH => rgb(0x8e44ad),
                        HttpMethod::DELETE => rgb(0xe74c3c),
                        _ => rgb(0x7a7a7a),
                    };

                    col_items.push(
                        div()
                            .id(ElementId::NamedInteger("sidebar-req".into(), req_counter))
                            .flex()
                            .items_center()
                            .gap_2()
                            .pl_6()
                            .pr_2()
                            .py_1()
                            .rounded_sm()
                            .hover(|s| s.bg(rgb(0x2a2a2c)))
                            .text_xs()
                            .text_color(rgb(0xcccccc))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_select(this, &c_id, &r_id, window, cx);
                            }))
                            .child(
                                div()
                                    .px_1()
                                    .rounded_sm()
                                    .bg(rgb(0x1b1b1d))
                                    .text_xs()
                                    .text_color(badge_col)
                                    .child(req.method.as_str()),
                            )
                            .child(req.name.clone())
                            .into_any_element(),
                    );
                    req_counter += 1;
                }

                col_children.push(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .py_1()
                        .children(col_items)
                        .into_any_element(),
                );
            }
        }

        div()
            .w(px(240.0))
            .flex()
            .flex_col()
            .bg(rgb(0x222224))
            .border_r_1()
            .border_color(border_color)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(border_color)
                    .child(
                        div()
                            .text_xs()
                            .text_color(text_muted)
                            .child(format!("COLLECTIONS ({})", state.collections.len())),
                    )
                    .child(
                        div()
                            .id("sidebar-new-btn")
                            .text_xs()
                            .text_color(accent_blue)
                            .child("+ New")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_new(this, window, cx);
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .p_2()
                    .gap_1()
                    .children(col_children),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .border_t_1()
                    .border_color(border_color)
                    .text_xs()
                    .text_color(text_muted)
                    .child(div().text_color(green_color).child("✓"))
                    .child("Workspace Synced"),
            )
    }
}
