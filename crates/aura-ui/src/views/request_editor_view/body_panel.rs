use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::ActiveInputTarget;
use crate::input_registry::{InputKey, InputRegistry};
use aura_core::model::{AuraRequest, BodyType};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader};
use gpui_kit::component::Sizable;

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_body_panel<V: 'static>(
        request: Option<&AuraRequest>,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let body = request.map(|r| &r.body);
        let active_type = body.map(|b| b.type_name()).unwrap_or("None");
        const FORMATS: [&str; 8] = [
            "None",
            "JSON",
            "XML",
            "Text",
            "Raw",
            "FormUrlEncoded",
            "Multipart",
            "Binary",
        ];

        div()
            .flex()
            .flex_col()
            .gap_3()
            // Format Selector Bar
            .child(div().flex().items_center().gap_1().flex_wrap().children(
                FORMATS.iter().copied().map(|fmt| {
                    let is_active = match fmt {
                        "FormUrlEncoded" => {
                            active_type == "FormUrlEncoded" || active_type == "Form"
                        }
                        _ => active_type == fmt,
                    };
                    let display_name = match fmt {
                        "FormUrlEncoded" => "Form (URL-Encoded)",
                        "Multipart" => "Multipart Form",
                        other => other,
                    };
                    let btn = Button::new(fmt)
                        .label(display_name)
                        .xsmall()
                        .rounded(ButtonRounded::Small)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            on_action(this, RequestEditorAction::SetBodyType(fmt), window, cx);
                        }));
                    if is_active {
                        btn.primary()
                    } else {
                        btn.secondary()
                    }
                }),
            ))
            // Body Content based on Format
            .child(match body {
                Some(BodyType::None) | None => div().child(
                    Empty::new().header(EmptyHeader::new().description(
                        EmptyDescription::new().child(
                            "This request does not have a body. Select JSON, Form, or Raw above to send payload data.",
                        ),
                    )),
                ),
                Some(BodyType::Json(content)) => Self::render_full_height_body_editor(
                    "JSON (application/json)",
                    content,
                    true,
                    active_input,
                    inputs,
                    border,
                    muted,
                    normal,
                    cx,
                    on_action,
                ),
                Some(BodyType::Xml(content)) => Self::render_full_height_body_editor(
                    "XML (application/xml)",
                    content,
                    false,
                    active_input,
                    inputs,
                    border,
                    muted,
                    normal,
                    cx,
                    on_action,
                ),
                Some(BodyType::Text(content)) => Self::render_full_height_body_editor(
                    "Text (text/plain)",
                    content,
                    false,
                    active_input,
                    inputs,
                    border,
                    muted,
                    normal,
                    cx,
                    on_action,
                ),
                Some(BodyType::Raw {
                    content,
                    content_type,
                }) => {
                    let label =
                        format!("Raw ({})", content_type.as_deref().unwrap_or("text/plain"));
                    Self::render_full_height_body_editor(
                        &label,
                        content,
                        false,
                        active_input,
                        inputs,
                        border,
                        muted,
                        normal,
                        cx,
                        on_action,
                    )
                }
                Some(BodyType::FormUrlEncoded(items)) => Self::render_form_urlencoded_editor(
                    items,
                    active_input,
                    inputs,
                    border,
                    muted,
                    normal,
                    cx,
                    on_action,
                ),
                Some(BodyType::Form(map)) => {
                    let items: Vec<aura_core::model::ParamItem> = map
                        .iter()
                        .map(|(k, v)| aura_core::model::ParamItem::new(k.clone(), v.clone()))
                        .collect();
                    Self::render_form_urlencoded_editor(
                        &items,
                        active_input,
                        inputs,
                        border,
                        muted,
                        normal,
                        cx,
                        on_action,
                    )
                }
                Some(BodyType::Multipart(parts)) => {
                    Self::render_multipart_editor(parts, border, muted, normal, cx, on_action)
                }
                Some(BodyType::Binary {
                    file_path,
                    content_type,
                }) => {
                    let ct = content_type
                        .as_deref()
                        .unwrap_or("application/octet-stream");
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .p_4()
                        .rounded_md()
                        .bg(rgb(0x222224))
                        .border_1()
                        .border_color(border)
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(muted)
                                .child(format!("BINARY FILE ({ct})")),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .p_2()
                                .bg(rgb(0x1d1d1f))
                                .border_1()
                                .border_color(border)
                                .rounded_sm()
                                .child(div().text_xs().text_color(normal).child(
                                    if file_path.is_empty() {
                                        "No binary file selected".to_string()
                                    } else {
                                        file_path.clone()
                                    },
                                )),
                        )
                }
            })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_full_height_body_editor<V: 'static>(
        label: &str,
        content: &str,
        is_json: bool,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let is_focused = matches!(active_input, Some(ActiveInputTarget::BodyText));
        let byte_len = content.len();

        div()
            .flex()
            .flex_col()
            .gap_2()
            // Top Toolbar: Label + Format JSON + Clear + Size indicator
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
                                    .child(label.to_string()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x555555))
                                    .child(format!("({byte_len} bytes)")),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .when(is_json, |el| {
                                el.child(
                                    div()
                                        .id("format-json-btn")
                                        .px_2()
                                        .py_0p5()
                                        .rounded_sm()
                                        .bg(rgb(0x28a745))
                                        .hover(|s| s.bg(rgb(0x218838)))
                                        .cursor_pointer()
                                        .text_xs()
                                        .text_color(rgb(0xffffff))
                                        .child("⚡ Format / Pretty-Print")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(
                                                this,
                                                RequestEditorAction::FormatJsonBody,
                                                window,
                                                cx,
                                            );
                                        })),
                                )
                            })
                            .child(
                                div()
                                    .id("clear-body-btn")
                                    .px_2()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(rgb(0x333333))
                                    .hover(|s| s.bg(rgb(0xe74c3c)).text_color(rgb(0xffffff)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(muted)
                                    .child("Clear")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::ClearBody, window, cx);
                                    })),
                            ),
                    ),
            )
            // Full-Height Direct Monospace Textarea
            .child(Self::render_inline_textarea(
                "direct-body-editor",
                inputs.get_textarea(&InputKey::BodyText),
                content,
                px(260.0),
                is_focused,
                border,
                if is_focused { rgb(0xffffff) } else { normal },
            ))
    }

}
