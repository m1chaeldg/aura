use super::{RequestEditorAction, RequestEditorView};
use crate::editor_support::{JS_REFERENCE_ENTRIES, SPEC_REFERENCE_ENTRIES};
use crate::input_registry::InputKey;
use crate::theme::subtle_surface;
use aura_core::model::AuraRequest;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_reference_panel<V: 'static>(
        target_key: InputKey,
        is_spec: bool,
        include_response: bool,
        request: Option<&AuraRequest>,
        variable_rows: &[(String, String, bool)],
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
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

        let surface = subtle_surface(cx);
        let card_bg = cx.theme().background;
        let primary_color = cx.theme().primary;
        let hover_bg = cx.theme().list_hover;
        let radius = cx.theme().radius;

        v_flex()
            .gap_3()
            .p_3()
            .rounded(radius)
            .bg(surface)
            .border_1()
            .border_color(border)
            .child(
                v_flex()
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
                        let mut row = h_flex().flex_wrap().gap_1p5();
                        for (idx, (k, v)) in vars.into_iter().enumerate() {
                            let insert = if is_spec {
                                format!("{{{{{k}}}}}")
                            } else {
                                format!("aura.variables.get(\"{k}\")")
                            };
                            let key_clone = target_key.clone();
                            row = row.child(
                                Button::new(ElementId::NamedInteger("ref-var".into(), idx as u64))
                                    .label(format!("{k} = {v}"))
                                    .xsmall()
                                    .outline()
                                    .rounded(ButtonRounded::Small)
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
            .when(!params.is_empty() || !headers.is_empty(), |el| {
                let mut scope = h_flex().flex_wrap().gap_1p5();
                for (k, v) in &params {
                    scope = scope.child(
                        Tag::info()
                            .outline()
                            .xsmall()
                            .child(format!("param: {k}={v}")),
                    );
                }
                for (k, v) in &headers {
                    scope = scope.child(
                        Tag::secondary()
                            .outline()
                            .xsmall()
                            .child(format!("header: {k}: {v}")),
                    );
                }
                el.child(
                    v_flex()
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
            .child(
                v_flex()
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
                        let mut grid = v_flex().gap_1();
                        for (idx, entry) in entries.into_iter().enumerate() {
                            let key_clone = target_key.clone();
                            let insert = entry.insert_text.to_string();
                            grid = grid.child(
                                h_flex()
                                    .id(("ref-entry", idx))
                                    .justify_between()
                                    .gap_2()
                                    .px_2()
                                    .py_1()
                                    .rounded(radius)
                                    .bg(card_bg)
                                    .border_1()
                                    .border_color(border)
                                    .hover(move |s| s.border_color(primary_color).bg(hover_bg))
                                    .cursor_pointer()
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .child(
                                                Tag::secondary()
                                                    .xsmall()
                                                    .child(entry.category),
                                            )
                                            .child(
                                                div()
                                                    .font_family("monospace")
                                                    .text_xs()
                                                    .text_color(primary_color)
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
