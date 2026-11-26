//! TabBar View: Renders open request document tabs, badges, dirty dots, and close buttons using gpui_kit.

use crate::app_state::AuraAppState;
use aura_core::model::HttpMethod;
use gpui::*;
use gpui_kit::component::badge::Badge;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::tag::{Tag, TagVariant};
use gpui_kit::component::{Selectable, Sizable};

pub struct TabBarView;

impl TabBarView {
    pub fn render<V: 'static>(
        state: &AuraAppState,
        cx: &mut Context<V>,
        on_select_tab: impl Fn(&mut V, usize, &mut Window, &mut Context<V>) + 'static + Copy,
        on_close_tab: impl Fn(&mut V, &str, &mut Window, &mut Context<V>) + 'static + Copy,
        on_new_request: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let border_color = rgb(0x333333);
        let amber_color = rgb(0xf5a623);

        let mut tab_bar = TabBar::new("workspace-tab-bar");

        for (idx, tab) in state.tabs.iter().enumerate() {
            let is_active = idx == state.active_tab_index;
            let tag_variant = match tab.method {
                HttpMethod::GET => TagVariant::Primary,
                HttpMethod::POST => TagVariant::Success,
                HttpMethod::PUT => TagVariant::Warning,
                HttpMethod::PATCH => TagVariant::Info,
                HttpMethod::DELETE => TagVariant::Danger,
                _ => TagVariant::Secondary,
            };

            let tab_id_close = tab.id.clone();

            let prefix_badge = Tag::new()
                .with_variant(tag_variant)
                .outline()
                .xsmall()
                .child(tab.method.as_str());

            let mut suffix_area = div().flex().items_center().gap_1();
            if tab.is_dirty {
                suffix_area = suffix_area.child(Badge::new().dot().color(amber_color));
            }
            suffix_area = suffix_area.child(
                Button::new(ElementId::NamedInteger("close-tab".into(), idx as u64))
                    .label("×")
                    .xsmall()
                    .ghost()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        on_close_tab(this, &tab_id_close, window, cx);
                    })),
            );

            let tab_id_right = tab.id.clone();
            let tab_item = Tab::new()
                .label(tab.title.clone())
                .selected(is_active)
                .prefix(prefix_badge)
                .suffix(suffix_area)
                .on_click(cx.listener(move |this, _, window, cx| {
                    on_select_tab(this, idx, window, cx);
                }))
                .on_mouse_down(
                    gpui::MouseButton::Right,
                    cx.listener(move |this, _, window, cx| {
                        on_close_tab(this, &tab_id_right, window, cx);
                    }),
                );

            tab_bar = tab_bar.child(tab_item);
        }

        div()
            .flex()
            .items_center()
            .justify_between()
            .px_3()
            .py_1()
            .bg(rgb(0x222224))
            .border_b_1()
            .border_color(border_color)
            .child(tab_bar)
            .child(
                Button::new("new-request-btn")
                    .label("+")
                    .xsmall()
                    .secondary()
                    .rounded(ButtonRounded::Medium)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        on_new_request(this, window, cx);
                    })),
            )
    }
}
