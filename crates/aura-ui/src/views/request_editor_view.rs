//! RequestEditor View: Sub-tabs (Params, Headers, Auth, Body, Scripts, Tests) and content.

use aura_core::model::HttpMethod;
use gpui::prelude::FluentBuilder;
use gpui::*;

pub struct RequestEditorView;

impl RequestEditorView {
    pub fn render(
        split_ratio: f32,
        is_horizontal: bool,
        active_title: &str,
        active_method: HttpMethod,
    ) -> impl IntoElement {
        let border_color = rgb(0x333333);
        let accent_blue = rgb(0x0066cc);
        let text_muted = rgb(0x7a7a7a);

        div()
            .flex()
            .flex_col()
            .when(is_horizontal, |el| el.w(gpui::relative(split_ratio)))
            .when(!is_horizontal, |el| el.h(gpui::relative(split_ratio)))
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(border_color)
                    .py_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(accent_blue)
                            .child("REQUEST PANE (Dual-Mode: Visual / TOML)"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(text_muted)
                            .child("Params • Headers • Body • Scripting"),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .py_3()
                    .text_sm()
                    .text_color(rgb(0xcccccc))
                    .child(format!(
                        "Editing: {} [{}]",
                        active_title,
                        active_method.as_str()
                    )),
            )
    }
}
