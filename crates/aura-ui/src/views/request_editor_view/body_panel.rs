use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::ActiveInputTarget;
use crate::input_registry::{InputKey, InputRegistry};
use crate::theme::{icon, subtle_surface};
use aura_core::model::{AuraRequest, BodyType};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Sizable, ThemeStyled as _, h_flex, v_flex};

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_body_panel<V: 'static>(
        request: Option<&AuraRequest>,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        is_format_dropdown_open: bool,
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
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

        fn format_display(fmt: &str) -> &str {
            match fmt {
                "FormUrlEncoded" | "Form" => "Form (URL-Encoded)",
                "Multipart" => "Multipart Form",
                other => other,
            }
        }
        let current_display = format_display(active_type);

        let format_dropdown = div()
            .relative()
            .child(
                Button::new("body-format-dropdown-btn")
                    .label(format!("Body: {}", current_display))
                    .dropdown_caret(true)
                    .xsmall()
                    .outline()
                    .rounded(ButtonRounded::Small)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        on_action(this, RequestEditorAction::ToggleBodyFormatDropdown, window, cx);
                    })),
            )
            .when(is_format_dropdown_open, |el| {
                let primary_accent = cx.theme().primary;
                let popover_fg = cx.theme().popover_foreground;
                let radius = cx.theme().radius;
                let menu = div()
                    .id("body-format-dropdown-menu")
                    .popover_style(cx)
                    .absolute()
                    .top_full()
                    .left_0()
                    .mt_1()
                    .w(px(200.0))
                    .p_1()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .on_mouse_down_out(cx.listener(move |this, _, window, cx| {
                        on_action(
                            this,
                            RequestEditorAction::DismissBodyFormatDropdown,
                            window,
                            cx,
                        );
                    }))
                    .children(FORMATS.iter().enumerate().map(|(idx, &fmt)| {
                        let is_selected = match fmt {
                            "FormUrlEncoded" => {
                                active_type == "FormUrlEncoded" || active_type == "Form"
                            }
                            _ => active_type == fmt,
                        };
                        ListItem::new(ElementId::NamedInteger(
                            "body-format-opt".into(),
                            idx as u64,
                        ))
                        .selected(is_selected)
                        .px_2()
                        .py_1()
                        .rounded(radius)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            on_action(this, RequestEditorAction::SetBodyType(fmt), window, cx);
                        }))
                        .child(
                            h_flex()
                                .justify_between()
                                .w_full()
                                .text_xs()
                                .child(
                                    div()
                                        .text_color(if is_selected {
                                            popover_fg
                                        } else {
                                            normal
                                        })
                                        .child(format_display(fmt)),
                                )
                                .when(is_selected, |el| {
                                    el.child(
                                        icon(IconName::Check)
                                            .xsmall()
                                            .text_color(primary_accent),
                                    )
                                }),
                        )
                    }));
                el.child(deferred(menu).with_priority(1))
            });

        v_flex()
            .gap_3()
            .child(h_flex().justify_between().gap_2().child(format_dropdown))
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
                    Self::render_multipart_editor(parts, inputs, border, muted, normal, cx, on_action)
                }
                Some(BodyType::Binary {
                    file_path,
                    content_type,
                }) => {
                    let ct = content_type
                        .as_deref()
                        .unwrap_or("application/octet-stream");
                    v_flex()
                        .gap_2()
                        .p_4()
                        .rounded(cx.theme().radius)
                        .bg(subtle_surface(cx))
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
                            h_flex()
                                .justify_between()
                                .p_2()
                                .bg(cx.theme().background)
                                .border_1()
                                .border_color(border)
                                .rounded(cx.theme().radius)
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
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let is_focused = matches!(active_input, Some(ActiveInputTarget::BodyText));
        let byte_len = content.len();

        v_flex()
            .gap_2()
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
                                    .child(label.to_string()),
                            )
                            .child(
                                Tag::secondary()
                                    .xsmall()
                                    .child(format!("{byte_len} B")),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .when(is_json, |el| {
                                el.child(
                                    Button::new("format-json-btn")
                                        .icon(icon(IconName::Zap))
                                        .label("Format")
                                        .xsmall()
                                        .success()
                                        .rounded(ButtonRounded::Small)
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
                                Button::new("clear-body-btn")
                                    .label("Clear")
                                    .xsmall()
                                    .ghost()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::ClearBody, window, cx);
                                    })),
                            ),
                    ),
            )
            .child(Self::render_inline_editor(
                "direct-body-editor",
                inputs.get_editor(&InputKey::BodyText),
                content,
                px(260.0),
                is_focused,
                border,
                normal,
            ))
    }
}
