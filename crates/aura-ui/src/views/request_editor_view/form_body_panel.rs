use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::ActiveInputTarget;
use crate::input_registry::{InputKey, InputRegistry};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::Sizable;

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_form_urlencoded_editor<V: 'static>(
        items: &[aura_core::model::ParamItem],
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
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(muted)
                            .child("X-WWW-FORM-URLENCODED FIELDS"),
                    )
                    .child(
                        div()
                            .id("add-form-field-btn")
                            .px_2()
                            .py_0p5()
                            .rounded_sm()
                            .bg(rgb(0x0066cc))
                            .hover(|s| s.bg(rgb(0x0071e3)))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(rgb(0xffffff))
                            .child("+ Add Field")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(
                                    this,
                                    RequestEditorAction::AddFormParamDirect,
                                    window,
                                    cx,
                                );
                            })),
                    ),
            )
            .when(items.is_empty(), |el| {
                el.child(
                    div()
                        .p_4()
                        .rounded_md()
                        .bg(rgb(0x222224))
                        .border_1()
                        .border_color(border)
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted)
                                .child("No form fields defined. Click '+ Add Field' to start."),
                        ),
                )
            })
            .children(items.iter().enumerate().map(|(idx, item)| {
                let enabled = item.enabled;
                let k = item.key.clone();
                let v = item.value.clone();
                let is_key_focused =
                    matches!(active_input, Some(ActiveInputTarget::FormParamKey(i)) if *i == idx);
                let is_val_focused =
                    matches!(active_input, Some(ActiveInputTarget::FormParamValue(i)) if *i == idx);

                div()
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
                    .child({
                        let view_entity = cx.entity().clone();
                        Checkbox::new(ElementId::NamedInteger(
                            "toggle-form-param".into(),
                            idx as u64,
                        ))
                        .checked(enabled)
                        .small()
                        .on_click(move |_, window, app| {
                            let _ = view_entity.update(app, |this, cx| {
                                on_action(
                                    this,
                                    RequestEditorAction::ToggleFormParam(idx),
                                    window,
                                    cx,
                                );
                            });
                        })
                    })
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("edit-form-key".into(), idx as u64),
                        inputs.get_input(&InputKey::FormParamKey(idx)),
                        &k,
                        is_key_focused,
                        false,
                        rgb(0x222224),
                        rgb(0x38383a),
                        if is_key_focused { rgb(0xffffff) } else { normal },
                    ))
                    .child(div().text_xs().text_color(muted).child("="))
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("edit-form-val".into(), idx as u64),
                        inputs.get_input(&InputKey::FormParamValue(idx)),
                        &v,
                        is_val_focused,
                        false,
                        rgb(0x222224),
                        rgb(0x38383a),
                        if is_val_focused { rgb(0xffffff) } else { normal },
                    ))
                    .child(
                        div()
                            .id(ElementId::NamedInteger("del-form-param".into(), idx as u64))
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
                                on_action(
                                    this,
                                    RequestEditorAction::RemoveFormParam(idx),
                                    window,
                                    cx,
                                );
                            })),
                    )
            }))
    }

    pub(super) fn render_multipart_editor<V: 'static>(
        parts: &[aura_core::model::MultipartPart],
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(muted)
                            .child("MULTIPART/FORM-DATA FIELDS"),
                    )
                    .child(
                        div()
                            .id("add-multipart-field-btn")
                            .px_2()
                            .py_0p5()
                            .rounded_sm()
                            .bg(rgb(0x0066cc))
                            .hover(|s| s.bg(rgb(0x0071e3)))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(rgb(0xffffff))
                            .child("+ Add Part")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(
                                    this,
                                    RequestEditorAction::AddMultipartDirect,
                                    window,
                                    cx,
                                );
                            })),
                    ),
            )
            .when(parts.is_empty(), |el| {
                el.child(
                    div()
                        .p_4()
                        .rounded_md()
                        .bg(rgb(0x222224))
                        .border_1()
                        .border_color(border)
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted)
                                .child("No multipart fields defined. Click '+ Add Part' to start."),
                        ),
                )
            })
            .children(parts.iter().enumerate().map(|(idx, part)| {
                let name = part.name.clone();
                let is_file = part.file_path.is_some();
                let display_val = part
                    .value
                    .clone()
                    .or_else(|| part.file_path.clone())
                    .unwrap_or_default();

                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(rgb(0x272729))
                    .border_1()
                    .border_color(border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(if is_file {
                                        rgb(0x8e44ad)
                                    } else {
                                        rgb(0x0066cc)
                                    })
                                    .text_xs()
                                    .text_color(rgb(0xffffff))
                                    .child(if is_file { "FILE" } else { "TEXT" }),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xffffff))
                                    .child(name),
                            )
                            .child(div().text_xs().text_color(muted).child("="))
                            .child(div().text_xs().text_color(normal).child(display_val)),
                    )
                    .child(
                        div()
                            .id(ElementId::NamedInteger("del-multipart".into(), idx as u64))
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
                                on_action(
                                    this,
                                    RequestEditorAction::RemoveMultipart(idx),
                                    window,
                                    cx,
                                );
                            })),
                    )
            }))
    }

    // =========================================================================
    // Authentication Panel (Modal-Free Full Space Inputs + Previews)
    // =========================================================================

}
