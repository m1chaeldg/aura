use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::ActiveInputTarget;
use crate::input_registry::{InputKey, InputRegistry};
use gpui::*;

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_spec_panel<V: 'static>(
        spec_content: &str,
        spec_error: Option<&str>,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let is_focused = matches!(active_input, Some(ActiveInputTarget::SpecText));
        let byte_len = spec_content.len();
        let line_count = spec_content.lines().count().max(1);

        div()
            .flex()
            .flex_col()
            .gap_2()
            // Top Toolbar: Label + Metrics + Status Pill + Actions (Format, Copy, Revert)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child("Aura Specification (.aura)"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x777777))
                                    .child(format!("({line_count} lines, {byte_len} bytes)")),
                            )
                            // Live Status Pill
                            .child(if let Some(err) = spec_error {
                                div()
                                    .id("spec-status-error")
                                    .px_2()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(rgb(0x4a1d1d))
                                    .border_1()
                                    .border_color(rgb(0xe74c3c))
                                    .text_xs()
                                    .text_color(rgb(0xff6b6b))
                                    .child(format!("⚠ {}", err))
                            } else {
                                div()
                                    .id("spec-status-synced")
                                    .px_2()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(rgb(0x1d3a24))
                                    .border_1()
                                    .border_color(rgb(0x28a745))
                                    .text_xs()
                                    .text_color(rgb(0x4cd964))
                                    .child("✓ Synced")
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .id("format-spec-btn")
                                    .px_2()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(rgb(0x28a745))
                                    .hover(|s| s.bg(rgb(0x218838)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(rgb(0xffffff))
                                    .child("⚡ Format Spec")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::FormatSpec, window, cx);
                                    })),
                            )
                            .child(
                                div()
                                    .id("copy-spec-btn")
                                    .px_2()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(rgb(0x333333))
                                    .hover(|s| s.bg(rgb(0x444444)).text_color(rgb(0xffffff)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(muted)
                                    .child("Copy Spec")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::CopySpec, window, cx);
                                    })),
                            )
                            .child(
                                div()
                                    .id("paste-spec-btn")
                                    .px_2()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(rgb(0x333333))
                                    .hover(|s| s.bg(rgb(0x444444)).text_color(rgb(0xffffff)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(muted)
                                    .child("Paste Spec")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::PasteSpec, window, cx);
                                    })),
                            )
                            .child(
                                div()
                                    .id("revert-spec-btn")
                                    .px_2()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(rgb(0x333333))
                                    .hover(|s| s.bg(rgb(0xe74c3c)).text_color(rgb(0xffffff)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(muted)
                                    .child("Revert")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::RevertSpec, window, cx);
                                    })),
                            ),
                    ),
            )
            // Monospace Editor Textarea
            .child(Self::render_inline_textarea(
                "direct-spec-editor",
                inputs.get_textarea(&InputKey::SpecText),
                spec_content,
                px(320.0),
                is_focused,
                border,
                if is_focused { rgb(0xffffff) } else { normal },
            ))
    }
}
