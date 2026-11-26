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
    pub(super) fn render_headers_panel<V: 'static>(
        request: Option<&AuraRequest>,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let headers = request.map(|r| r.headers.as_slice()).unwrap_or(&[]);

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
                            .label("+ Add Header")
                            .primary()
                            .xsmall()
                            .rounded(ButtonRounded::Small)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::AddHeaderDirect, window, cx);
                            })),
                    ),
            )
            // Presets row
            .child(
                div()
                    .flex()
                    .items_center()
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
            // Empty state or table
            .when(headers.is_empty(), |el| {
                el.child(
                    Empty::new()
                        .header(EmptyHeader::new().description(
                            EmptyDescription::new().child(
                                "No custom headers defined. Click '+ Add Header' or pick a preset.",
                            ),
                        ))
                        .child(
                            Button::new("btn-add-first-header")
                                .label("+ Add First Header")
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

                div()
                    .id(ElementId::NamedInteger("header-row".into(), idx as u64))
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
                    // Enabled Checkbox
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
                    // Inline Editable Key Field
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("edit-header-key".into(), idx as u64),
                        inputs.get_input(&InputKey::HeaderKey(idx)),
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
                    .child(div().text_xs().text_color(muted).child(":"))
                    // Inline Editable Value Field
                    .child(Self::render_inline_input(
                        ElementId::NamedInteger("edit-header-val".into(), idx as u64),
                        inputs.get_input(&InputKey::HeaderValue(idx)),
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
                            .id(ElementId::NamedInteger("del-header-btn".into(), idx as u64))
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
            .secondary()
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

    // =========================================================================
    // Query Parameters Panel (Modal-Free Interactive Key-Value Grid)
    // =========================================================================

}
