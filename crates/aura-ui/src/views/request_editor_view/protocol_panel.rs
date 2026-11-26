//! Protocol-specific request panels: WebSocket, SSE, gRPC, and SOAP helpers.

use super::{RequestEditorAction, RequestEditorView};
use crate::input_registry::{InputKey, InputRegistry};
use crate::templating_ui::render_template_token;
use crate::theme::{icon, subtle_surface};
use aura_core::grpc::ProtoSchema;
use aura_core::model::AuraRequest;
use aura_core::soap::SoapVersion;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::input::{Editor, Input};
use gpui_kit::component::tag::{Tag, TagVariant};
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};
use gpui_kit::base::Disableable as _;

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_websocket_panel<V: 'static>(
        request: Option<&AuraRequest>,
        is_connected: bool,
        ws_composer_text: &str,
        inputs: &InputRegistry,
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let templates = request.map(|r| r.ws_templates.clone()).unwrap_or_default();

        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .p_2()
                    .rounded(cx.theme().radius)
                    .bg(subtle_surface(cx))
                    .border_1()
                    .border_color(border)
                    .child(
                        h_flex()
                            .gap_2()
                            .child(if is_connected {
                                Tag::new()
                                    .with_variant(TagVariant::Success)
                                    .xsmall()
                                    .child("Connected")
                            } else {
                                Tag::secondary().xsmall().child("Disconnected")
                            })
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted)
                                    .child("Persistent ws:// / wss:// connection"),
                            ),
                    )
                    .child(
                        Button::new("ws-toggle-conn-btn")
                            .icon(icon(if is_connected {
                                IconName::Square
                            } else {
                                IconName::Zap
                            }))
                            .label(if is_connected { "Disconnect" } else { "Connect" })
                            .xsmall()
                            .rounded(ButtonRounded::Small)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::ToggleWsConnection, window, cx);
                            })),
                    ),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child("MESSAGE PAYLOAD"),
                            )
                            .child(
                                Button::new("ws-send-msg-btn")
                                    .icon(icon(IconName::Send))
                                    .label("Send Message")
                                    .xsmall()
                                    .primary()
                                    .rounded(ButtonRounded::Small)
                                    .disabled(!is_connected)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            RequestEditorAction::SendWsMessage,
                                            window,
                                            cx,
                                        );
                                    })),
                            ),
                    )
                    .child(if let Some(editor) = inputs.get_editor(&InputKey::WsMessageInput) {
                        div()
                            .h(px(160.0))
                            .w_full()
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(border)
                            .overflow_hidden()
                            .child(Editor::new(editor).h(px(160.0)).w_full())
                            .into_any_element()
                    } else {
                        div()
                            .p_2()
                            .text_xs()
                            .text_color(normal)
                            .child(ws_composer_text.to_string())
                            .into_any_element()
                    }),
            )
            .child(
                v_flex()
                    .gap_2()
                    .p_2()
                    .rounded(cx.theme().radius)
                    .border_1()
                    .border_color(border)
                    .bg(subtle_surface(cx))
                    .child(
                        h_flex()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child(format!("SAVED MESSAGE TEMPLATES ({})", templates.len())),
                            )
                            .child(
                                h_flex()
                                    .gap_1p5()
                                    .when_some(
                                        inputs.get_input(&InputKey::WsTemplateName),
                                        |el, inp| {
                                            el.child(div().w(px(160.0)).child(
                                                Input::new(inp).xsmall().token(render_template_token),
                                            ))
                                        },
                                    )
                                    .child(
                                        Button::new("ws-save-tpl-btn")
                                            .icon(icon(IconName::Plus))
                                            .label("Save Template")
                                            .xsmall()
                                            .secondary()
                                            .rounded(ButtonRounded::Small)
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                on_action(
                                                    this,
                                                    RequestEditorAction::SaveWsTemplate,
                                                    window,
                                                    cx,
                                                );
                                            })),
                                    ),
                            ),
                    )
                    .when(!templates.is_empty(), |el| {
                        el.child(h_flex().gap_1p5().flex_wrap().children(
                            templates.into_iter().enumerate().map(|(idx, tpl)| {
                                let id_load = tpl.id.clone();
                                let id_del = tpl.id.clone();
                                h_flex()
                                    .gap_1()
                                    .px_2()
                                    .py_0p5()
                                    .rounded(cx.theme().radius)
                                    .border_1()
                                    .border_color(border)
                                    .bg(cx.theme().background)
                                    .child(
                                        Button::new(ElementId::NamedInteger(
                                            "ws-tpl-load".into(),
                                            idx as u64,
                                        ))
                                        .label(tpl.name)
                                        .xsmall()
                                        .ghost()
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(
                                                this,
                                                RequestEditorAction::LoadWsTemplate(id_load.clone()),
                                                window,
                                                cx,
                                            );
                                        })),
                                    )
                                    .child(
                                        Button::new(ElementId::NamedInteger(
                                            "ws-tpl-del".into(),
                                            idx as u64,
                                        ))
                                        .icon(icon(IconName::X))
                                        .xsmall()
                                        .ghost()
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(
                                                this,
                                                RequestEditorAction::DeleteWsTemplate(id_del.clone()),
                                                window,
                                                cx,
                                            );
                                        })),
                                    )
                            }),
                        ))
                    }),
            )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_grpc_panel<V: 'static>(
        request: Option<&AuraRequest>,
        schema: Option<&ProtoSchema>,
        inputs: &InputRegistry,
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let grpc_cfg = request.and_then(|r| r.grpc.as_ref());
        let selected_svc = grpc_cfg.map(|g| g.service.as_str()).unwrap_or("");
        let selected_method = grpc_cfg.map(|g| g.method.as_str()).unwrap_or("");
        let proto_path = grpc_cfg.and_then(|g| g.proto_path.as_deref());

        let selected_cardinality = schema
            .and_then(|s| {
                s.services
                    .iter()
                    .find(|svc| svc.full_name == selected_svc || svc.name == selected_svc)
                    .and_then(|svc| svc.methods.iter().find(|m| m.name == selected_method))
            })
            .map(|m| m.cardinality().as_str())
            .unwrap_or("Unary");

        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .flex_wrap()
                    .p_2()
                    .rounded(cx.theme().radius)
                    .bg(subtle_surface(cx))
                    .border_1()
                    .border_color(border)
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Tag::new()
                                    .with_variant(TagVariant::Color(
                                        gpui_kit::component::ColorName::Purple,
                                    ))
                                    .xsmall()
                                    .child(selected_cardinality),
                            )
                            .when_some(proto_path, |el, p| {
                                el.child(div().text_xs().text_color(muted).truncate().child(p.to_string()))
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_1p5()
                            .child(
                                Button::new("grpc-load-proto-btn")
                                    .icon(icon(IconName::FileText))
                                    .label("Load .proto")
                                    .xsmall()
                                    .outline()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            RequestEditorAction::LoadProtoFile,
                                            window,
                                            cx,
                                        );
                                    })),
                            )
                            .child(
                                Button::new("grpc-reflect-btn")
                                    .icon(icon(IconName::RefreshCw))
                                    .label("Server Reflection")
                                    .xsmall()
                                    .outline()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            RequestEditorAction::ReflectGrpcServices,
                                            window,
                                            cx,
                                        );
                                    })),
                            )
                            .child(
                                Button::new("grpc-sample-btn")
                                    .icon(icon(IconName::Sparkles))
                                    .label("Sample JSON")
                                    .xsmall()
                                    .ghost()
                                    .rounded(ButtonRounded::Small)
                                    .disabled(schema.is_none())
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            RequestEditorAction::GenerateGrpcSample,
                                            window,
                                            cx,
                                        );
                                    })),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(div().text_xs().text_color(muted).child("SERVICE"))
                            .when_some(inputs.get_input(&InputKey::GrpcService), |el, inp| {
                                el.child(Input::new(inp).xsmall().token(render_template_token))
                            }),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(div().text_xs().text_color(muted).child("METHOD"))
                            .when_some(inputs.get_input(&InputKey::GrpcMethod), |el, inp| {
                                el.child(Input::new(inp).xsmall().token(render_template_token))
                            }),
                    ),
            )
            .when_some(schema, |el, sch| {
                let services = sch.services.clone();
                let current_methods = sch
                    .services
                    .iter()
                    .find(|s| s.full_name == selected_svc || s.name == selected_svc)
                    .or_else(|| sch.services.first())
                    .map(|s| s.methods.clone())
                    .unwrap_or_default();
                el.child(
                    v_flex()
                        .gap_1p5()
                        .when(!services.is_empty(), |el| {
                            el.child(h_flex().gap_1().flex_wrap().children(
                                services.into_iter().enumerate().map(|(idx, svc)| {
                                    let is_sel = svc.full_name == selected_svc;
                                    let full_n = svc.full_name.clone();
                                    let btn = Button::new(ElementId::NamedInteger(
                                        "grpc-svc-pick".into(),
                                        idx as u64,
                                    ))
                                    .label(svc.name)
                                    .xsmall()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            RequestEditorAction::SelectGrpcService(full_n.clone()),
                                            window,
                                            cx,
                                        );
                                    }));
                                    if is_sel { btn.primary() } else { btn.outline() }
                                }),
                            ))
                        })
                        .when(!current_methods.is_empty(), |el| {
                            el.child(h_flex().gap_1().flex_wrap().children(
                                current_methods.into_iter().enumerate().map(|(idx, m)| {
                                    let is_sel = m.name == selected_method;
                                    let m_name = m.name.clone();
                                    let label = format!("{} ({})", m.name, m.cardinality().as_str());
                                    let btn = Button::new(ElementId::NamedInteger(
                                        "grpc-mth-pick".into(),
                                        idx as u64,
                                    ))
                                    .label(label)
                                    .xsmall()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            RequestEditorAction::SelectGrpcMethod(m_name.clone()),
                                            window,
                                            cx,
                                        );
                                    }));
                                    if is_sel { btn.secondary() } else { btn.ghost() }
                                }),
                            ))
                        }),
                )
            })
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted)
                            .child("MESSAGE JSON (or JSON array / lines for client/bidi streaming)"),
                    )
                    .child(if let Some(editor) = inputs.get_editor(&InputKey::GrpcPayload) {
                        div()
                            .h(px(210.0))
                            .w_full()
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(border)
                            .overflow_hidden()
                            .child(Editor::new(editor).h(px(210.0)).w_full())
                            .into_any_element()
                    } else {
                        div()
                            .text_xs()
                            .text_color(normal)
                            .child(grpc_cfg.map(|g| g.payload.clone()).unwrap_or_default())
                            .into_any_element()
                    }),
            )
    }

    pub(super) fn render_soap_helper_bar<V: 'static>(
        inputs: &InputRegistry,
        border: Hsla,
        muted: Hsla,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        v_flex()
            .gap_2()
            .p_2()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(border)
            .bg(subtle_surface(cx))
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(muted)
                            .child("SOAP / WSDL HELPERS"),
                    )
                    .child(
                        h_flex()
                            .gap_1p5()
                            .child(
                                Button::new("soap-wrap-11-btn")
                                    .label("Wrap SOAP 1.1")
                                    .xsmall()
                                    .outline()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            RequestEditorAction::WrapSoapEnvelope(SoapVersion::Soap11),
                                            window,
                                            cx,
                                        );
                                    })),
                            )
                            .child(
                                Button::new("soap-wrap-12-btn")
                                    .label("Wrap SOAP 1.2")
                                    .xsmall()
                                    .outline()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            RequestEditorAction::WrapSoapEnvelope(SoapVersion::Soap12),
                                            window,
                                            cx,
                                        );
                                    })),
                            )
                            .child(
                                Button::new("soap-import-wsdl-btn")
                                    .icon(icon(IconName::FolderOpen))
                                    .label("Import WSDL")
                                    .xsmall()
                                    .secondary()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            RequestEditorAction::OpenWsdlImport,
                                            window,
                                            cx,
                                        );
                                    })),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .when_some(inputs.get_input(&InputKey::SoapOperation), |el, inp| {
                        el.child(div().flex_1().child(Input::new(inp).xsmall()))
                    })
                    .when_some(inputs.get_input(&InputKey::SoapNamespace), |el, inp| {
                        el.child(div().flex_1().child(Input::new(inp).xsmall()))
                    })
                    .when_some(inputs.get_input(&InputKey::SoapAction), |el, inp| {
                        el.child(div().flex_1().child(Input::new(inp).xsmall()))
                    }),
            )
    }
}
