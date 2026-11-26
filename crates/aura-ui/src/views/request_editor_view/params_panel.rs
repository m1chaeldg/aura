use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::ActiveInputTarget;
use crate::input_registry::{InputKey, InputRegistry};
use crate::theme::icon;
use aura_core::model::AuraRequest;
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
    pub(super) fn render_params_panel<V: 'static>(
        request: Option<&AuraRequest>,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let params = request.map(|r| r.params.as_slice()).unwrap_or(&[]);
        let row_active_bg = cx.theme().secondary.opacity(0.35);
        let row_disabled_bg = cx.theme().muted.opacity(0.2);
        let focus_border = cx.theme().primary;
        let field_bg = cx.theme().background;
        let radius = cx.theme().radius;

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
                            .icon(icon(IconName::Plus))
                            .label("Add Parameter")
                            .primary()
                            .xsmall()
                            .rounded(ButtonRounded::Small)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::AddParamDirect, window, cx);
                            })),
                    ),
            )
            .when(params.is_empty(), |el| {
                el.child(
                    Empty::new()
                        .header(EmptyHeader::new().description(
                            EmptyDescription::new().child("No query parameters attached to URL."),
                        ))
                        .child(
                            Button::new("btn-add-first-param")
                                .icon(icon(IconName::Plus))
                                .label("Add First Parameter")
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

                h_flex()
                    .id(ElementId::NamedInteger("param-row".into(), idx as u64))
                    .gap_2()
                    .px_3()
                    .py_1p5()
                    .rounded(radius)
                    .bg(if enabled {
                        row_active_bg
                    } else {
                        row_disabled_bg
                    })
                    .border_1()
                    .border_color(if is_key_focused || is_val_focused {
                        focus_border
                    } else {
                        border
                    })
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
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("edit-param-key".into(), idx as u64),
                        inputs.get_input(&InputKey::ParamKey(idx)),
                        &key,
                        is_key_focused,
                        false,
                        field_bg,
                        border,
                        if enabled { normal } else { muted },
                    ))
                    .child(div().text_xs().text_color(muted).child("="))
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("edit-param-val".into(), idx as u64),
                        inputs.get_input(&InputKey::ParamValue(idx)),
                        &val,
                        is_val_focused,
                        false,
                        field_bg,
                        border,
                        if enabled { normal } else { muted },
                    ))
                    .child(
                        Button::new(ElementId::NamedInteger("del-param-btn".into(), idx as u64))
                            .icon(icon(IconName::X))
                            .xsmall()
                            .ghost()
                            .tooltip("Remove parameter")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::RemoveParam(idx), window, cx);
                            })),
                    )
            }))
    }
}
