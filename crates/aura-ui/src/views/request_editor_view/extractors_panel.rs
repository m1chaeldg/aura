use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::ActiveInputTarget;
use crate::input_registry::{InputKey, InputRegistry};
use crate::theme::icon;
use aura_core::extractors::NativeExtractor;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_native_extractors_section<V: 'static>(
        extractors: &[NativeExtractor],
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let row_active_bg = cx.theme().secondary.opacity(0.35);
        let row_disabled_bg = cx.theme().muted.opacity(0.2);
        let focus_border = cx.theme().primary;
        let field_bg = cx.theme().background;
        let radius = cx.theme().radius;

        v_flex()
            .gap_2()
            .p_2()
            .rounded(radius)
            .border_1()
            .border_color(border)
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(muted)
                            .child("NATIVE EXTRACTORS ([extractors], Zero-JS Correlation)"),
                    )
                    .child(
                        Button::new("btn-add-extractor")
                            .icon(icon(IconName::Plus))
                            .label("Add Extractor")
                            .xsmall()
                            .secondary()
                            .rounded(ButtonRounded::Small)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::AddExtractor, window, cx);
                            })),
                    ),
            )
            .when(extractors.is_empty(), |el| {
                el.child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .child("No native extractors yet. Extract JSONPath ($.data.token), Regex, Header, or Cookie values without V8 overhead."),
                )
            })
            .children(extractors.iter().enumerate().map(|(idx, ext)| {
                let enabled = ext.enabled;
                let var = ext.variable.clone();
                let expr = ext.expression.clone();
                let kind_label = ext.kind.label();
                let is_var_focused =
                    matches!(active_input, Some(ActiveInputTarget::ExtractorVar(i)) if *i == idx);
                let is_expr_focused =
                    matches!(active_input, Some(ActiveInputTarget::ExtractorExpr(i)) if *i == idx);
                let view_entity = cx.entity().clone();

                h_flex()
                    .id(ElementId::NamedInteger("extractor-row".into(), idx as u64))
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded(radius)
                    .bg(if enabled {
                        row_active_bg
                    } else {
                        row_disabled_bg
                    })
                    .border_1()
                    .border_color(if is_var_focused || is_expr_focused {
                        focus_border
                    } else {
                        border
                    })
                    .child(
                        Checkbox::new(ElementId::NamedInteger("toggle-extractor".into(), idx as u64))
                            .checked(enabled)
                            .small()
                            .on_click(move |_, window, app| {
                                let _ = view_entity.update(app, |this, cx| {
                                    on_action(
                                        this,
                                        RequestEditorAction::ToggleExtractor(idx),
                                        window,
                                        cx,
                                    );
                                });
                            }),
                    )
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("edit-extractor-var".into(), idx as u64),
                        inputs.get_input(&InputKey::ExtractorVar(idx)),
                        &var,
                        is_var_focused,
                        false,
                        field_bg,
                        border,
                        if enabled { normal } else { muted },
                    ))
                    .child(
                        Button::new(ElementId::NamedInteger("cycle-extractor-kind".into(), idx as u64))
                            .label(kind_label)
                            .xsmall()
                            .outline()
                            .rounded(ButtonRounded::Small)
                            .tooltip("Click to switch extractor type (JSONPath, Regex, Header, Cookie)")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(
                                    this,
                                    RequestEditorAction::CycleExtractorType(idx),
                                    window,
                                    cx,
                                );
                            })),
                    )
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("edit-extractor-expr".into(), idx as u64),
                        inputs.get_input(&InputKey::ExtractorExpr(idx)),
                        &expr,
                        is_expr_focused,
                        false,
                        field_bg,
                        border,
                        if enabled { normal } else { muted },
                    ))
                    .child(
                        Button::new(ElementId::NamedInteger("del-extractor-btn".into(), idx as u64))
                            .icon(icon(IconName::X))
                            .xsmall()
                            .ghost()
                            .tooltip("Remove native extractor")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(
                                    this,
                                    RequestEditorAction::RemoveExtractor(idx),
                                    window,
                                    cx,
                                );
                            })),
                    )
            }))
    }
}
