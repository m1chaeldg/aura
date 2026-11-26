//! Dedicated GraphQL split editor, schema introspection explorer, and live query linter.

use super::{RequestEditorAction, RequestEditorView};
use crate::input_registry::{InputKey, InputRegistry};
use crate::theme::{icon, subtle_surface};
use aura_core::graphql::{lint_graphql_query, GraphQLSchema};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::input::Editor;
use gpui_kit::component::tag::{Tag, TagVariant};
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};
use gpui_kit::base::Disableable as _;

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_graphql_panel<V: 'static>(
        query: &str,
        variables: &str,
        schema: Option<&GraphQLSchema>,
        is_introspecting: bool,
        show_schema: bool,
        inputs: &InputRegistry,
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let diagnostics = lint_graphql_query(query, schema);
        let has_diagnostics = !diagnostics.is_empty();
        let err_count = diagnostics
            .iter()
            .filter(|d| d.severity == "error")
            .count();
        let warn_count = diagnostics.len().saturating_sub(err_count);

        let schema_types_count = schema.map(|s| s.types.len()).unwrap_or(0);
        let danger_fg = cx.theme().red;
        let warn_fg = cx.theme().yellow;
        let success_fg = cx.theme().green;
        let primary_fg = cx.theme().primary;

        let toolbar = h_flex()
            .justify_between()
            .gap_2()
            .flex_wrap()
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Tag::new()
                            .with_variant(TagVariant::Color(gpui_kit::component::ColorName::Pink))
                            .xsmall()
                            .child("GraphQL"),
                    )
                    .child(if err_count > 0 {
                        Tag::new()
                            .with_variant(TagVariant::Danger)
                            .xsmall()
                            .child(format!("{err_count} Lint Error(s)"))
                    } else if warn_count > 0 {
                        Tag::new()
                            .with_variant(TagVariant::Warning)
                            .xsmall()
                            .child(format!("{warn_count} Warning(s)"))
                    } else {
                        Tag::new()
                            .with_variant(TagVariant::Success)
                            .xsmall()
                            .child("Syntax OK")
                    })
                    .when_some(schema, |el, _| {
                        el.child(
                            Tag::secondary()
                                .xsmall()
                                .child(format!("{schema_types_count} Types")),
                        )
                    }),
            )
            .child(
                h_flex()
                    .gap_1p5()
                    .child(
                        Button::new("gql-introspect-btn")
                            .icon(icon(IconName::RefreshCw))
                            .label(if is_introspecting {
                                "Introspecting..."
                            } else {
                                "Introspect Schema"
                            })
                            .xsmall()
                            .outline()
                            .rounded(ButtonRounded::Small)
                            .disabled(is_introspecting)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(
                                    this,
                                    RequestEditorAction::IntrospectGraphQLSchema,
                                    window,
                                    cx,
                                );
                            })),
                    )
                    .when(schema.is_some(), |el| {
                        el.child(
                            Button::new("gql-toggle-schema-btn")
                                .icon(icon(IconName::Braces))
                                .label(if show_schema {
                                    "Hide Schema"
                                } else {
                                    "Schema"
                                })
                                .xsmall()
                                .ghost()
                                .rounded(ButtonRounded::Small)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_action(
                                        this,
                                        RequestEditorAction::ToggleGraphQLSchema,
                                        window,
                                        cx,
                                    );
                                })),
                        )
                    }),
            );

        let query_pane = v_flex()
            .flex_1()
            .min_w_0()
            .gap_1()
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(muted)
                            .child("QUERY"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted)
                            .child("Autocomplete: Ctrl+Space"),
                    ),
            )
            .child(if let Some(editor) = inputs.get_editor(&InputKey::GraphQLQuery) {
                div()
                    .h(px(280.0))
                    .w_full()
                    .rounded(cx.theme().radius)
                    .border_1()
                    .border_color(if err_count > 0 { danger_fg } else { border })
                    .overflow_hidden()
                    .child(Editor::new(editor).h(px(280.0)).w_full())
                    .into_any_element()
            } else {
                div()
                    .p_2()
                    .text_xs()
                    .text_color(normal)
                    .child(query.to_string())
                    .into_any_element()
            });

        let variables_pane = v_flex()
            .w(px(260.0))
            .flex_shrink_0()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(muted)
                    .child("VARIABLES (JSON)"),
            )
            .child(
                if let Some(editor) = inputs.get_editor(&InputKey::GraphQLVariables) {
                    div()
                        .h(px(280.0))
                        .w_full()
                        .rounded(cx.theme().radius)
                        .border_1()
                        .border_color(border)
                        .overflow_hidden()
                        .child(Editor::new(editor).h(px(280.0)).w_full())
                        .into_any_element()
                } else {
                    div()
                        .p_2()
                        .text_xs()
                        .text_color(normal)
                        .child(variables.to_string())
                        .into_any_element()
                },
            );

        let schema_drawer = if show_schema {
            schema.map(|sch| {
                let mut root_fields: Vec<(String, String, String)> = Vec::new();
                for root_name in [
                    sch.query_type.as_deref(),
                    sch.mutation_type.as_deref(),
                    sch.subscription_type.as_deref(),
                ]
                .into_iter()
                .flatten()
                {
                    if let Some(t) = sch.types.get(root_name) {
                        for f in &t.fields {
                            let snippet = if f.args.is_empty() {
                                f.name.clone()
                            } else {
                                let args_s: Vec<String> = f
                                    .args
                                    .iter()
                                    .map(|a| format!("{}: \"\"", a.name))
                                    .collect();
                                format!("{}({})", f.name, args_s.join(", "))
                            };
                            root_fields.push((t.name.clone(), f.return_type.clone(), snippet));
                        }
                    }
                }

                v_flex()
                    .w(px(220.0))
                    .flex_shrink_0()
                    .h(px(298.0))
                    .rounded(cx.theme().radius)
                    .border_1()
                    .border_color(border)
                    .bg(subtle_surface(cx))
                    .p_2()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(primary_fg)
                            .child("SCHEMA FIELDS (click to insert)"),
                    )
                    .child(
                        div()
                            .id("gql-schema-field-list")
                            .flex_1()
                            .overflow_y_scroll()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .children(root_fields.into_iter().take(60).enumerate().map(
                                |(idx, (root_t, ret, snip))| {
                                    let snip_insert = snip.clone();
                                    div()
                                        .id(ElementId::NamedInteger(
                                            "gql-sch-fld".into(),
                                            idx as u64,
                                        ))
                                        .px_1p5()
                                        .py_1()
                                        .rounded(cx.theme().radius)
                                        .cursor_pointer()
                                        .hover(|s| s.bg(cx.theme().list_hover))
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(
                                                this,
                                                RequestEditorAction::InsertGraphQLField(
                                                    snip_insert.clone(),
                                                ),
                                                window,
                                                cx,
                                            );
                                        }))
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(normal)
                                                .truncate()
                                                .child(snip),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(muted)
                                                .truncate()
                                                .child(format!("{root_t} -> {ret}")),
                                        )
                                },
                            )),
                    )
            })
        } else {
            None
        };

        v_flex()
            .gap_2()
            .child(toolbar)
            .child(
                h_flex()
                    .gap_2()
                    .items_start()
                    .child(query_pane)
                    .child(variables_pane)
                    .when_some(schema_drawer, |el, drawer| el.child(drawer)),
            )
            .when(has_diagnostics, |el| {
                el.child(
                    v_flex()
                        .gap_1()
                        .p_2()
                        .rounded(cx.theme().radius)
                        .border_1()
                        .border_color(border)
                        .bg(subtle_surface(cx))
                        .children(diagnostics.into_iter().take(4).map(|d| {
                            let col = if d.severity == "error" {
                                danger_fg
                            } else {
                                warn_fg
                            };
                            h_flex()
                                .gap_2()
                                .text_xs()
                                .child(
                                    icon(IconName::CircleAlert)
                                        .xsmall()
                                        .text_color(col),
                                )
                                .child(
                                    div()
                                        .text_color(muted)
                                        .child(format!("L{}:C{}", d.line, d.column)),
                                )
                                .child(div().text_color(normal).child(d.message))
                        })),
                )
            })
            .when(!has_diagnostics && schema.is_none(), |el| {
                el.child(
                    div()
                        .text_xs()
                        .text_color(success_fg)
                        .child("Tip: Click 'Introspect Schema' to load types and schema-aware linting."),
                )
            })
    }
}
