//! Collapsible JSON/XML Tree Viewer panel for `ResponsePaneView`.

use super::{ResponsePaneAction, ResponsePaneView};
use crate::services::{ResponseDataService, TreeNodeKind};
use aura_core::model::AuraResponse;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader};
use gpui_kit::component::tag::{Tag, TagVariant};
use gpui_kit::component::Sizable;
use std::collections::HashSet;

impl ResponsePaneView {
    pub(super) fn render_tree_panel<V: 'static>(
        resp: &AuraResponse,
        collapsed_ids: &HashSet<String>,
        border: Rgba,
        muted: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, ResponsePaneAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        match ResponseDataService::build_visible_tree(&resp.body, collapsed_ids) {
            Err(err) => div().child(
                Empty::new().header(
                    EmptyHeader::new().description(
                        EmptyDescription::new().child(format!(
                            "Tree viewer supports JSON and XML payloads ({err})."
                        )),
                    ),
                ),
            ),
            Ok(nodes) => {
                let total_visible = nodes.len();
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .min_w_0()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child(format!("INTERACTIVE TREE ({total_visible} visible nodes)")),
                            )
                            .child(
                                Button::new("tree-expand-all-btn")
                                    .label("Expand All")
                                    .xsmall()
                                    .secondary()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            ResponsePaneAction::ExpandAllTree,
                                            window,
                                            cx,
                                        );
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .p_2()
                            .rounded_md()
                            .bg(rgb(0x222224))
                            .border_1()
                            .border_color(border)
                            .children(nodes.into_iter().take(500).enumerate().map(|(idx, node)| {
                                let is_collapsed = collapsed_ids.contains(&node.id);
                                let node_id_toggle = node.id.clone();
                                let node_id_copy = node.id.clone();
                                let copy_val = node.copy_value.clone();
                                let indent_px = (node.depth as f32) * 16.0;
                                let (badge_variant, val_color) = match node.kind {
                                    TreeNodeKind::Object => (TagVariant::Primary, rgb(0x9cdcfe)),
                                    TreeNodeKind::Array => (TagVariant::Info, rgb(0x4ec9b0)),
                                    TreeNodeKind::String => (TagVariant::Success, rgb(0xce9178)),
                                    TreeNodeKind::Number => (TagVariant::Warning, rgb(0xb5cea8)),
                                    TreeNodeKind::Bool => (TagVariant::Secondary, rgb(0x569cd6)),
                                    TreeNodeKind::Null => (TagVariant::Secondary, rgb(0x7a7a7a)),
                                    TreeNodeKind::XmlElement => (TagVariant::Primary, rgb(0x4fc1ff)),
                                };

                                div()
                                    .id(ElementId::NamedInteger("resp-tree-row".into(), idx as u64))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_sm()
                                    .hover(|s| s.bg(rgb(0x2d2d30)))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1p5()
                                            .pl(px(indent_px))
                                            .min_w_0()
                                            .flex_1()
                                            .when(node.has_children, |el| {
                                                el.child(
                                                    div()
                                                        .id(ElementId::NamedInteger(
                                                            "resp-tree-toggle".into(),
                                                            idx as u64,
                                                        ))
                                                        .cursor_pointer()
                                                        .text_xs()
                                                        .text_color(muted)
                                                        .child(if is_collapsed { "▶" } else { "▼" })
                                                        .on_click(cx.listener(
                                                            move |this, _, window, cx| {
                                                                on_action(
                                                                    this,
                                                                    ResponsePaneAction::ToggleTreeNode(
                                                                        node_id_toggle.clone(),
                                                                    ),
                                                                    window,
                                                                    cx,
                                                                );
                                                            },
                                                        )),
                                                )
                                            })
                                            .when(!node.has_children, |el| {
                                                el.child(div().w(px(10.0)))
                                            })
                                            .child(
                                                div()
                                                    .font_family("monospace")
                                                    .text_xs()
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(rgb(0x9cdcfe))
                                                    .child(node.key),
                                            )
                                            .child(
                                                Tag::new()
                                                    .with_variant(badge_variant)
                                                    .xsmall()
                                                    .child(node.kind.badge()),
                                            )
                                            .child(
                                                div()
                                                    .font_family("monospace")
                                                    .text_xs()
                                                    .text_color(val_color)
                                                    .truncate()
                                                    .child(node.preview),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .flex_shrink_0()
                                            .child(
                                                Button::new(ElementId::NamedInteger(
                                                    "resp-tree-cp-path".into(),
                                                    idx as u64,
                                                ))
                                                .label("Path")
                                                .xsmall()
                                                .secondary()
                                                .rounded(ButtonRounded::Small)
                                                .on_click(cx.listener(
                                                    move |this, _, window, cx| {
                                                        on_action(
                                                            this,
                                                            ResponsePaneAction::CopyText {
                                                                label: "Path",
                                                                text: node_id_copy.clone(),
                                                            },
                                                            window,
                                                            cx,
                                                        );
                                                    },
                                                )),
                                            )
                                            .child(
                                                Button::new(ElementId::NamedInteger(
                                                    "resp-tree-cp-val".into(),
                                                    idx as u64,
                                                ))
                                                .label("Val")
                                                .xsmall()
                                                .secondary()
                                                .rounded(ButtonRounded::Small)
                                                .on_click(cx.listener(
                                                    move |this, _, window, cx| {
                                                        on_action(
                                                            this,
                                                            ResponsePaneAction::CopyText {
                                                                label: "Value",
                                                                text: copy_val.clone(),
                                                            },
                                                            window,
                                                            cx,
                                                        );
                                                    },
                                                )),
                                            ),
                                    )
                            })),
                    )
            }
        }
    }
}
