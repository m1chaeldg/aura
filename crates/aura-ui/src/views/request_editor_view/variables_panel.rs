use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::ActiveInputTarget;
use crate::input_registry::{InputKey, InputRegistry};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::Sizable;

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_variables_panel<V: 'static>(
        variable_rows: &[(String, String)],
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_3()
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
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child("REQUEST VARIABLES"),
                            )
                            .child(
                                Tag::secondary()
                                    .xsmall()
                                    .child(format!("{}", variable_rows.len())),
                            ),
                    )
                    .child(
                        Button::new("add-variable-btn")
                            .label("+ Add Variable")
                            .primary()
                            .xsmall()
                            .rounded(ButtonRounded::Small)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::AddVariableDirect, window, cx);
                            })),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child("Reference variables with {{var_name}} in URL, headers, params, and body."),
            )
            .when(variable_rows.is_empty(), |el| {
                el.child(
                    Empty::new()
                        .header(EmptyHeader::new().description(
                            EmptyDescription::new().child(
                                "No request variables defined. Click '+ Add Variable' to add one.",
                            ),
                        ))
                        .child(
                            Button::new("btn-add-first-variable")
                                .label("+ Add First Variable")
                                .secondary()
                                .small()
                                .rounded(ButtonRounded::Medium)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_action(
                                        this,
                                        RequestEditorAction::AddVariableDirect,
                                        window,
                                        cx,
                                    );
                                })),
                        ),
                )
            })
            .children(variable_rows.iter().enumerate().map(|(idx, (key, val))| {
                let is_key_focused =
                    matches!(active_input, Some(ActiveInputTarget::VariableKey(i)) if *i == idx);
                let is_val_focused =
                    matches!(active_input, Some(ActiveInputTarget::VariableValue(i)) if *i == idx);

                div()
                    .id(ElementId::NamedInteger("variable-row".into(), idx as u64))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1p5()
                    .rounded_md()
                    .bg(rgb(0x272729))
                    .border_1()
                    .border_color(if is_key_focused || is_val_focused {
                        rgb(0x0066cc)
                    } else {
                        border
                    })
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("edit-var-key".into(), idx as u64),
                        inputs.get_input(&InputKey::VariableKey(idx)),
                        key,
                        is_key_focused,
                        false,
                        rgb(0x222224),
                        rgb(0x38383a),
                        if is_key_focused { rgb(0xffffff) } else { normal },
                    ))
                    .child(div().text_xs().text_color(muted).child("="))
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("edit-var-val".into(), idx as u64),
                        inputs.get_input(&InputKey::VariableValue(idx)),
                        val,
                        is_val_focused,
                        false,
                        rgb(0x222224),
                        rgb(0x38383a),
                        if is_val_focused { rgb(0xffffff) } else { normal },
                    ))
                    .child(
                        Button::new(ElementId::NamedInteger("remove-var".into(), idx as u64))
                            .label("✕")
                            .xsmall()
                            .ghost()
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(
                                    this,
                                    RequestEditorAction::RemoveVariable(idx),
                                    window,
                                    cx,
                                );
                            })),
                    )
            }))
    }

}
