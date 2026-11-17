//! RequestEditor View: Sub-tabs (Params, Headers, Auth, Body, Scripts, Tests, Settings) and content panels.

use crate::stores::EditorTab;
use aura_core::model::{AuraRequest, AuthConfig, BodyType, HttpMethod};
use base64::Engine;
use gpui::prelude::FluentBuilder;
use gpui::*;

#[derive(Debug, Clone)]
pub enum RequestEditorAction {
    SelectTab(EditorTab),
    // Headers
    ToggleHeader(usize),
    RemoveHeader(usize),
    OpenAddHeader,
    OpenEditHeader(usize),
    AddPresetHeader(&'static str, &'static str),
    // Params
    ToggleParam(usize),
    RemoveParam(usize),
    OpenAddParam,
    OpenEditParam(usize),
    // Body
    SetBodyType(&'static str),
    OpenEditBody,
    FormatJsonBody,
    ClearBody,
    OpenAddFormParam,
    ToggleFormParam(usize),
    RemoveFormParam(usize),
    OpenAddMultipart,
    RemoveMultipart(usize),
    // Auth
    SetAuthType(&'static str),
    OpenEditAuth,
    ToggleApiKeyPlacement(bool),
}

pub struct RequestEditorView;

impl RequestEditorView {
    pub const fn available_tabs() -> &'static [EditorTab] {
        &[
            EditorTab::Params,
            EditorTab::Headers,
            EditorTab::Auth,
            EditorTab::Body,
            EditorTab::Variables,
            EditorTab::Scripts,
            EditorTab::Tests,
            EditorTab::Settings,
        ]
    }

    pub fn tab_title(tab: EditorTab) -> &'static str {
        match tab {
            EditorTab::Params => "Params",
            EditorTab::Headers => "Headers",
            EditorTab::Auth => "Auth",
            EditorTab::Body => "Body",
            EditorTab::Variables => "Variables",
            EditorTab::Scripts => "Scripts",
            EditorTab::Tests => "Tests",
            EditorTab::Settings => "Settings",
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render<V: 'static>(
        split_ratio: f32,
        is_horizontal: bool,
        _active_title: &str,
        _active_method: HttpMethod,
        active_tab: EditorTab,
        request: Option<&AuraRequest>,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let border_color = rgb(0x333333);
        let accent_blue = rgb(0x0066cc);
        let bg_surface = rgb(0x272729);
        let text_muted = rgb(0x7a7a7a);

        let headers_count = request.map(|r| r.headers.iter().filter(|h| h.enabled).count()).unwrap_or(0);
        let params_count = request.map(|r| r.params.iter().filter(|p| p.enabled).count()).unwrap_or(0);
        let body_name = request.map(|r| r.body.type_name()).unwrap_or("None");
        let auth_name = request.map(|r| r.auth.type_name()).unwrap_or("Inherit");

        div()
            .flex()
            .flex_col()
            .when(is_horizontal, |el| el.w(gpui::relative(split_ratio)))
            .when(!is_horizontal, |el| el.h(gpui::relative(split_ratio)))
            .bg(rgb(0x1d1d1f))
            .border_r_1()
            .border_color(border_color)
            // Tabs Bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .px_3()
                    .py_1()
                    .bg(bg_surface)
                    .border_b_1()
                    .border_color(border_color)
                    .children(Self::available_tabs().iter().copied().map(|tab| {
                        let is_active = tab == active_tab;
                        let badge = match tab {
                            EditorTab::Headers if headers_count > 0 => Some(format!(" ({})", headers_count)),
                            EditorTab::Params if params_count > 0 => Some(format!(" ({})", params_count)),
                            EditorTab::Body if body_name != "None" => Some(format!(" ({})", body_name)),
                            EditorTab::Auth if auth_name != "Inherit" && auth_name != "None" => Some(format!(" ({})", auth_name)),
                            _ => None,
                        };

                        div()
                            .id(ElementId::NamedInteger("editor-tab".into(), tab as u16 as u64))
                            .flex()
                            .items_center()
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(if is_active { rgb(0x1d1d1f) } else { bg_surface })
                            .border_b_2()
                            .border_color(if is_active { accent_blue } else { rgba(0x00000000) })
                            .hover(|s| s.bg(rgb(0x333333)))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(if is_active { rgb(0xffffff) } else { text_muted })
                            .child(format!("{}{}", Self::tab_title(tab), badge.unwrap_or_default()))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::SelectTab(tab), window, cx);
                            }))
                    })),
            )
            // Content Panel
            .child(
                div()
                    .id("editor-panel-scroll")
                    .flex_1()
                    .p_4()
                    .overflow_y_scroll()
                    .child(Self::render_panel_content(active_tab, request, cx, on_action)),
            )
    }

    fn render_panel_content<V: 'static>(
        tab: EditorTab,
        request: Option<&AuraRequest>,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let border = rgb(0x333333);
        let muted = rgb(0x7a7a7a);
        let normal = rgb(0xcccccc);

        match tab {
            EditorTab::Headers => Self::render_headers_panel(request, border, muted, normal, cx, on_action),
            EditorTab::Params => Self::render_params_panel(request, border, muted, normal, cx, on_action),
            EditorTab::Body => Self::render_body_panel(request, border, muted, normal, cx, on_action),
            EditorTab::Auth => Self::render_auth_panel(request, border, muted, normal, cx, on_action),
            EditorTab::Variables => Self::render_box(
                "ENVIRONMENT & REQUEST VARIABLES",
                "Variables referenced with {{var_name}} in URL, headers, and body.",
                false, border, muted, normal,
            ),
            EditorTab::Scripts => Self::render_scripts_panel(request, border, muted, normal),
            EditorTab::Tests => Self::render_tests_panel(request, border, muted, normal),
            EditorTab::Settings => Self::render_settings_panel(request, border, muted, normal),
        }
    }

    // =========================================================================
    // Headers Panel
    // =========================================================================

    fn render_headers_panel<V: 'static>(
        request: Option<&AuraRequest>,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let headers = request.map(|r| r.headers.as_slice()).unwrap_or(&[]);

        div()
            .flex()
            .flex_col()
            .gap_3()
            // Title + Actions row
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("REQUEST HEADERS"))
                            .child(
                                div()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(rgb(0x272729))
                                    .text_xs()
                                    .text_color(muted)
                                    .child(format!("{}", headers.len())),
                            ),
                    )
                    .child(
                        div()
                            .id("add-header-btn")
                            .flex()
                            .items_center()
                            .gap_1()
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .bg(rgb(0x0066cc))
                            .hover(|s| s.bg(rgb(0x0071e3)))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(rgb(0xffffff))
                            .child("+ Add Header")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::OpenAddHeader, window, cx);
                            })),
                    ),
            )
            // Presets row
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(div().text_xs().text_color(muted).child("Presets:"))
                    .child(Self::render_preset_chip("Content-Type: JSON", "Content-Type", "application/json", cx, on_action))
                    .child(Self::render_preset_chip("Accept: */*", "Accept", "*/*", cx, on_action))
                    .child(Self::render_preset_chip("User-Agent: Aura", "User-Agent", "Aura-Client/0.1.0", cx, on_action)),
            )
            // Headers list
            .when(headers.is_empty(), |el| {
                el.child(
                    div()
                        .p_4()
                        .rounded_md()
                        .bg(rgb(0x222224))
                        .border_1()
                        .border_color(border)
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_2()
                        .child(div().text_xs().text_color(muted).child("No custom headers defined. Add headers above or pick a preset."))
                        .child(
                            div()
                                .id("btn-add-first-header")
                                .px_3()
                                .py_1()
                                .rounded_sm()
                                .bg(rgb(0x333333))
                                .hover(|s| s.bg(rgb(0x444444)))
                                .cursor_pointer()
                                .text_xs()
                                .text_color(rgb(0xffffff))
                                .child("+ Add First Header")
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_action(this, RequestEditorAction::OpenAddHeader, window, cx);
                                })),
                        ),
                )
            })
            .children(headers.iter().enumerate().map(|(idx, header)| {
                let enabled = header.enabled;
                let key = header.key.clone();
                let val = header.value.clone();

                div()
                    .id(ElementId::NamedInteger("header-row".into(), idx as u64))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(if enabled { rgb(0x272729) } else { rgb(0x202022) })
                    .border_1()
                    .border_color(border)
                    .hover(|s| s.bg(rgb(0x2c2c2e)))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            // Enabled Checkbox
                            .child(
                                div()
                                    .id(ElementId::NamedInteger("toggle-header".into(), idx as u64))
                                    .cursor_pointer()
                                    .text_sm()
                                    .text_color(if enabled { rgb(0x28a745) } else { muted })
                                    .child(if enabled { "☑" } else { "☐" })
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::ToggleHeader(idx), window, cx);
                                    })),
                            )
                            // Key
                            .child(
                                div()
                                    .id(ElementId::NamedInteger("edit-header-key".into(), idx as u64))
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(if enabled { rgb(0xffffff) } else { muted })
                                    .child(key)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::OpenEditHeader(idx), window, cx);
                                    })),
                            )
                            .child(div().text_xs().text_color(muted).child(":"))
                            // Value
                            .child(
                                div()
                                    .id(ElementId::NamedInteger("edit-header-val".into(), idx as u64))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(if enabled { normal } else { muted })
                                    .child(val)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::OpenEditHeader(idx), window, cx);
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            // Edit button
                            .child(
                                div()
                                    .id(ElementId::NamedInteger("edit-header-btn".into(), idx as u64))
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(rgb(0x333333))
                                    .hover(|s| s.bg(rgb(0x444444)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(rgb(0x2997ff))
                                    .child("Edit")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::OpenEditHeader(idx), window, cx);
                                    })),
                            )
                            // Delete button
                            .child(
                                div()
                                    .id(ElementId::NamedInteger("del-header-btn".into(), idx as u64))
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(rgb(0x333333))
                                    .hover(|s| s.bg(rgb(0xe74c3c)).text_color(rgb(0xffffff)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(muted)
                                    .child("✕")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::RemoveHeader(idx), window, cx);
                                    })),
                            ),
                    )
            }))
    }

    fn render_preset_chip<V: 'static>(
        label: &'static str,
        key: &'static str,
        value: &'static str,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        div()
            .id(label)
            .px_2()
            .py_0p5()
            .rounded_sm()
            .bg(rgb(0x272729))
            .border_1()
            .border_color(rgb(0x333333))
            .hover(|s| s.bg(rgb(0x333333)).border_color(rgb(0x0066cc)))
            .cursor_pointer()
            .text_xs()
            .text_color(rgb(0x2997ff))
            .child(label)
            .on_click(cx.listener(move |this, _, window, cx| {
                on_action(this, RequestEditorAction::AddPresetHeader(key, value), window, cx);
            }))
    }

    // =========================================================================
    // Query Parameters Panel
    // =========================================================================

    fn render_params_panel<V: 'static>(
        request: Option<&AuraRequest>,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let params = request.map(|r| r.params.as_slice()).unwrap_or(&[]);

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("QUERY PARAMETERS"))
                            .child(
                                div()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(rgb(0x272729))
                                    .text_xs()
                                    .text_color(muted)
                                    .child(format!("{}", params.len())),
                            ),
                    )
                    .child(
                        div()
                            .id("add-param-btn")
                            .flex()
                            .items_center()
                            .gap_1()
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .bg(rgb(0x0066cc))
                            .hover(|s| s.bg(rgb(0x0071e3)))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(rgb(0xffffff))
                            .child("+ Add Parameter")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::OpenAddParam, window, cx);
                            })),
                    ),
            )
            .when(params.is_empty(), |el| {
                el.child(
                    div()
                        .p_4()
                        .rounded_md()
                        .bg(rgb(0x222224))
                        .border_1()
                        .border_color(border)
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_2()
                        .child(div().text_xs().text_color(muted).child("No query parameters attached to URL."))
                        .child(
                            div()
                                .id("btn-add-first-param")
                                .px_3()
                                .py_1()
                                .rounded_sm()
                                .bg(rgb(0x333333))
                                .hover(|s| s.bg(rgb(0x444444)))
                                .cursor_pointer()
                                .text_xs()
                                .text_color(rgb(0xffffff))
                                .child("+ Add First Parameter")
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_action(this, RequestEditorAction::OpenAddParam, window, cx);
                                })),
                        ),
                )
            })
            .children(params.iter().enumerate().map(|(idx, param)| {
                let enabled = param.enabled;
                let key = param.key.clone();
                let val = param.value.clone();

                div()
                    .id(ElementId::NamedInteger("param-row".into(), idx as u64))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(if enabled { rgb(0x272729) } else { rgb(0x202022) })
                    .border_1()
                    .border_color(border)
                    .hover(|s| s.bg(rgb(0x2c2c2e)))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            // Checkbox
                            .child(
                                div()
                                    .id(ElementId::NamedInteger("toggle-param".into(), idx as u64))
                                    .cursor_pointer()
                                    .text_sm()
                                    .text_color(if enabled { rgb(0x28a745) } else { muted })
                                    .child(if enabled { "☑" } else { "☐" })
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::ToggleParam(idx), window, cx);
                                    })),
                            )
                            // Key
                            .child(
                                div()
                                    .id(ElementId::NamedInteger("edit-param-key".into(), idx as u64))
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(if enabled { rgb(0xffffff) } else { muted })
                                    .child(key)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::OpenEditParam(idx), window, cx);
                                    })),
                            )
                            .child(div().text_xs().text_color(muted).child("="))
                            // Value
                            .child(
                                div()
                                    .id(ElementId::NamedInteger("edit-param-val".into(), idx as u64))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(if enabled { normal } else { muted })
                                    .child(val)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::OpenEditParam(idx), window, cx);
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .id(ElementId::NamedInteger("edit-param-btn".into(), idx as u64))
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(rgb(0x333333))
                                    .hover(|s| s.bg(rgb(0x444444)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(rgb(0x2997ff))
                                    .child("Edit")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::OpenEditParam(idx), window, cx);
                                    })),
                            )
                            .child(
                                div()
                                    .id(ElementId::NamedInteger("del-param-btn".into(), idx as u64))
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(rgb(0x333333))
                                    .hover(|s| s.bg(rgb(0xe74c3c)).text_color(rgb(0xffffff)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(muted)
                                    .child("✕")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::RemoveParam(idx), window, cx);
                                    })),
                            ),
                    )
            }))
    }

    // =========================================================================
    // Request Body Panel
    // =========================================================================

    fn render_body_panel<V: 'static>(
        request: Option<&AuraRequest>,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let body = request.map(|r| &r.body);
        let active_type = body.map(|b| b.type_name()).unwrap_or("None");
        const FORMATS: [&'static str; 8] = ["None", "JSON", "XML", "Text", "Raw", "FormUrlEncoded", "Multipart", "Binary"];

        div()
            .flex()
            .flex_col()
            .gap_3()
            // Format Selector Bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .flex_wrap()
                    .children(FORMATS.iter().copied().map(|fmt| {
                        let is_active = match fmt {
                            "FormUrlEncoded" => active_type == "FormUrlEncoded" || active_type == "Form",
                            _ => active_type == fmt,
                        };
                        let display_name = match fmt {
                            "FormUrlEncoded" => "Form (URL-Encoded)",
                            "Multipart" => "Multipart Form",
                            other => other,
                        };
                        div()
                            .id(fmt)
                            .px_2p5()
                            .py_1()
                            .rounded_sm()
                            .bg(if is_active { rgb(0x0066cc) } else { rgb(0x272729) })
                            .border_1()
                            .border_color(if is_active { rgb(0x0071e3) } else { border })
                            .hover(|s| s.bg(rgb(0x333333)))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(if is_active { rgb(0xffffff) } else { muted })
                            .child(display_name)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::SetBodyType(fmt), window, cx);
                            }))
                    })),
            )
            // Body Content based on Format
            .child(match body {
                Some(BodyType::None) | None => {
                    div()
                        .p_6()
                        .rounded_md()
                        .bg(rgb(0x222224))
                        .border_1()
                        .border_color(border)
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_1()
                        .child(div().text_xs().text_color(muted).child("This request does not have a body."))
                        .child(div().text_xs().text_color(rgb(0x555555)).child("Select JSON, Form, or Raw above to send payload data."))
                }
                Some(BodyType::Json(content)) => Self::render_text_body_editor("JSON (application/json)", content, true, border, muted, normal, cx, on_action),
                Some(BodyType::Xml(content)) => Self::render_text_body_editor("XML (application/xml)", content, false, border, muted, normal, cx, on_action),
                Some(BodyType::Text(content)) => Self::render_text_body_editor("Text (text/plain)", content, false, border, muted, normal, cx, on_action),
                Some(BodyType::Raw { content, content_type }) => {
                    let label = format!("Raw ({})", content_type.as_deref().unwrap_or("text/plain"));
                    Self::render_text_body_editor(&label, content, false, border, muted, normal, cx, on_action)
                }
                Some(BodyType::FormUrlEncoded(items)) => Self::render_form_urlencoded_editor(items, border, muted, normal, cx, on_action),
                Some(BodyType::Form(map)) => {
                    let items: Vec<aura_core::model::ParamItem> = map.iter().map(|(k, v)| aura_core::model::ParamItem::new(k.clone(), v.clone())).collect();
                    Self::render_form_urlencoded_editor(&items, border, muted, normal, cx, on_action)
                }
                Some(BodyType::Multipart(parts)) => Self::render_multipart_editor(parts, border, muted, normal, cx, on_action),
                Some(BodyType::Binary { file_path, content_type }) => {
                    let ct = content_type.as_deref().unwrap_or("application/octet-stream");
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .p_4()
                        .rounded_md()
                        .bg(rgb(0x222224))
                        .border_1()
                        .border_color(border)
                        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child(format!("BINARY FILE ({})", ct)))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .p_2()
                                .bg(rgb(0x1d1d1f))
                                .border_1()
                                .border_color(border)
                                .rounded_sm()
                                .child(div().text_xs().text_color(normal).child(if file_path.is_empty() { "No binary file selected".to_string() } else { file_path.clone() })),
                        )
                }
            })
    }

    fn render_text_body_editor<V: 'static>(
        label: &str,
        content: &str,
        is_json: bool,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child(label.to_string()))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .when(is_json, |el| {
                                el.child(
                                    div()
                                        .id("format-json-btn")
                                        .px_2()
                                        .py_0p5()
                                        .rounded_sm()
                                        .bg(rgb(0x28a745))
                                        .hover(|s| s.bg(rgb(0x218838)))
                                        .cursor_pointer()
                                        .text_xs()
                                        .text_color(rgb(0xffffff))
                                        .child("⚡ Format / Pretty-Print")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(this, RequestEditorAction::FormatJsonBody, window, cx);
                                        })),
                                )
                            })
                            .child(
                                div()
                                    .id("edit-body-btn")
                                    .px_2()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(rgb(0x0066cc))
                                    .hover(|s| s.bg(rgb(0x0071e3)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(rgb(0xffffff))
                                    .child("✎ Edit Body")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::OpenEditBody, window, cx);
                                    })),
                            )
                            .child(
                                div()
                                    .id("clear-body-btn")
                                    .px_2()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(rgb(0x333333))
                                    .hover(|s| s.bg(rgb(0xe74c3c)).text_color(rgb(0xffffff)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .text_color(muted)
                                    .child("Clear")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::ClearBody, window, cx);
                                    })),
                            ),
                    ),
            )
            .child(
                div()
                    .p_3()
                    .rounded_md()
                    .bg(rgb(0x18181a))
                    .border_1()
                    .border_color(border)
                    .font_family("monospace")
                    .text_xs()
                    .text_color(normal)
                    .child(if content.trim().is_empty() { "// Empty body payload. Click 'Edit Body' to input.".to_string() } else { content.to_string() }),
            )
    }

    fn render_form_urlencoded_editor<V: 'static>(
        items: &[aura_core::model::ParamItem],
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("X-WWW-FORM-URLENCODED FIELDS"))
                    .child(
                        div()
                            .id("add-form-field-btn")
                            .px_2()
                            .py_0p5()
                            .rounded_sm()
                            .bg(rgb(0x0066cc))
                            .hover(|s| s.bg(rgb(0x0071e3)))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(rgb(0xffffff))
                            .child("+ Add Field")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::OpenAddFormParam, window, cx);
                            })),
                    ),
            )
            .when(items.is_empty(), |el| {
                el.child(
                    div()
                        .p_4()
                        .rounded_md()
                        .bg(rgb(0x222224))
                        .border_1()
                        .border_color(border)
                        .child(div().text_xs().text_color(muted).child("No form fields defined. Click '+ Add Field' to start.")),
                )
            })
            .children(items.iter().enumerate().map(|(idx, item)| {
                let enabled = item.enabled;
                let k = item.key.clone();
                let v = item.value.clone();

                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(rgb(0x272729))
                    .border_1()
                    .border_color(border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .id(ElementId::NamedInteger("toggle-form-param".into(), idx as u64))
                                    .cursor_pointer()
                                    .text_sm()
                                    .text_color(if enabled { rgb(0x28a745) } else { muted })
                                    .child(if enabled { "☑" } else { "☐" })
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, RequestEditorAction::ToggleFormParam(idx), window, cx);
                                    })),
                            )
                            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(rgb(0xffffff)).child(k))
                            .child(div().text_xs().text_color(muted).child("="))
                            .child(div().text_xs().text_color(normal).child(v)),
                    )
                    .child(
                        div()
                            .id(ElementId::NamedInteger("del-form-param".into(), idx as u64))
                            .px_1p5()
                            .py_0p5()
                            .rounded_sm()
                            .bg(rgb(0x333333))
                            .hover(|s| s.bg(rgb(0xe74c3c)).text_color(rgb(0xffffff)))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(muted)
                            .child("✕")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::RemoveFormParam(idx), window, cx);
                            })),
                    )
            }))
    }

    fn render_multipart_editor<V: 'static>(
        parts: &[aura_core::model::MultipartPart],
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("MULTIPART/FORM-DATA FIELDS"))
                    .child(
                        div()
                            .id("add-multipart-field-btn")
                            .px_2()
                            .py_0p5()
                            .rounded_sm()
                            .bg(rgb(0x0066cc))
                            .hover(|s| s.bg(rgb(0x0071e3)))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(rgb(0xffffff))
                            .child("+ Add Part")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::OpenAddMultipart, window, cx);
                            })),
                    ),
            )
            .when(parts.is_empty(), |el| {
                el.child(
                    div()
                        .p_4()
                        .rounded_md()
                        .bg(rgb(0x222224))
                        .border_1()
                        .border_color(border)
                        .child(div().text_xs().text_color(muted).child("No multipart fields defined. Click '+ Add Part' to start.")),
                )
            })
            .children(parts.iter().enumerate().map(|(idx, part)| {
                let name = part.name.clone();
                let is_file = part.file_path.is_some();
                let display_val = part.value.clone().or_else(|| part.file_path.clone()).unwrap_or_default();

                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(rgb(0x272729))
                    .border_1()
                    .border_color(border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(if is_file { rgb(0x8e44ad) } else { rgb(0x0066cc) })
                                    .text_xs()
                                    .text_color(rgb(0xffffff))
                                    .child(if is_file { "FILE" } else { "TEXT" }),
                            )
                            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(rgb(0xffffff)).child(name))
                            .child(div().text_xs().text_color(muted).child("="))
                            .child(div().text_xs().text_color(normal).child(display_val)),
                    )
                    .child(
                        div()
                            .id(ElementId::NamedInteger("del-multipart".into(), idx as u64))
                            .px_1p5()
                            .py_0p5()
                            .rounded_sm()
                            .bg(rgb(0x333333))
                            .hover(|s| s.bg(rgb(0xe74c3c)).text_color(rgb(0xffffff)))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(muted)
                            .child("✕")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::RemoveMultipart(idx), window, cx);
                            })),
                    )
            }))
    }

    // =========================================================================
    // Authentication Panel
    // =========================================================================

    fn render_auth_panel<V: 'static>(
        request: Option<&AuraRequest>,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let auth = request.map(|r| &r.auth);
        let active_auth_type = auth.map(|a| a.type_name()).unwrap_or("Inherit");
        const STRATEGIES: [&'static str; 7] = ["Inherit", "None", "Bearer Token", "Basic Auth", "API Key", "AWS SigV4", "OAuth 2.0"];

        div()
            .flex()
            .flex_col()
            .gap_3()
            // Strategy Selector Bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .flex_wrap()
                    .children(STRATEGIES.iter().copied().map(|strat| {
                        let is_active = active_auth_type == strat;
                        div()
                            .id(strat)
                            .px_2p5()
                            .py_1()
                            .rounded_sm()
                            .bg(if is_active { rgb(0x0066cc) } else { rgb(0x272729) })
                            .border_1()
                            .border_color(if is_active { rgb(0x0071e3) } else { border })
                            .hover(|s| s.bg(rgb(0x333333)))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(if is_active { rgb(0xffffff) } else { muted })
                            .child(strat)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::SetAuthType(strat), window, cx);
                            }))
                    })),
            )
            // Auth Configuration details based on strategy
            .child(match auth {
                Some(AuthConfig::Inherit) | None => {
                    div()
                        .p_4()
                        .rounded_md()
                        .bg(rgb(0x222224))
                        .border_1()
                        .border_color(border)
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(rgb(0xffffff)).child("Inherit Auth from Parent"))
                        .child(div().text_xs().text_color(muted).child("This request will automatically use the authentication settings defined on its parent collection or folder."))
                }
                Some(AuthConfig::None) => {
                    div()
                        .p_4()
                        .rounded_md()
                        .bg(rgb(0x222224))
                        .border_1()
                        .border_color(border)
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(rgb(0xffffff)).child("No Authentication"))
                        .child(div().text_xs().text_color(muted).child("No Authorization headers or authentication query parameters will be included in the request."))
                }
                Some(AuthConfig::Bearer { token }) => {
                    let is_jwt = token.split('.').count() == 3;
                    let decoded = if is_jwt {
                        aura_core::security::inspect_jwt(token).ok()
                    } else {
                        None
                    };

                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("BEARER TOKEN"))
                                .child(
                                    div()
                                        .id("edit-bearer-btn")
                                        .px_2()
                                        .py_0p5()
                                        .rounded_sm()
                                        .bg(rgb(0x0066cc))
                                        .hover(|s| s.bg(rgb(0x0071e3)))
                                        .cursor_pointer()
                                        .text_xs()
                                        .text_color(rgb(0xffffff))
                                        .child("✎ Edit Token")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(this, RequestEditorAction::OpenEditAuth, window, cx);
                                        })),
                                 ),
                        )
                        .child(
                            div()
                                .p_3()
                                .rounded_md()
                                .bg(rgb(0x18181a))
                                .border_1()
                                .border_color(border)
                                .font_family("monospace")
                                .text_xs()
                                .text_color(normal)
                                .child(if token.is_empty() { "// No token set. Click 'Edit Token' to set.".to_string() } else { token.clone() }),
                        )
                        .when_some(decoded, |el, jwt| {
                            el.child(
                                div()
                                    .p_3()
                                    .rounded_md()
                                    .bg(rgb(0x222224))
                                    .border_1()
                                    .border_color(rgb(0x0066cc))
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_2()
                                                    .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(rgb(0x2997ff)).child("JWT INSPECTOR"))
                                                    .child(div().px_1p5().py_0p5().rounded_sm().bg(rgb(0x333333)).text_xs().text_color(rgb(0xffffff)).child(format!("alg: {}", jwt.algorithm))),
                                            )
                                            .child(
                                                div()
                                                    .px_2()
                                                    .py_0p5()
                                                    .rounded_sm()
                                                    .bg(if jwt.is_expired { rgb(0xe74c3c) } else { rgb(0x28a745) })
                                                    .text_xs()
                                                    .text_color(rgb(0xffffff))
                                                    .child(if jwt.is_expired { "EXPIRED" } else { "VALID" }),
                                            ),
                                    )
                                    .when_some(jwt.subject, |el, sub| {
                                        el.child(div().text_xs().text_color(muted).child(format!("Subject (sub): {}", sub)))
                                    })
                                    .when_some(jwt.issuer, |el, iss| {
                                        el.child(div().text_xs().text_color(muted).child(format!("Issuer (iss): {}", iss)))
                                    })
                                    .child(
                                        div()
                                            .p_2()
                                            .rounded_sm()
                                            .bg(rgb(0x18181a))
                                            .font_family("monospace")
                                            .text_xs()
                                            .text_color(rgb(0xcccccc))
                                            .child(serde_json::to_string_pretty(&jwt.payload).unwrap_or_default()),
                                    ),
                            )
                        })
                }
                Some(AuthConfig::Basic { username, password }) => {
                    let preview = {
                        let encoded = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", username, password));
                        format!("Authorization: Basic {}", encoded)
                    };

                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("BASIC AUTHENTICATION"))
                                .child(
                                    div()
                                        .id("edit-basic-btn")
                                        .px_2()
                                        .py_0p5()
                                        .rounded_sm()
                                        .bg(rgb(0x0066cc))
                                        .hover(|s| s.bg(rgb(0x0071e3)))
                                        .cursor_pointer()
                                        .text_xs()
                                        .text_color(rgb(0xffffff))
                                        .child("✎ Edit Credentials")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(this, RequestEditorAction::OpenEditAuth, window, cx);
                                        })),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .p_3()
                                .rounded_md()
                                .bg(rgb(0x272729))
                                .border_1()
                                .border_color(border)
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(div().text_xs().text_color(muted).child("Username:"))
                                        .child(div().text_xs().text_color(rgb(0xffffff)).child(if username.is_empty() { "(not set)".to_string() } else { username.clone() })),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(div().text_xs().text_color(muted).child("Password:"))
                                        .child(div().text_xs().text_color(rgb(0xffffff)).child(if password.is_empty() { "(not set)".to_string() } else { "••••••••".to_string() })),
                                )
                                .child(
                                    div()
                                        .mt_1()
                                        .p_2()
                                        .rounded_sm()
                                        .bg(rgb(0x1d1d1f))
                                        .font_family("monospace")
                                        .text_xs()
                                        .text_color(rgb(0x28a745))
                                        .child(preview),
                                ),
                        )
                }
                Some(AuthConfig::ApiKey { key, value, in_header }) => {
                    let in_hdr = *in_header;
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("API KEY AUTHENTICATION"))
                                .child(
                                    div()
                                        .id("edit-api-key-btn")
                                        .px_2()
                                        .py_0p5()
                                        .rounded_sm()
                                        .bg(rgb(0x0066cc))
                                        .hover(|s| s.bg(rgb(0x0071e3)))
                                        .cursor_pointer()
                                        .text_xs()
                                        .text_color(rgb(0xffffff))
                                        .child("✎ Edit API Key")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(this, RequestEditorAction::OpenEditAuth, window, cx);
                                        })),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .p_3()
                                .rounded_md()
                                .bg(rgb(0x272729))
                                .border_1()
                                .border_color(border)
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(div().text_xs().text_color(muted).child("Key Name:"))
                                        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(rgb(0xffffff)).child(key.clone())),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(div().text_xs().text_color(muted).child("Key Value:"))
                                        .child(div().text_xs().text_color(normal).child(if value.is_empty() { "(empty)".to_string() } else { value.clone() })),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .mt_1()
                                        .child(div().text_xs().text_color(muted).child("Send In:"))
                                        .child(
                                            div()
                                                .id("btn-apikey-hdr")
                                                .px_2()
                                                .py_0p5()
                                                .rounded_sm()
                                                .bg(if in_hdr { rgb(0x0066cc) } else { rgb(0x333333) })
                                                .hover(|s| s.bg(rgb(0x444444)))
                                                .cursor_pointer()
                                                .text_xs()
                                                .text_color(rgb(0xffffff))
                                                .child("Header")
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    on_action(this, RequestEditorAction::ToggleApiKeyPlacement(true), window, cx);
                                                })),
                                        )
                                        .child(
                                            div()
                                                .id("btn-apikey-query")
                                                .px_2()
                                                .py_0p5()
                                                .rounded_sm()
                                                .bg(if !in_hdr { rgb(0x0066cc) } else { rgb(0x333333) })
                                                .hover(|s| s.bg(rgb(0x444444)))
                                                .cursor_pointer()
                                                .text_xs()
                                                .text_color(rgb(0xffffff))
                                                .child("Query Params")
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    on_action(this, RequestEditorAction::ToggleApiKeyPlacement(false), window, cx);
                                                })),
                                        ),
                                ),
                        )
                }
                Some(AuthConfig::AwsSigV4 { access_key, region, service, session_token, .. }) => {
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("AWS SIGNATURE V4"))
                                .child(
                                    div()
                                        .id("edit-aws-btn")
                                        .px_2()
                                        .py_0p5()
                                        .rounded_sm()
                                        .bg(rgb(0x0066cc))
                                        .hover(|s| s.bg(rgb(0x0071e3)))
                                        .cursor_pointer()
                                        .text_xs()
                                        .text_color(rgb(0xffffff))
                                        .child("✎ Edit AWS Auth")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(this, RequestEditorAction::OpenEditAuth, window, cx);
                                        })),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .p_3()
                                .rounded_md()
                                .bg(rgb(0x272729))
                                .border_1()
                                .border_color(border)
                                .child(div().text_xs().text_color(normal).child(format!("Access Key: {}", if access_key.is_empty() { "(none)" } else { access_key })))
                                .child(div().text_xs().text_color(normal).child(format!("Region: {}", region)))
                                .child(div().text_xs().text_color(normal).child(format!("Service: {}", service)))
                                .child(div().text_xs().text_color(muted).child(format!("Session Token: {}", if session_token.is_some() { "Present" } else { "None" }))),
                        )
                }
                Some(AuthConfig::OAuth2 { grant_type, access_token, token_url, .. }) => {
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("OAUTH 2.0"))
                                .child(
                                    div()
                                        .id("edit-oauth2-btn")
                                        .px_2()
                                        .py_0p5()
                                        .rounded_sm()
                                        .bg(rgb(0x0066cc))
                                        .hover(|s| s.bg(rgb(0x0071e3)))
                                        .cursor_pointer()
                                        .text_xs()
                                        .text_color(rgb(0xffffff))
                                        .child("✎ Edit OAuth2")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(this, RequestEditorAction::OpenEditAuth, window, cx);
                                        })),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .p_3()
                                .rounded_md()
                                .bg(rgb(0x272729))
                                .border_1()
                                .border_color(border)
                                .child(div().text_xs().text_color(normal).child(format!("Grant Type: {}", grant_type)))
                                .child(div().text_xs().text_color(normal).child(format!("Access Token: {}", if access_token.is_empty() { "(empty)" } else { access_token })))
                                .child(div().text_xs().text_color(muted).child(format!("Token URL: {}", token_url.as_deref().unwrap_or("(none)")))),
                        )
                }
            })
    }

    fn render_box(title: &'static str, content: &str, is_code: bool, border: Rgba, muted: Rgba, normal: Rgba) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child(title))
            .child(
                div()
                    .p_3()
                    .rounded_md()
                    .bg(if is_code { rgb(0x18181a) } else { rgb(0x272729) })
                    .border_1()
                    .border_color(border)
                    .when(is_code, |el| el.font_family("monospace"))
                    .text_xs()
                    .text_color(normal)
                    .child(content.to_string()),
            )
    }

    fn render_scripts_panel(request: Option<&AuraRequest>, border: Rgba, muted: Rgba, normal: Rgba) -> Div {
        let pre_req = request.map(|r| r.scripts.pre_request.as_str()).unwrap_or("");
        let post_resp = request.map(|r| r.scripts.post_response.as_str()).unwrap_or("");
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(Self::render_box("PRE-REQUEST SCRIPT", if pre_req.is_empty() { "// None configured" } else { pre_req }, true, border, muted, normal))
            .child(Self::render_box("POST-RESPONSE SCRIPT", if post_resp.is_empty() { "// None configured" } else { post_resp }, true, border, muted, normal))
    }

    fn render_tests_panel(request: Option<&AuraRequest>, border: Rgba, muted: Rgba, normal: Rgba) -> Div {
        let tests = request.map(|r| r.scripts.tests.as_str()).unwrap_or("");
        let hint = "// describe('API Test', () => {\n//     it('status is 200', () => expect(response.status).toBe(200));\n// });";
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("TEST SUITE (Jasmine / Chai BDD)"))
                    .child(div().text_xs().text_color(rgb(0x28a745)).child("describe, it, expect")),
            )
            .child(Self::render_box("", if tests.is_empty() { hint } else { tests }, true, border, muted, normal))
    }

    fn render_settings_panel(request: Option<&AuraRequest>, border: Rgba, muted: Rgba, normal: Rgba) -> Div {
        let timeout = request.map(|r| r.settings.timeout_ms).unwrap_or(30000);
        let redirects = request.map(|r| r.settings.follow_redirects).unwrap_or(true);
        let ssl = request.map(|r| r.settings.verify_ssl).unwrap_or(true);
        let summary = format!("Timeout: {} ms\nFollow Redirects: {}\nVerify SSL: {}", timeout, if redirects { "Yes" } else { "No" }, if ssl { "Yes" } else { "No" });
        Self::render_box("REQUEST EXECUTION SETTINGS", &summary, false, border, muted, normal)
    }
}
