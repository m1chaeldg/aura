//! Streaming response panel for SSE events, WebSocket bidirectional stream, gRPC frames & SOAP Faults.

use super::{ResponsePaneAction, ResponsePaneView};
use crate::theme::{icon, subtle_surface};
use aura_core::model::AuraResponse;
use aura_core::soap::parse_soap_fault;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::base::SelectableText;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader};
use gpui_kit::component::tag::{Tag, TagVariant};
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};

impl ResponsePaneView {
    pub(super) fn render_stream_panel<V: 'static>(
        resp: &AuraResponse,
        border: Hsla,
        muted: Hsla,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, ResponsePaneAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let fg = cx.theme().foreground;
        let primary = cx.theme().primary;
        let success = cx.theme().green;
        let info = cx.theme().blue;

        let sse_count = resp.sse_events.len();
        let msg_count = resp.stream_messages.len();
        let soap_fault = parse_soap_fault(&resp.body);

        v_flex()
            .gap_2()
            .min_w_0()
            .when_some(soap_fault, |el, fault| {
                el.child(
                    v_flex()
                        .p_2()
                        .gap_1()
                        .rounded(cx.theme().radius)
                        .border_1()
                        .border_color(cx.theme().red)
                        .bg(subtle_surface(cx))
                        .child(
                            h_flex()
                                .gap_2()
                                .child(
                                    Tag::new()
                                        .with_variant(TagVariant::Danger)
                                        .xsmall()
                                        .child(format!("SOAP Fault: {}", fault.code)),
                                )
                                .child(div().text_xs().text_color(fg).child(fault.reason)),
                        )
                        .when_some(fault.detail, |el, d| {
                            el.child(div().text_xs().text_color(muted).child(d))
                        }),
                )
            })
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child(format!(
                                        "LIVE STREAM ({} events, {} messages)",
                                        sse_count, msg_count
                                    )),
                            ),
                    )
                    .when(sse_count > 0 || msg_count > 0, |el| {
                        el.child(
                            Button::new("resp-clear-stream-btn")
                                .icon(icon(IconName::Trash))
                                .label("Clear")
                                .xsmall()
                                .secondary()
                                .rounded(ButtonRounded::Small)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_action(this, ResponsePaneAction::ClearStream, window, cx);
                                })),
                        )
                    }),
            )
            .when(sse_count == 0 && msg_count == 0, |el| {
                el.child(
                    Empty::new().header(
                        EmptyHeader::new().description(
                            EmptyDescription::new().child(
                                "No stream events or messages yet. Send an SSE, WebSocket, or gRPC request to inspect events live.",
                            ),
                        ),
                    ),
                )
            })
            .when(!resp.sse_events.is_empty(), |el| {
                el.child(v_flex().gap_1p5().children(
                    resp.sse_events.iter().enumerate().map(|(idx, ev)| {
                        v_flex()
                            .p_2()
                            .gap_1()
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(border)
                            .bg(subtle_surface(cx))
                            .child(
                                h_flex()
                                    .justify_between()
                                    .child(
                                        h_flex()
                                            .gap_1p5()
                                            .child(
                                                Tag::new()
                                                    .with_variant(TagVariant::Info)
                                                    .xsmall()
                                                    .child(ev.event.clone()),
                                            )
                                            .when_some(ev.id.clone(), |el, id| {
                                                el.child(
                                                    Tag::secondary()
                                                        .xsmall()
                                                        .child(format!("id: {id}")),
                                                )
                                            }),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(muted)
                                            .child(ev.timestamp.format("%H:%M:%S%.3f").to_string()),
                                    ),
                            )
                            .child(
                                div()
                                    .font_family("monospace")
                                    .text_xs()
                                    .text_color(fg)
                                    .child(SelectableText::new(
                                        ElementId::NamedInteger("sse-ev-data".into(), idx as u64),
                                        ev.data.clone(),
                                    )),
                            )
                    }),
                ))
            })
            .when(!resp.stream_messages.is_empty(), |el| {
                el.child(v_flex().gap_1p5().children(
                    resp.stream_messages.iter().enumerate().map(|(idx, msg)| {
                        let (variant, label, accent) = match msg.direction.as_str() {
                            "out" => (TagVariant::Info, "SENT", primary),
                            "in" => (TagVariant::Success, "RECV", success),
                            _ => (TagVariant::Secondary, "SYS", info),
                        };
                        v_flex()
                            .p_2()
                            .gap_1()
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(border)
                            .bg(subtle_surface(cx))
                            .child(
                                h_flex()
                                    .justify_between()
                                    .child(
                                        h_flex()
                                            .gap_1p5()
                                            .child(
                                                Tag::new()
                                                    .with_variant(variant)
                                                    .xsmall()
                                                    .child(label),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(accent)
                                                    .child(format!("#{}", idx + 1)),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(muted)
                                            .child(msg.timestamp.format("%H:%M:%S%.3f").to_string()),
                                    ),
                            )
                            .child(
                                div()
                                    .font_family("monospace")
                                    .text_xs()
                                    .text_color(fg)
                                    .child(SelectableText::new(
                                        ElementId::NamedInteger("ws-msg-data".into(), idx as u64),
                                        msg.payload.clone(),
                                    )),
                            )
                    }),
                ))
            })
    }
}
