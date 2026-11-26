use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::ActiveInputTarget;
use crate::input_registry::{InputKey, InputRegistry};
use crate::theme::icon;
use aura_core::model::{AuraRequest, TestAssertion};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{Sizable, h_flex, v_flex};

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
                                    .child("TEST SUITE (Jasmine / Chai BDD)"),
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
}
