//! TabBar View: Renders open request document tabs, badges, dirty dots, and close buttons using gpui_kit.

use crate::app_state::AuraAppState;
use crate::theme::{icon, method_tag_variant};
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::badge::Badge;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Selectable, Sizable, h_flex};

pub struct TabBarView;

impl TabBarView {
    pub fn render<V: 'static>(
        state: &AuraAppState,
        cx: &mut Context<V>,
        on_select_tab: impl Fn(&mut V, usize, &mut Window, &mut Context<V>) + 'static + Copy,
        on_close_tab: impl Fn(&mut V, &str, &mut Window, &mut Context<V>) + 'static + Copy,
        on_new_request: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let mut tab_bar = TabBar::new("workspace-tab-bar")
            .small()
            .menu(true)
            .suffix(
                Button::new("new-request-btn")
                    .icon(icon(IconName::Plus))
                    .xsmall()
                    .ghost()
                    .tooltip("New Request (Ctrl+N)")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        on_new_request(this, window, cx);
                    })),
            );

        for (idx, tab) in state.tabs.iter().enumerate() {
            let is_active = idx == state.active_tab_index;
            let tag_variant = method_tag_variant(&tab.method);
            let tab_id_close = tab.id.clone();

            let prefix_badge = Tag::new()
                .with_variant(tag_variant)
                .outline()
                .xsmall()
                .child(tab.method.as_str());

            let mut suffix_area = h_flex().gap_1();
            if tab.is_dirty {
                suffix_area = suffix_area.child(Badge::new().dot().color(cx.theme().warning));
            }
            suffix_area = suffix_area.child(
                Button::new(ElementId::Name(format!("close-tab-{}", tab.id).into()))
                    .icon(icon(IconName::X))
                    .xsmall()
                    .ghost()
                    .tooltip("Close tab")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        on_close_tab(this, &tab_id_close, window, cx);
                    })),
            );

            let tab_item = Tab::new()
                .label(tab.title.clone())
                .selected(is_active)
                .prefix(prefix_badge)
                .suffix(suffix_area)
                .on_click(cx.listener(move |this, _, window, cx| {
                    on_select_tab(this, idx, window, cx);
                }));

            tab_bar = tab_bar.child(tab_item);
        }

        h_flex()
            .w_full()
            .bg(cx.theme().tab_bar)
            .border_b_1()
            .border_color(cx.theme().border)
            .child(div().flex_1().min_w_0().child(tab_bar))
    }
}
