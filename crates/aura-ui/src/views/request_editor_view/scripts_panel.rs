use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::{ActiveInputTarget, ScriptSubTab};
use crate::input_registry::{InputKey, InputRegistry};
use aura_core::model::AuraRequest;
use gpui::prelude::FluentBuilder;
use gpui::*;

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_scripts_panel<V: 'static>(
        request: Option<&AuraRequest>,
        variable_rows: &[(String, String, bool)],
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        sub_tab: ScriptSubTab,
        show_script_reference: bool,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let is_pre = sub_tab == ScriptSubTab::PreRequest;
        let target_key = if is_pre {
            InputKey::PreRequestScript
        } else {
            InputKey::PostResponseScript
        };
        let script_content = request
            .map(|r| {
                if is_pre {
                    r.scripts.pre_request.as_str()
                } else {
                    r.scripts.post_response.as_str()
                }
            })
            .unwrap_or("");
        let is_focused = if is_pre {
            matches!(active_input, Some(ActiveInputTarget::PreRequestScript))
        } else {
            matches!(active_input, Some(ActiveInputTarget::PostResponseScript))
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            // Sub-tabs Bar + Editor Tools
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .border_b_1()
                    .border_color(border)
                    .pb_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .id("subtab-pre-request")
                                    .px_3()
                                    .py_1()
                                    .rounded_sm()
                                    .bg(if is_pre { rgb(0x0066cc) } else { rgb(0x272729) })
                                    .hover(|s| s.bg(rgb(0x333333)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(if is_pre { rgb(0xffffff) } else { muted })
                                    .child("Pre-request Script")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::SelectScriptSubTab(ScriptSubTab::PreRequest), window, cx);
                                    })),
                            )
                            .child(
                                div()
                                    .id("subtab-post-response")
                                    .px_3()
                                    .py_1()
                                    .rounded_sm()
                                    .bg(if !is_pre { rgb(0x0066cc) } else { rgb(0x272729) })
                                    .hover(|s| s.bg(rgb(0x333333)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(if !is_pre { rgb(0xffffff) } else { muted })
                                    .child("Post-response Script")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::SelectScriptSubTab(ScriptSubTab::PostResponse), window, cx);
                                    })),
                            ),
                    )
                    .child(Self::render_editor_tools(
                        target_key.clone(),
                        show_script_reference,
                        muted,
                        cx,
                        on_action,
                    )),
            )
            // Snippets Row
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(div().text_xs().text_color(muted).child("Snippets:"))
                    .child(Self::render_snippet_chip("+ Set Var", is_pre, "aura.variables.set(\"key\", \"value\");\n", cx, on_action))
                    .child(Self::render_snippet_chip("+ Get Var", is_pre, "const val = aura.variables.get(\"key\");\n", cx, on_action))
                    .child(Self::render_snippet_chip("+ Timestamp", is_pre, "aura.variables.set(\"timestamp\", Date.now());\n", cx, on_action))
                    .child(Self::render_snippet_chip("+ Faker Email", is_pre, "aura.variables.set(\"email\", faker.internet.email());\n", cx, on_action))
                    .when(!is_pre, |el| {
                        el.child(Self::render_snippet_chip(
                            "+ Parse JSON",
                            is_pre,
                            "const data = response.json();\nconsole.log(\"Status:\", response.status, data);\n",
                            cx,
                            on_action,
                        ))
                    })
                    .child(Self::render_snippet_chip("+ Log Info", is_pre, "console.log(\"Executing script at:\", new Date());\n", cx, on_action)),
            )
            // Full-Height Multiline Code Area
            .child(Self::render_inline_editor(
                "direct-script-editor",
                inputs.get_editor(&target_key),
                script_content,
                px(260.0),
                is_focused,
                border,
                if is_focused { rgb(0xffffff) } else { normal },
            ))
            .when(show_script_reference, |el| {
                el.child(Self::render_reference_panel(
                    target_key.clone(),
                    false,
                    !is_pre,
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

    pub(super) fn render_snippet_chip<V: 'static>(
        label: &'static str,
        is_pre: bool,
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
                    RequestEditorAction::InsertScriptSnippet(is_pre, snippet),
                    window,
                    cx,
                );
            }))
    }

    // =========================================================================
    // Tests Panel (Full Test Editor + Snippets + '▶ Run Tests')
    // =========================================================================

}
