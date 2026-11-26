use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::ActiveInputTarget;
use crate::input_registry::{InputKey, InputRegistry};
use aura_core::model::AuraRequest;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::Sizable;

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_params_panel<V: 'static>(
        request: Option<&AuraRequest>,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let params = request.map(|r| r.params.as_slice()).unwrap_or(&[]);

        div()
            .flex()
            .flex_col()
            .gap_3()
            // Title + Action row
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
                                    .child("QUERY PARAMETERS"),
                            )
                            .child(
                                Tag::secondary()
                                    .xsmall()
                                    .child(format!("{}", params.len())),
                            ),
                    )
                    .child(
                        Button::new("add-param-btn")
                            .label("+ Add Parameter")
                            .primary()
                            .xsmall()
                            .rounded(ButtonRounded::Small)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::AddParamDirect, window, cx);
                            })),
                    ),
            )
            // Empty state or table
            .when(params.is_empty(), |el| {
                el.child(
                    Empty::new()
                        .header(EmptyHeader::new().description(
                            EmptyDescription::new().child("No query parameters attached to URL."),
                        ))
                        .child(
                            Button::new("btn-add-first-param")
                                .label("+ Add First Parameter")
                                .secondary()
                                .small()
                                .rounded(ButtonRounded::Medium)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_action(
                                        this,
                                        RequestEditorAction::AddParamDirect,
                                        window,
                                        cx,
                                    );
                                })),
                        ),
                )
            })
            .children(params.iter().enumerate().map(|(idx, param)| {
                let enabled = param.enabled;
                let key = param.key.clone();
                let val = param.value.clone();
                let is_key_focused =
                    matches!(active_input, Some(ActiveInputTarget::ParamKey(i)) if *i == idx);
                let is_val_focused =
                    matches!(active_input, Some(ActiveInputTarget::ParamValue(i)) if *i == idx);
                let view_entity = cx.entity().clone();

                div()
                    .id(ElementId::NamedInteger("param-row".into(), idx as u64))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1p5()
                    .rounded_md()
                    .bg(if enabled {
                        rgb(0x272729)
                    } else {
                        rgb(0x202022)
                    })
                    .border_1()
                    .border_color(if is_key_focused || is_val_focused {
                        rgb(0x0066cc)
                    } else {
                        border
                    })
                    // Checkbox
                    .child(
                        Checkbox::new(ElementId::NamedInteger("toggle-param".into(), idx as u64))
                            .checked(enabled)
                            .small()
                            .on_click(move |_, window, app| {
                                let _ = view_entity.update(app, |this, cx| {
                                    on_action(
                                        this,
                                        RequestEditorAction::ToggleParam(idx),
                                        window,
                                        cx,
                                    );
                                });
                            }),
                    )
                    // Inline Editable Key Field
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("edit-param-key".into(), idx as u64),
                        inputs.get_input(&InputKey::ParamKey(idx)),
                        &key,
                        is_key_focused,
                        false,
                        rgb(0x222224),
                        rgb(0x38383a),
                        if is_key_focused {
                            rgb(0xffffff)
                        } else if enabled {
                            normal
                        } else {
                            muted
                        },
                    ))
                    .child(div().text_xs().text_color(muted).child("="))
                    // Inline Editable Value Field
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("edit-param-val".into(), idx as u64),
                        inputs.get_input(&InputKey::ParamValue(idx)),
                        &val,
                        is_val_focused,
                        false,
                        rgb(0x222224),
                        rgb(0x38383a),
                        if is_val_focused {
                            rgb(0xffffff)
                        } else if enabled {
                            normal
                        } else {
                            muted
                        },
                    ))
                    // Delete Button
                    .child(
                        div()
                            .id(ElementId::NamedInteger("del-param-btn".into(), idx as u64))
                            .px_1p5()
                            .py_0p5()
                            .rounded_sm()
                            .bg(rgb(0x333333))
                            .hover(|s| s.bg(rgb(0xe74c3c)).text_color(rgb(0xffffff)))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(muted)
                            .child("✕")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::RemoveParam(idx), window, cx);
                            })),
                    )
            }))
    }

    // =========================================================================
    // Request Body Panel (Full-Height Direct Monospace Editor + Actions)
    // =========================================================================

}
