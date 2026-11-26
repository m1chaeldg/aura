use super::{RequestEditorAction, RequestEditorView};
use crate::editor_support::{JS_REFERENCE_ENTRIES, SPEC_REFERENCE_ENTRIES};
use crate::input_registry::InputKey;
use aura_core::model::AuraRequest;
use gpui::prelude::FluentBuilder;
use gpui::*;

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_reference_panel<V: 'static>(
        target_key: InputKey,
        is_spec: bool,
        include_response: bool,
        request: Option<&AuraRequest>,
        variable_rows: &[(String, String, bool)],
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let vars: Vec<(String, String)> = variable_rows
            .iter()
            .filter(|(k, _, enabled)| *enabled && !k.trim().is_empty())
            .map(|(k, v, _)| (k.clone(), v.clone()))
            .collect();

        let params: Vec<(String, String)> = request
            .map(|r| {
                r.params
                    .iter()
                    .filter(|p| p.enabled && !p.key.trim().is_empty())
                    .map(|p| (p.key.clone(), p.value.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let headers: Vec<(String, String)> = request
            .map(|r| {
                r.headers
                    .iter()
                    .filter(|h| h.enabled && !h.key.trim().is_empty())
                    .map(|h| (h.key.clone(), h.value.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let entries: Vec<_> = if is_spec {
            SPEC_REFERENCE_ENTRIES.iter().collect()
        } else {
            JS_REFERENCE_ENTRIES
                .iter()
                .filter(|e| include_response || e.category != "Response Object")
                .collect()
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .rounded_md()
            .bg(rgb(0x222224))
            .border_1()
            .border_color(border)
            // Available Variables
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(normal)
                            .child("Available Variables (click to insert)"),
                    )
                    .child(if vars.is_empty() {
                        div()
                            .text_xs()
                            .text_color(muted)
                            .child("No active variables. Add them in the Variables tab.")
                            .into_any_element()
                    } else {
                        let mut row = div().flex().flex_wrap().gap_1();
                        for (idx, (k, v)) in vars.into_iter().enumerate() {
                            let insert = if is_spec {
                                format!("{{{{{k}}}}}")
                            } else {
                                format!("aura.variables.get(\"{k}\")")
                            };
                            let key_clone = target_key.clone();
                            row = row.child(
                                div()
                                    .id(("ref-var", idx))
                                    .px_2()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(rgb(0x272729))
                                    .border_1()
                                    .border_color(border)
                                    .hover(|s| s.border_color(rgb(0x0066cc)).bg(rgb(0x2d2d30)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(rgb(0x4cd964))
                                    .child(format!("{k} = {v}"))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            RequestEditorAction::InsertDynamicSnippet(
                                                key_clone.clone(),
                                                insert.clone(),
                                            ),
                                            window,
                                            cx,
                                        );
                                    })),
                            );
                        }
                        row.into_any_element()
                    }),
            )
            // Query Parameters & Headers in Scope
            .when(!params.is_empty() || !headers.is_empty(), |el| {
                let mut scope = div().flex().flex_wrap().gap_1();
                for (k, v) in &params {
                    scope = scope.child(
                        div()
                            .px_2()
                            .py_0p5()
                            .rounded_sm()
                            .bg(rgb(0x1d1d1f))
                            .border_1()
                            .border_color(border)
                            .text_xs()
                            .text_color(rgb(0x2997ff))
                            .child(format!("param: {k}={v}")),
                    );
                }
                for (k, v) in &headers {
                    scope = scope.child(
                        div()
                            .px_2()
                            .py_0p5()
                            .rounded_sm()
                            .bg(rgb(0x1d1d1f))
                            .border_1()
                            .border_color(border)
                            .text_xs()
                            .text_color(muted)
                            .child(format!("header: {k}: {v}")),
                    );
                }
                el.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(normal)
                                .child("Active Request Parameters & Headers"),
                        )
                        .child(scope),
                )
            })
            // Objects / Language Features Reference
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(normal)
                            .child(if is_spec {
                                "Aura Spec (.aura) Sections & Built-ins (click to insert)"
                            } else {
                                "JavaScript Runtime & Objects Reference (click to insert)"
                            }),
                    )
                    .child({
                        let mut grid = div().flex().flex_col().gap_1();
                        for (idx, entry) in entries.into_iter().enumerate() {
                            let key_clone = target_key.clone();
                            let insert = entry.insert_text.to_string();
                            grid = grid.child(
                                div()
                                    .id(("ref-entry", idx))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    .px_2()
                                    .py_1()
                                    .rounded_sm()
                                    .bg(rgb(0x1d1d1f))
                                    .border_1()
                                    .border_color(border)
                                    .hover(|s| s.border_color(rgb(0x0066cc)).bg(rgb(0x272729)))
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .px_1()
                                                    .rounded_sm()
                                                    .bg(rgb(0x272729))
                                                    .text_xs()
                                                    .text_color(muted)
                                                    .child(entry.category),
                                            )
                                            .child(
                                                div()
                                                    .font_family("monospace")
                                                    .text_xs()
                                                    .text_color(rgb(0x2997ff))
                                                    .child(entry.label),
                                            )
                                            .child(
                                                div()
                                                    .font_family("monospace")
                                                    .text_xs()
                                                    .text_color(muted)
                                                    .child(entry.detail),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(muted)
                                            .child(entry.description),
                                    )
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            RequestEditorAction::InsertDynamicSnippet(
                                                key_clone.clone(),
                                                insert.clone(),
                                            ),
                                            window,
                                            cx,
                                        );
                                    })),
                            );
                        }
                        grid
                    }),
            )
    }
}
