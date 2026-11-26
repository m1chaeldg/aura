use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::ActiveInputTarget;
use crate::input_registry::{InputKey, InputRegistry};
use crate::templating_ui::render_templated_text;
use crate::theme::{icon, subtle_surface};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_form_urlencoded_editor<V: 'static>(
        items: &[aura_core::model::ParamItem],
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let row_bg = subtle_surface(cx);
        let focus_border = cx.theme().primary;
        let field_bg = cx.theme().background;
        let radius = cx.theme().radius;

        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(muted)
                            .child("X-WWW-FORM-URLENCODED FIELDS"),
                    )
                    .child(
                        Button::new("add-form-field-btn")
                            .icon(icon(IconName::Plus))
                            .label("Add Field")
                            .primary()
                            .xsmall()
                            .rounded(ButtonRounded::Small)
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
                    Empty::new().header(EmptyHeader::new().description(
                        EmptyDescription::new()
                            .child("No form fields defined. Click 'Add Field' to start."),
                    )),
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
                let view_entity = cx.entity().clone();

                h_flex()
                    .gap_2()
                    .px_3()
                    .py_1p5()
                    .rounded(radius)
                    .bg(row_bg)
                    .border_1()
                    .border_color(if is_key_focused || is_val_focused {
                        focus_border
                    } else {
                        border
                    })
                    .child(
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
                        }),
                    )
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("edit-form-key".into(), idx as u64),
                        inputs.get_input(&InputKey::FormParamKey(idx)),
                        &k,
                        is_key_focused,
                        false,
                        field_bg,
                        border,
                        if enabled { normal } else { muted },
                    ))
                    .child(div().text_xs().text_color(muted).child("="))
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("edit-form-val".into(), idx as u64),
                        inputs.get_input(&InputKey::FormParamValue(idx)),
                        &v,
                        is_val_focused,
                        false,
                        field_bg,
                        border,
                        if enabled { normal } else { muted },
                    ))
                    .child(
                        Button::new(ElementId::NamedInteger("del-form-param".into(), idx as u64))
                            .icon(icon(IconName::X))
                            .xsmall()
                            .ghost()
                            .tooltip("Remove field")
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
        inputs: &InputRegistry,
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let tpl_ctx = inputs.template_context().borrow().clone();
        let row_bg = subtle_surface(cx);
        let radius = cx.theme().radius;

        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(muted)
                            .child("MULTIPART/FORM-DATA FIELDS"),
                    )
                    .child(
                        Button::new("add-multipart-field-btn")
                            .icon(icon(IconName::Plus))
                            .label("Add Part")
                            .primary()
                            .xsmall()
                            .rounded(ButtonRounded::Small)
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
                    Empty::new().header(EmptyHeader::new().description(
                        EmptyDescription::new()
                            .child("No multipart fields defined. Click 'Add Part' to start."),
                    )),
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

                h_flex()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .rounded(radius)
                    .bg(row_bg)
                    .border_1()
                    .border_color(border)
                    .child(
                        h_flex()
                            .gap_2()
                            .min_w_0()
                            .child(if is_file {
                                Tag::warning().outline().xsmall().child("FILE")
                            } else {
                                Tag::info().outline().xsmall().child("TEXT")
                            })
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(normal)
                                    .child(name),
                            )
                            .child(div().text_xs().text_color(muted).child("="))
                            .child(
                                div().text_xs().min_w_0().child(render_templated_text(
                                    ElementId::NamedInteger("multipart-val".into(), idx as u64),
                                    &display_val,
                                    &tpl_ctx,
                                    normal,
                                    cx,
                                )),
                            ),
                    )
                    .child(
                        Button::new(ElementId::NamedInteger("del-multipart".into(), idx as u64))
                            .icon(icon(IconName::X))
                            .xsmall()
                            .ghost()
                            .tooltip("Remove part")
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
}
