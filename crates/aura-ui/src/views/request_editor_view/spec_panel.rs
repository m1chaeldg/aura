use super::{RequestEditorAction, RequestEditorView};
use crate::app_state::ActiveInputTarget;
use crate::input_registry::{InputKey, InputRegistry};
use crate::theme::icon;
use aura_core::model::AuraRequest;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{Sizable, h_flex, v_flex};

impl RequestEditorView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_spec_panel<V: 'static>(
        spec_content: &str,
        spec_error: Option<&str>,
        request: Option<&AuraRequest>,
        variable_rows: &[(String, String, bool)],
        show_script_reference: bool,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        border: Hsla,
        muted: Hsla,
        normal: Hsla,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let is_focused = matches!(active_input, Some(ActiveInputTarget::SpecText));
        let byte_len = spec_content.len();
        let line_count = spec_content.lines().count().max(1);

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
                                    .child("Aura Specification (.aura)"),
                            )
                            .child(
                                Tag::secondary()
                                    .xsmall()
                                    .child(format!("{line_count}L · {byte_len}B")),
                            )
                            .child(if let Some(err) = spec_error {
                                Tag::danger()
                                    .outline()
                                    .xsmall()
                                    .child(format!("Invalid: {err}"))
                            } else {
                                Tag::success().outline().xsmall().child("Synced")
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_1p5()
                            .child(Self::render_editor_tools(
                                InputKey::SpecText,
                                show_script_reference,
                                muted,
                                cx,
                                on_action,
                            ))
                            .child(
                                Button::new("format-spec-btn")
                                    .icon(icon(IconName::Zap))
                                    .label("Format")
                                    .xsmall()
                                    .success()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::FormatSpec, window, cx);
                                    })),
                            )
                            .child(
                                Button::new("copy-spec-btn")
                                    .icon(icon(IconName::Copy))
                                    .label("Copy")
                                    .xsmall()
                                    .outline()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::CopySpec, window, cx);
                                    })),
                            )
                            .child(
                                Button::new("paste-spec-btn")
                                    .icon(icon(IconName::FileText))
                                    .label("Paste")
                                    .xsmall()
                                    .outline()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::PasteSpec, window, cx);
                                    })),
                            )
                            .child(
                                Button::new("revert-spec-btn")
                                    .icon(icon(IconName::RotateCw))
                                    .label("Revert")
                                    .xsmall()
                                    .ghost()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::RevertSpec, window, cx);
                                    })),
                            ),
                    ),
            )
            .child(Self::render_inline_editor(
                "direct-spec-editor",
                inputs.get_editor(&InputKey::SpecText),
                spec_content,
                px(320.0),
                is_focused,
                border,
                normal,
            ))
            .when(show_script_reference, |el| {
                el.child(Self::render_reference_panel(
                    InputKey::SpecText,
                    true,
                    true,
                    request,
                    variable_rows,
                    border,
                    muted,
                    normal,
                    cx,
                    on_action,
                ))
            })
    }
}
