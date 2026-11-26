use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::{ActiveInputTarget, ScriptSubTab};
use crate::input_registry::{InputKey, InputRegistry};
use aura_core::model::AuraRequest;
use gpui::*;

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_scripts_panel<V: 'static>(
        request: Option<&AuraRequest>,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        sub_tab: ScriptSubTab,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let is_pre = sub_tab == ScriptSubTab::PreRequest;
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
            // Sub-tabs Bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .border_b_1()
                    .border_color(border)
                    .pb_2()
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
            // Snippets Row
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(div().text_xs().text_color(muted).child("Snippets:"))
                    .child(Self::render_snippet_chip("+ Set Env Var", is_pre, "aura.environment.set(\"key\", \"value\");\n", cx, on_action))
                    .child(Self::render_snippet_chip("+ Get Env Var", is_pre, "const val = aura.environment.get(\"key\");\n", cx, on_action))
                    .child(Self::render_snippet_chip("+ Timestamp", is_pre, "aura.variables.set(\"timestamp\", Date.now());\n", cx, on_action))
                    .child(Self::render_snippet_chip("+ Log Info", is_pre, "console.log(\"Executing script at:\", new Date());\n", cx, on_action)),
            )
            // Full-Height Multiline Code Area
            .child(Self::render_inline_textarea(
                "direct-script-editor",
                inputs.get_textarea(if is_pre {
                    &InputKey::PreRequestScript
                } else {
                    &InputKey::PostResponseScript
                }),
                script_content,
                px(260.0),
                is_focused,
                border,
                if is_focused { rgb(0xffffff) } else { normal },
            ))
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
