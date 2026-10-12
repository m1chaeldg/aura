//! Sidebar View: Displays workspace collections, folders, and request items.

use crate::app_state::AuraAppState;
use crate::stores::workspace_store::WorkspaceTreeNode;
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

        let mut item_elements: Vec<AnyElement> = Vec::new();

        if !state.workspace_store.tree.is_empty() {
            let mut element_counter = 0u64;
            for node in &state.workspace_store.tree {
                render_tree_node(
                    node,
                    0,
                    state,
                    cx,
                    on_select,
                    &mut element_counter,
                    &mut item_elements,
                );
            }
        } else if state.collections.is_empty() {
            item_elements.push(
                div()
                    .px_3()
                    .py_4()
                    .text_xs()
                    .text_color(text_muted)
                    .child("No requests or collections in workspace.")
                    .into_any_element(),
            );
        } else {
            let mut req_counter = 0u64;
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
                    let badge_col = method_badge_color(req.method);

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

                item_elements.push(
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

        let workspace_title = state
            .workspace_store
            .metadata
            .as_ref()
            .map(|m| m.name.clone())
            .unwrap_or_else(|| {
                if !state.collections.is_empty() {
                    format!("COLLECTIONS ({})", state.collections.len())
                } else {
                    "WORKSPACE".to_string()
                }
            });

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
                            .child(workspace_title),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .id("sidebar-new-btn")
                                    .text_xs()
                                    .text_color(accent_blue)
                                    .hover(|s| s.text_color(rgb(0x0071e3)))
                                    .child("+ Req")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_new(this, window, cx);
                                    })),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .p_2()
                    .gap_1()
                    .overflow_hidden()
                    .children(item_elements),
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

fn method_badge_color(method: HttpMethod) -> Rgba {
    match method {
        HttpMethod::GET => rgb(0x0066cc),
        HttpMethod::POST => rgb(0x28a745),
        HttpMethod::PUT => rgb(0xf5a623),
        HttpMethod::PATCH => rgb(0x8e44ad),
        HttpMethod::DELETE => rgb(0xe74c3c),
        _ => rgb(0x7a7a7a),
    }
}

fn render_tree_node<V: 'static>(
    node: &WorkspaceTreeNode,
    depth: usize,
    state: &AuraAppState,
    cx: &mut Context<V>,
    on_select: impl Fn(&mut V, &str, &str, &mut Window, &mut Context<V>) + 'static + Copy,
    counter: &mut u64,
    out: &mut Vec<AnyElement>,
) {
    let padding_left = px(8.0 + (depth as f32 * 12.0));
    let node_path = node.path();
    let is_rename = state
        .inline_rename
        .as_ref()
        .map(|r| r.path == node_path)
        .unwrap_or(false);
    let is_drag_target = state
        .drag_target
        .as_ref()
        .map(|d| d == node_path)
        .unwrap_or(false);

    let border_indicator = if is_drag_target {
        rgb(0x0066cc)
    } else {
        rgb(0x222224)
    };

    match node {
        WorkspaceTreeNode::Folder {
            name,
            path: _,
            is_expanded,
            children,
        } => {
            let chevron = if *is_expanded { "▾" } else { "▸" };
            let el_id = ElementId::NamedInteger("sidebar-folder".into(), *counter);
            *counter += 1;

            out.push(
                div()
                    .id(el_id)
                    .flex()
                    .items_center()
                    .gap_2()
                    .pl(padding_left)
                    .pr_2()
                    .py_1()
                    .rounded_sm()
                    .hover(|s| s.bg(rgb(0x2a2a2c)))
                    .border_b_1()
                    .border_color(border_indicator)
                    .text_xs()
                    .text_color(rgb(0xffffff))
                    .child(div().text_color(rgb(0x7a7a7a)).child(chevron))
                    .child("📁")
                    .child(if is_rename {
                        div()
                            .px_1()
                            .bg(rgb(0x1d1d1f))
                            .border_1()
                            .border_color(rgb(0x0066cc))
                            .rounded_sm()
                            .child(name.clone())
                    } else {
                        div().child(name.clone())
                    })
                    .into_any_element(),
            );

            if *is_expanded {
                for child in children {
                    render_tree_node(child, depth + 1, state, cx, on_select, counter, out);
                }
            }
        }
        WorkspaceTreeNode::Request {
            name,
            path,
            method,
        } => {
            let badge_col = method_badge_color(*method);
            let el_id = ElementId::NamedInteger("sidebar-node-req".into(), *counter);
            *counter += 1;
            let path_str = path.to_string_lossy().to_string();

            out.push(
                div()
                    .id(el_id)
                    .flex()
                    .items_center()
                    .gap_2()
                    .pl(padding_left)
                    .pr_2()
                    .py_1()
                    .rounded_sm()
                    .hover(|s| s.bg(rgb(0x2a2a2c)))
                    .border_b_1()
                    .border_color(border_indicator)
                    .text_xs()
                    .text_color(rgb(0xcccccc))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        on_select(this, "", &path_str, window, cx);
                    }))
                    .child(
                        div()
                            .px_1()
                            .rounded_sm()
                            .bg(rgb(0x1b1b1d))
                            .text_xs()
                            .text_color(badge_col)
                            .child(method.as_str()),
                    )
                    .child(if is_rename {
                        div()
                            .px_1()
                            .bg(rgb(0x1d1d1f))
                            .border_1()
                            .border_color(rgb(0x0066cc))
                            .rounded_sm()
                            .child(name.clone())
                    } else {
                        div().child(name.clone())
                    })
                    .into_any_element(),
            );
        }
    }
}
