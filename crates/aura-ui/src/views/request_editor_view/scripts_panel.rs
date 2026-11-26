use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::{ActiveInputTarget, ScriptSubTab};
use crate::input_registry::{InputKey, InputRegistry};
use aura_core::model::AuraRequest;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::{Sizable, h_flex, v_flex};

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_scripts_panel<V: 'static>(
        request: Option<&AuraRequest>,
        variable_rows: &[(String, String, bool)],
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        sub_tab: ScriptSubTab,
        show_script_reference: bool,
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
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

        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .border_b_1()
                    .border_color(border)
                    .pb_2()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .child({
                                let btn = Button::new("subtab-pre-request")
                                    .label("Pre-request Script")
                                    .xsmall()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            RequestEditorAction::SelectScriptSubTab(
                                                ScriptSubTab::PreRequest,
                                            ),
                                            window,
                                            cx,
                                        );
                                    }));
                                if is_pre {
                                    btn.primary()
                                } else {
                                    btn.ghost()
                                }
                            })
                            .child({
                                let btn = Button::new("subtab-post-response")
                                    .label("Post-response Script")
                                    .xsmall()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            RequestEditorAction::SelectScriptSubTab(
                                                ScriptSubTab::PostResponse,
                                            ),
                                            window,
                                            cx,
                                        );
                                    }));
                                if !is_pre {
                                    btn.primary()
                                } else {
                                    btn.ghost()
                                }
                            }),
                    )
                    .child(Self::render_editor_tools(
                        target_key.clone(),
                        show_script_reference,
                        muted,
                        cx,
                        on_action,
                    )),
            )
            .child(
                h_flex()
                    .gap_1p5()
                    .flex_wrap()
                    .child(div().text_xs().text_color(muted).child("Snippets:"))
                    .child(Self::render_snippet_chip(
                        "Set Var",
                        is_pre,
                        "aura.variables.set(\"key\", \"value\");\n",
                        cx,
                        on_action,
                    ))
                    .child(Self::render_snippet_chip(
                        "Get Var",
                        is_pre,
                        "const val = aura.variables.get(\"key\");\n",
                        cx,
                        on_action,
                    ))
                    .child(Self::render_snippet_chip(
                        "Timestamp",
                        is_pre,
                        "aura.variables.set(\"timestamp\", Date.now());\n",
                        cx,
                        on_action,
                    ))
                    .child(Self::render_snippet_chip(
                        "Faker Email",
                        is_pre,
                        "aura.variables.set(\"email\", faker.internet.email());\n",
                        cx,
                        on_action,
                    ))
                    .when(!is_pre, |el| {
                        el.child(Self::render_snippet_chip(
                            "Parse JSON",
                            is_pre,
                            "const data = response.json();\nconsole.log(\"Status:\", response.status, data);\n",
                            cx,
                            on_action,
                        ))
                    })
                    .child(Self::render_snippet_chip(
                        "Log Info",
                        is_pre,
                        "console.log(\"Executing script at:\", new Date());\n",
                        cx,
                        on_action,
                    )),
            )
            .child(Self::render_inline_editor(
                "direct-script-editor",
                inputs.get_editor(&target_key),
                script_content,
                px(260.0),
                is_focused,
                border,
                normal,
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
        Button::new(label)
            .label(label)
            .xsmall()
            .outline()
            .rounded(ButtonRounded::Small)
            .on_click(cx.listener(move |this, _, window, cx| {
                on_action(
                    this,
                    RequestEditorAction::InsertScriptSnippet(is_pre, snippet),
                    window,
                    cx,
                );
            }))
    }
}
