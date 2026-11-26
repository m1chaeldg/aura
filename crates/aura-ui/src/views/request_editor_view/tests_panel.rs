use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::ActiveInputTarget;
use crate::input_registry::{InputKey, InputRegistry};
use aura_core::model::{AuraRequest, TestAssertion};
use gpui::prelude::FluentBuilder;
use gpui::*;

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_tests_panel<V: 'static>(
        request: Option<&AuraRequest>,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        assertions: Option<&[TestAssertion]>,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
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

        div()
            .flex()
            .flex_col()
            .gap_3()
            // Top Bar: Title + Results Pill + '▶ Run Tests' Button
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("TEST SUITE (Jasmine / Chai BDD)"))
                            .when(has_results, |el| {
                                let all_passed = passed_tests == total_tests;
                                el.child(
                                    div()
                                        .px_2()
                                        .py_0p5()
                                        .rounded_sm()
                                        .bg(if all_passed { rgb(0x28a745) } else { rgb(0xe74c3c) })
                                        .text_xs()
                                        .text_color(rgb(0xffffff))
                                        .child(format!("{passed_tests}/{total_tests} Passed")),
                                )
                            }),
                    )
                    .child(
                        div()
                            .id("btn-run-tests")
                            .flex()
                            .items_center()
                            .gap_1()
                            .px_3()
                            .py_1()
                            .rounded_sm()
                            .bg(rgb(0x28a745))
                            .hover(|s| s.bg(rgb(0x218838)))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(rgb(0xffffff))
                            .child("▶ Run Tests")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::RunTests, window, cx);
                            })),
                    ),
            )
            // Snippets Row
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(div().text_xs().text_color(muted).child("Snippets:"))
                    .child(Self::render_test_snippet_chip(
                        "+ Status is 200",
                        "describe('Status Code', () => {\n    it('is 200 OK', () => expect(response.status).toBe(200));\n});\n",
                        cx, on_action,
                    ))
                    .child(Self::render_test_snippet_chip(
                        "+ JSON Field Check",
                        "describe('Response Payload', () => {\n    it('has data field', () => {\n        const data = JSON.parse(response.body);\n        expect(data).toBeDefined();\n    });\n});\n",
                        cx, on_action,
                    ))
                    .child(Self::render_test_snippet_chip(
                        "+ Response Time < 500ms",
                        "describe('Performance', () => {\n    it('responds in under 500ms', () => expect(response.duration_ms).toBeLessThan(500));\n});\n",
                        cx, on_action,
                    ))
                    .child(Self::render_test_snippet_chip(
                        "+ Content-Type Header",
                        "describe('Headers', () => {\n    it('is application/json', () => expect(response.headers['content-type']).toContain('application/json'));\n});\n",
                        cx, on_action,
                    )),
            )
            // Full-Height Multiline Code Editor
            .child(Self::render_inline_textarea(
                "direct-test-editor",
                inputs.get_textarea(&InputKey::TestScript),
                tests,
                px(260.0),
                is_focused,
                border,
                if is_focused { rgb(0xffffff) } else { normal },
            ))
    }

    pub(super) fn render_test_snippet_chip<V: 'static>(
        label: &'static str,
        snippet: &'static str,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        div()
            .id(label)
            .px_2()
            .py_0p5()
            .rounded_sm()
            .bg(rgb(0x272729))
            .border_1()
            .border_color(rgb(0x333333))
            .hover(|s| s.bg(rgb(0x333333)).border_color(rgb(0x0066cc)))
            .cursor_pointer()
            .text_xs()
            .text_color(rgb(0x2997ff))
            .child(label)
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
