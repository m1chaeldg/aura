use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::ActiveInputTarget;
use crate::input_registry::{InputKey, InputRegistry};
use crate::theme::icon;
use aura_core::model::{AuraRequest, TestAssertion};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_tests_panel<V: 'static>(
        request: Option<&AuraRequest>,
        variable_rows: &[(String, String, bool)],
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        assertions: Option<&[TestAssertion]>,
        show_script_reference: bool,
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let tests = request.map(|r| r.scripts.tests.as_str()).unwrap_or("");
        let decl_rules = request
            .map(|r| r.scripts.assertions.as_slice())
            .unwrap_or(&[]);
        let is_focused = matches!(active_input, Some(ActiveInputTarget::TestScript));

        let total_tests = assertions.map(|a| a.len()).unwrap_or(0);
        let passed_tests = assertions
            .map(|a| a.iter().filter(|x| x.passed).count())
            .unwrap_or(0);
        let has_results = total_tests > 0;

        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child("ASSERTIONS & TEST SUITE"),
                            )
                            .when(has_results, |el| {
                                let label = format!("{passed_tests}/{total_tests} Passed");
                                if passed_tests == total_tests {
                                    el.child(Tag::success().xsmall().child(label))
                                } else {
                                    el.child(Tag::danger().xsmall().child(label))
                                }
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_1p5()
                            .child(Self::render_editor_tools(
                                InputKey::TestScript,
                                show_script_reference,
                                muted,
                                cx,
                                on_action,
                            ))
                            .child(
                                Button::new("btn-run-tests")
                                    .icon(icon(IconName::Play))
                                    .label("Run Tests")
                                    .xsmall()
                                    .success()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::RunTests, window, cx);
                                    })),
                            ),
                    ),
            )
            .child(Self::render_declarative_assertions_section(
                decl_rules,
                active_input,
                inputs,
                border,
                muted,
                normal,
                cx,
                on_action,
            ))
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(muted)
                    .child("SCRIPT TESTS (Jasmine / Chai BDD)"),
            )
            .child(
                h_flex()
                    .gap_1p5()
                    .flex_wrap()
                    .child(div().text_xs().text_color(muted).child("Snippets:"))
                    .child(Self::render_test_snippet_chip(
                        "Status is 200",
                        "describe('Status Code', () => {\n    it('is 200 OK', () => expect(response.status).toBe(200));\n});\n",
                        cx,
                        on_action,
                    ))
                    .child(Self::render_test_snippet_chip(
                        "JSON Field Check",
                        "describe('Response Payload', () => {\n    it('has data field', () => {\n        const data = JSON.parse(response.body);\n        expect(data).toBeDefined();\n    });\n});\n",
                        cx,
                        on_action,
                    ))
                    .child(Self::render_test_snippet_chip(
                        "Response Time < 500ms",
                        "describe('Performance', () => {\n    it('responds in under 500ms', () => expect(response.duration_ms).toBeLessThan(500));\n});\n",
                        cx,
                        on_action,
                    ))
                    .child(Self::render_test_snippet_chip(
                        "Content-Type Header",
                        "describe('Headers', () => {\n    it('is application/json', () => expect(response.headers['content-type']).toContain('application/json'));\n});\n",
                        cx,
                        on_action,
                    )),
            )
            .child(Self::render_inline_editor(
                "direct-test-editor",
                inputs.get_editor(&InputKey::TestScript),
                tests,
                px(260.0),
                is_focused,
                border,
                normal,
            ))
            .when(show_script_reference, |el| {
                el.child(Self::render_reference_panel(
                    InputKey::TestScript,
                    false,
                    true,
                    request,
                    variable_rows,
                    border,
                    muted,
                    normal,
                    cx,
                    on_action,
                ))
            })
    }

    pub(super) fn render_test_snippet_chip<V: 'static>(
        label: &'static str,
        snippet: &'static str,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        Button::new(label)
            .label(label)
            .xsmall()
            .outline()
            .rounded(ButtonRounded::Small)
            .on_click(cx.listener(move |this, _, window, cx| {
                on_action(
                    this,
                    RequestEditorAction::InsertTestSnippet(snippet),
                    window,
                    cx,
                );
            }))
    }

    #[allow(clippy::too_many_arguments)]
    fn render_declarative_assertions_section<V: 'static>(
        rules: &[aura_core::assertions::DeclarativeAssertion],
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        use aura_core::assertions::AssertOperator;

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
                        h_flex()
                            .gap_1p5()
                            .flex_wrap()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child("NO-CODE ASSERTIONS ([assert])"),
                            )
                            .child(Self::render_assert_preset_chip(
                                "Status 200",
                                "res.status",
                                AssertOperator::Eq,
                                "200",
                                cx,
                                on_action,
                            ))
                            .child(Self::render_assert_preset_chip(
                                "body.id isNumber",
                                "res.body.id",
                                AssertOperator::IsNumber,
                                "",
                                cx,
                                on_action,
                            ))
                            .child(Self::render_assert_preset_chip(
                                "Time < 500ms",
                                "res.duration_ms",
                                AssertOperator::Lt,
                                "500",
                                cx,
                                on_action,
                            )),
                    )
                    .child(
                        Button::new("btn-add-assert")
                            .icon(icon(IconName::Plus))
                            .label("Add Rule")
                            .xsmall()
                            .secondary()
                            .rounded(ButtonRounded::Small)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::AddAssertion, window, cx);
                            })),
                    ),
            )
            .when(rules.is_empty(), |el| {
                el.child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .child("No declarative assertions yet. Add a rule or click a preset (e.g. res.status == 200, res.body.id isNumber)."),
                )
            })
            .children(rules.iter().enumerate().map(|(idx, rule)| {
                let enabled = rule.enabled;
                let expr = rule.expression.clone();
                let expected = rule.expected.clone();
                let op_label = rule.operator.symbol();
                let is_unary = rule.operator.is_unary();
                let is_expr_focused =
                    matches!(active_input, Some(ActiveInputTarget::AssertExpr(i)) if *i == idx);
                let is_exp_focused =
                    matches!(active_input, Some(ActiveInputTarget::AssertExpected(i)) if *i == idx);
                let view_entity = cx.entity().clone();

                h_flex()
                    .id(ElementId::NamedInteger("assert-row".into(), idx as u64))
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
                    .border_color(if is_expr_focused || is_exp_focused {
                        focus_border
                    } else {
                        border
                    })
                    .child(
                        Checkbox::new(ElementId::NamedInteger("toggle-assert".into(), idx as u64))
                            .checked(enabled)
                            .small()
                            .on_click(move |_, window, app| {
                                let _ = view_entity.update(app, |this, cx| {
                                    on_action(
                                        this,
                                        RequestEditorAction::ToggleAssertion(idx),
                                        window,
                                        cx,
                                    );
                                });
                            }),
                    )
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("edit-assert-expr".into(), idx as u64),
                        inputs.get_input(&InputKey::AssertExpr(idx)),
                        &expr,
                        is_expr_focused,
                        false,
                        field_bg,
                        border,
                        if enabled { normal } else { muted },
                    ))
                    .child(
                        Button::new(ElementId::NamedInteger("cycle-assert-op".into(), idx as u64))
                            .label(op_label)
                            .xsmall()
                            .outline()
                            .rounded(ButtonRounded::Small)
                            .tooltip("Click to cycle assertion operator")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(
                                    this,
                                    RequestEditorAction::CycleAssertionOperator(idx),
                                    window,
                                    cx,
                                );
                            })),
                    )
                    .when(!is_unary, |el| {
                        el.child(Self::render_inline_input(
                            ElementId::NamedInteger("edit-assert-expected".into(), idx as u64),
                            inputs.get_input(&InputKey::AssertExpected(idx)),
                            &expected,
                            is_exp_focused,
                            false,
                            field_bg,
                            border,
                            if enabled { normal } else { muted },
                        ))
                    })
                    .child(
                        Button::new(ElementId::NamedInteger("del-assert-btn".into(), idx as u64))
                            .icon(icon(IconName::X))
                            .xsmall()
                            .ghost()
                            .tooltip("Remove assertion rule")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(
                                    this,
                                    RequestEditorAction::RemoveAssertion(idx),
                                    window,
                                    cx,
                                );
                            })),
                    )
            }))
    }

    fn render_assert_preset_chip<V: 'static>(
        label: &'static str,
        expr: &'static str,
        op: aura_core::assertions::AssertOperator,
        expected: &'static str,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        Button::new(ElementId::Name(format!("assert-preset-{label}").into()))
            .label(label)
            .xsmall()
            .outline()
            .rounded(ButtonRounded::Small)
            .on_click(cx.listener(move |this, _, window, cx| {
                on_action(
                    this,
                    RequestEditorAction::AddPresetAssertion(expr, op, expected),
                    window,
                    cx,
                );
            }))
    }
}
