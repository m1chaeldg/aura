//! Collapsible JSON/XML Tree Viewer panel for `ResponsePaneView`.

use super::{ResponsePaneAction, ResponsePaneView};
use crate::services::{ResponseDataService, TreeNodeKind};
use crate::theme::{icon, subtle_surface};
use aura_core::model::AuraResponse;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonRounded};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::tag::{Tag, TagVariant};
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};
use std::collections::HashSet;

impl ResponsePaneView {
    pub(super) fn render_tree_panel<V: 'static>(
        resp: &AuraResponse,
        collapsed_ids: &HashSet<String>,
        border: Hsla,
        muted: Hsla,
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
                let tree_bg = subtle_surface(cx);
                let hover_bg = cx.theme().list_hover;
                let key_color = cx.theme().blue;
                let str_color = cx.theme().green;
                let num_color = cx.theme().yellow;
                let array_color = cx.theme().cyan;
                let bool_color = cx.theme().magenta;
                let radius = cx.theme().radius;

                v_flex()
                    .gap_2()
                    .min_w_0()
                    .child(
                        h_flex()
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
                                    .outline()
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
                        v_flex()
                            .id("response-tree-scroll")
                            .max_h(px(460.))
                            .overflow_y_scrollbar()
                            .gap_0p5()
                            .p_2()
                            .rounded(radius)
                            .bg(tree_bg)
                            .border_1()
                            .border_color(border)
                            .children(nodes.into_iter().take(500).enumerate().map(|(idx, node)| {
                                let is_collapsed = collapsed_ids.contains(&node.id);
                                let node_id_toggle = node.id.clone();
                                let node_id_copy = node.id.clone();
                                let copy_val = node.copy_value.clone();
                                let indent_px = (node.depth as f32) * 16.0;
                                let (badge_variant, val_color) = match node.kind {
                                    TreeNodeKind::Object => (TagVariant::Primary, key_color),
                                    TreeNodeKind::Array => (TagVariant::Info, array_color),
                                    TreeNodeKind::String => (TagVariant::Success, str_color),
                                    TreeNodeKind::Number => (TagVariant::Warning, num_color),
                                    TreeNodeKind::Bool => (TagVariant::Secondary, bool_color),
                                    TreeNodeKind::Null => (TagVariant::Secondary, muted),
                                    TreeNodeKind::XmlElement => (TagVariant::Primary, key_color),
                                };

                                h_flex()
                                    .id(ElementId::NamedInteger("resp-tree-row".into(), idx as u64))
                                    .justify_between()
                                    .gap_2()
                                    .px_2()
                                    .py_0p5()
                                    .rounded(radius)
                                    .hover(move |s| s.bg(hover_bg))
                                    .child(
                                        h_flex()
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
                                                        .text_color(muted)
                                                        .child(
                                                            icon(if is_collapsed {
                                                                IconName::ChevronRight
                                                            } else {
                                                                IconName::ChevronDown
                                                            })
                                                            .xsmall(),
                                                        )
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
                                                el.child(div().w(px(12.0)))
                                            })
                                            .child(
                                                div()
                                                    .font_family("monospace")
                                                    .text_xs()
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(key_color)
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
                                        h_flex()
                                            .gap_1()
                                            .flex_shrink_0()
                                            .child(
                                                Button::new(ElementId::NamedInteger(
                                                    "resp-tree-cp-path".into(),
                                                    idx as u64,
                                                ))
                                                .label("Path")
                                                .xsmall()
                                                .outline()
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
                                                .outline()
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
