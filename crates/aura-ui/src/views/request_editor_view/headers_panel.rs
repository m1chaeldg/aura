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
    pub(super) fn render_headers_panel<V: 'static>(
        request: Option<&AuraRequest>,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let headers = request.map(|r| r.headers.as_slice()).unwrap_or(&[]);
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
                                    .child("REQUEST HEADERS"),
                            )
                            .child(
                                Tag::secondary()
                                    .xsmall()
                                    .child(format!("{}", headers.len())),
                            ),
                    )
                    .child(
                        Button::new("add-header-btn")
                            .icon(icon(IconName::Plus))
                            .label("Add Header")
                            .primary()
                            .xsmall()
                            .rounded(ButtonRounded::Small)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::AddHeaderDirect, window, cx);
                            })),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .child(div().text_xs().text_color(muted).child("Presets:"))
                    .child(Self::render_preset_chip(
                        "Content-Type: JSON",
                        "Content-Type",
                        "application/json",
                        cx,
                        on_action,
                    ))
                    .child(Self::render_preset_chip(
                        "Accept: */*",
                        "Accept",
                        "*/*",
                        cx,
                        on_action,
                    ))
                    .child(Self::render_preset_chip(
                        "User-Agent: Aura",
                        "User-Agent",
                        "Aura-Client/0.1.0",
                        cx,
                        on_action,
                    )),
            )
            .when(headers.is_empty(), |el| {
                el.child(
                    Empty::new()
                        .header(EmptyHeader::new().description(
                            EmptyDescription::new().child(
                                "No custom headers defined. Click 'Add Header' or pick a preset.",
                            ),
                        ))
                        .child(
                            Button::new("btn-add-first-header")
                                .icon(icon(IconName::Plus))
                                .label("Add First Header")
                                .secondary()
                                .small()
                                .rounded(ButtonRounded::Medium)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_action(
                                        this,
                                        RequestEditorAction::AddHeaderDirect,
                                        window,
                                        cx,
                                    );
                                })),
                        ),
                )
            })
            .children(headers.iter().enumerate().map(|(idx, header)| {
                let enabled = header.enabled;
                let key = header.key.clone();
                let val = header.value.clone();
                let is_key_focused =
                    matches!(active_input, Some(ActiveInputTarget::HeaderKey(i)) if *i == idx);
                let is_val_focused =
                    matches!(active_input, Some(ActiveInputTarget::HeaderValue(i)) if *i == idx);
                let view_entity = cx.entity().clone();

                h_flex()
                    .id(ElementId::NamedInteger("header-row".into(), idx as u64))
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
                        Checkbox::new(ElementId::NamedInteger("toggle-header".into(), idx as u64))
                            .checked(enabled)
                            .small()
                            .on_click(move |_, window, app| {
                                let _ = view_entity.update(app, |this, cx| {
                                    on_action(
                                        this,
                                        RequestEditorAction::ToggleHeader(idx),
                                        window,
                                        cx,
                                    );
                                });
                            }),
                    )
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("edit-header-key".into(), idx as u64),
                        inputs.get_input(&InputKey::HeaderKey(idx)),
                        &key,
                        is_key_focused,
                        false,
                        field_bg,
                        border,
                        if enabled { normal } else { muted },
                    ))
                    .child(div().text_xs().text_color(muted).child(":"))
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("edit-header-val".into(), idx as u64),
                        inputs.get_input(&InputKey::HeaderValue(idx)),
                        &val,
                        is_val_focused,
                        false,
                        field_bg,
                        border,
                        if enabled { normal } else { muted },
                    ))
                    .child(
                        Button::new(ElementId::NamedInteger("del-header-btn".into(), idx as u64))
                            .icon(icon(IconName::X))
                            .xsmall()
                            .ghost()
                            .tooltip("Remove header")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::RemoveHeader(idx), window, cx);
                            })),
                    )
            }))
    }

    pub(super) fn render_preset_chip<V: 'static>(
        label: &'static str,
        key: &'static str,
        value: &'static str,
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
                    RequestEditorAction::AddPresetHeader(key, value),
                    window,
                    cx,
                );
            }))
    }
}
