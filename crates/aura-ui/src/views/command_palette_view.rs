//! CommandPalette View: Keyboard-driven search modal overlay using gpui_kit.

use crate::app_state::AuraAppState;
use aura_core::model::HttpMethod;
use gpui::*;
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::separator::Separator;
use gpui_kit::component::tag::{Tag, TagVariant};
use gpui_kit::component::Sizable;

pub struct CommandPaletteView;

impl CommandPaletteView {
    pub fn render(state: &AuraAppState, search_input: &Entity<InputState>) -> impl IntoElement {
        let accent_blue = rgb(0x0066cc);
        let surface_color = rgb(0x272729);
        let text_color = rgb(0xf5f5f7);
        let text_muted = rgb(0x7a7a7a);

        let filtered = state.filtered_palette_items();
        let sel_idx = state.selected_palette_index;

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_start()
            .justify_center()
            .pt_16()
            .bg(rgba(0x0a0a0ccc))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w_96()
                    .rounded_lg()
                    .bg(surface_color)
                    .border_1()
                    .border_color(accent_blue)
                    .shadow_xl()
                    .child(
                        div()
                            .p_3()
                            .child(
                                Input::new(search_input)
                                    .id("command-palette-search-input")
                                    .small()
                                    .cleanable(true)
                                    .w_full(),
                            ),
                    )
                    .child(Separator::horizontal())
                    .child(if filtered.is_empty() {
                        Empty::new()
                            .header(EmptyHeader::new().description(
                                EmptyDescription::new().child("No matching commands or requests."),
                            ))
                            .into_any_element()
                    } else {
                        div()
                            .flex()
                            .flex_col()
                            .py_1()
                            .children(filtered.iter().enumerate().map(|(idx, item)| {
                                let is_sel = idx == sel_idx;
                                let item_method = item.method.unwrap_or(HttpMethod::GET);
                                let tag_variant = match item_method {
                                    HttpMethod::GET => TagVariant::Primary,
                                    HttpMethod::POST => TagVariant::Success,
                                    HttpMethod::PUT => TagVariant::Warning,
                                    HttpMethod::PATCH => TagVariant::Info,
                                    HttpMethod::DELETE => TagVariant::Danger,
                                    _ => TagVariant::Secondary,
                                };
                                let subtitle = item.subtitle.clone();

                                ListItem::new(ElementId::NamedInteger("palette-row".into(), idx as u64))
                                    .selected(is_sel)
                                    .px_4()
                                    .py_2()
                                    .suffix(move |_, _| {
                                        div()
                                            .text_xs()
                                            .text_color(text_muted)
                                            .child(subtitle.clone())
                                            .into_any_element()
                                    })
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                Tag::new()
                                                    .with_variant(tag_variant)
                                                    .outline()
                                                    .xsmall()
                                                    .child(item_method.as_str()),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(text_color)
                                                    .child(item.title.clone()),
                                            ),
                                    )
                            }))
                            .into_any_element()
                    })
                    .child(Separator::horizontal())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_4()
                            .py_2()
                            .text_xs()
                            .text_color(text_muted)
                            .child("↑ ↓ to navigate • ↵ to select • Esc to dismiss"),
                    ),
            )
    }
}
