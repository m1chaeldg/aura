//! Shared GPUI Kit Design Guide theme helpers and semantic mappings for Aura.

use aura_core::model::HttpMethod;
use gpui::{App, Hsla};
use gpui_kit::assets::IconName;
use gpui_kit::component::tag::TagVariant;
use gpui_kit::component::{ActiveTheme as _, ColorName, Icon};

/// Builds a GPUI Kit `Icon` from the full bundled Lucide icon catalog.
pub fn icon(name: IconName) -> Icon {
    Icon::from(name)
}

/// Maps an HTTP method to a semantic theme accent color from `cx.theme()`.
pub fn method_color(method: &HttpMethod, cx: &App) -> Hsla {
    match method {
        HttpMethod::GET => cx.theme().blue,
        HttpMethod::POST => cx.theme().green,
        HttpMethod::PUT => cx.theme().yellow,
        HttpMethod::PATCH => cx.theme().magenta,
        HttpMethod::DELETE => cx.theme().red,
        HttpMethod::HEAD | HttpMethod::OPTIONS => cx.theme().cyan,
    }
}

/// Maps an HTTP method to a semantic GPUI Kit `TagVariant`.
pub fn method_tag_variant(method: &HttpMethod) -> TagVariant {
    match method {
        HttpMethod::GET => TagVariant::Info,
        HttpMethod::POST => TagVariant::Success,
        HttpMethod::PUT => TagVariant::Warning,
        HttpMethod::PATCH => TagVariant::Color(ColorName::Purple),
        HttpMethod::DELETE => TagVariant::Danger,
        HttpMethod::HEAD | HttpMethod::OPTIONS => TagVariant::Color(ColorName::Cyan),
    }
}

/// Maps an HTTP status code to a semantic theme color from `cx.theme()`.
pub fn status_color(status: u16, cx: &App) -> Hsla {
    match status {
        200..=299 => cx.theme().green,
        300..=399 => cx.theme().blue,
        400..=499 => cx.theme().yellow,
        _ => cx.theme().red,
    }
}

/// Maps an HTTP status code to a semantic GPUI Kit `TagVariant`.
pub fn status_tag_variant(status: u16) -> TagVariant {
    match status {
        200..=299 => TagVariant::Success,
        300..=399 => TagVariant::Info,
        400..=499 => TagVariant::Warning,
        _ => TagVariant::Danger,
    }
}

/// Subtle elevated panel surface for grouped headers, filter bars, and banners.
pub fn subtle_surface(cx: &App) -> Hsla {
    cx.theme().secondary.opacity(0.35)
}

/// Table header background surface derived cleanly from the active theme.
pub fn table_header_bg(cx: &App) -> Hsla {
    cx.theme().list_head
}

/// Color used to highlight resolved `{{...}}` template expressions.
pub fn template_resolved_color(cx: &App) -> Hsla {
    cx.theme().green
}

/// Color used to highlight unresolved `{{...}}` template expressions.
pub fn template_unresolved_color(cx: &App) -> Hsla {
    cx.theme().red
}
