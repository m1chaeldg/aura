use super::{RequestEditorAction, RequestEditorView};
use crate::editor_support::{JS_REFERENCE_ENTRIES, ReferenceEntry, SPEC_REFERENCE_ENTRIES};
use crate::input_registry::InputKey;
use crate::theme::subtle_surface;
use aura_core::model::AuraRequest;
use aura_core::templating::TemplateContext;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::scroll::ScrollableElement as _;
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
        template_ctx: &TemplateContext,
        reference_category: Option<&str>,
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let req_vars: Vec<(String, String)> = variable_rows
            .iter()
            .filter(|(k, _, enabled)| *enabled && !k.trim().is_empty())
            .map(|(k, v, _)| (k.clone(), v.clone()))
            .collect();

        let mut env_vars: Vec<(String, String)> = template_ctx
            .environment_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        env_vars.sort_by(|a, b| a.0.cmp(&b.0));

        let mut col_vars: Vec<(String, String)> = template_ctx
            .collection_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        col_vars.sort_by(|a, b| a.0.cmp(&b.0));

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

        let all_entries: Vec<&'static ReferenceEntry> = if is_spec {
            SPEC_REFERENCE_ENTRIES.iter().collect()
        } else {
            JS_REFERENCE_ENTRIES
                .iter()
                .filter(|e| include_response || e.category != "Response Object")
                .collect()
        };

        let mut categories: Vec<&'static str> = Vec::new();
        for entry in &all_entries {
            if !categories.contains(&entry.category) {
                categories.push(entry.category);
            }
        }

        let active_cat = reference_category.filter(|c| categories.contains(c));
        let entries: Vec<&'static ReferenceEntry> = all_entries
            .into_iter()
            .filter(|e| active_cat.is_none_or(|cat| e.category == cat))
            .collect();

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
                    .gap_1p5()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(normal)
                            .child("Available Variables (Request → Env → Collection, click to insert)"),
                    )
                    .child(if req_vars.is_empty() && env_vars.is_empty() && col_vars.is_empty() {
                        div()
                            .text_xs()
                            .text_color(muted)
                            .child("No active variables. Add Request variables in the Variables tab or select an Environment.")
                            .into_any_element()
                    } else {
                        let mut row = h_flex().flex_wrap().gap_1p5();
                        let mut chip_idx = 0u64;
                        for (scope_tag, list) in [
                            ("req", req_vars),
                            ("env", env_vars),
                            ("col", col_vars),
                        ] {
                            for (k, v) in list {
                                let insert = if is_spec {
                                    format!("{{{{{k}}}}}")
                                } else {
                                    format!("aura.variables.get(\"{k}\")")
                                };
                                let key_clone = target_key.clone();
                                row = row.child(
                                    Button::new(ElementId::NamedInteger("ref-var".into(), chip_idx))
                                        .label(format!("[{scope_tag}] {k} = {v}"))
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
                                chip_idx += 1;
                            }
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
                    .gap_2()
                    .child(
                        h_flex()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(normal)
                                    .child(if is_spec {
                                        "Aura Spec (.aura) Sections & Built-ins (click to insert)"
                                    } else {
                                        "JavaScript Runtime, Tests & Faker Reference (click to insert)"
                                    }),
                            )
                            .child(
                                Tag::secondary()
                                    .xsmall()
                                    .child(format!("{} items", entries.len())),
                            ),
                    )
                    .child({
                        let mut pills = h_flex().flex_wrap().gap_1();
                        let is_all = active_cat.is_none();
                        let all_btn = Button::new("ref-cat-all")
                            .label("All")
                            .xsmall()
                            .rounded(ButtonRounded::Small)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(
                                    this,
                                    RequestEditorAction::SetReferenceCategory(None),
                                    window,
                                    cx,
                                );
                            }));
                        pills = pills.child(if is_all {
                            all_btn.primary()
                        } else {
                            all_btn.ghost()
                        });
                        for (idx, cat) in categories.iter().enumerate() {
                            let is_sel = active_cat == Some(*cat);
                            let cat_owned = (*cat).to_string();
                            let btn = Button::new(ElementId::NamedInteger("ref-cat".into(), idx as u64))
                                .label(*cat)
                                .xsmall()
                                .rounded(ButtonRounded::Small)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_action(
                                        this,
                                        RequestEditorAction::SetReferenceCategory(Some(
                                            cat_owned.clone(),
                                        )),
                                        window,
                                        cx,
                                    );
                                }));
                            pills = pills.child(if is_sel { btn.primary() } else { btn.ghost() });
                        }
                        pills
                    })
                    .child({
                        let mut grid = v_flex()
                            .id("reference-entries-scroll")
                            .max_h(px(320.))
                            .overflow_y_scrollbar()
                            .gap_1()
                            .pr_2();
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
                                            .min_w_0()
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
                                                    .truncate()
                                                    .child(entry.detail),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(muted)
                                            .max_w(px(320.))
                                            .truncate()
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
