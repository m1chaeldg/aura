//! RequestEditor View: Sub-tabs (Params, Headers, Auth, Body, Scripts, Tests, Settings) and content panels.

use crate::stores::EditorTab;
use aura_core::model::{AuraRequest, BodyType, HttpMethod};
use gpui::prelude::FluentBuilder;
use gpui::*;

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

    pub fn render<V: 'static>(
        split_ratio: f32,
        is_horizontal: bool,
        _active_title: &str,
        _active_method: HttpMethod,
        active_tab: EditorTab,
        request: Option<&AuraRequest>,
        cx: &mut Context<V>,
        on_select_tab: impl Fn(&mut V, EditorTab, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let border_color = rgb(0x333333);
        let accent_blue = rgb(0x0066cc);
        let bg_surface = rgb(0x272729);
        let text_muted = rgb(0x7a7a7a);

        let headers_count = request.map(|r| r.headers.iter().filter(|h| h.enabled).count()).unwrap_or(0);
        let params_count = request.map(|r| r.params.iter().filter(|p| p.enabled).count()).unwrap_or(0);
        let body_name = request.map(|r| r.body.type_name()).unwrap_or("None");

        div()
            .flex()
            .flex_col()
            .when(is_horizontal, |el| el.w(gpui::relative(split_ratio)))
            .when(!is_horizontal, |el| el.h(gpui::relative(split_ratio)))
            .bg(rgb(0x1d1d1f))
            .border_r_1()
            .border_color(border_color)
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
                            .text_xs()
                            .text_color(if is_active { rgb(0xffffff) } else { text_muted })
                            .child(format!("{}{}", Self::tab_title(tab), badge.unwrap_or_default()))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_select_tab(this, tab, window, cx);
                            }))
                    })),
            )
            .child(
                div()
                    .id("editor-panel-scroll")
                    .flex_1()
                    .p_4()
                    .overflow_y_scroll()
                    .child(Self::render_panel_content(active_tab, request)),
            )
    }

    fn render_panel_content(tab: EditorTab, request: Option<&AuraRequest>) -> impl IntoElement {
        let border = rgb(0x333333);
        let muted = rgb(0x7a7a7a);
        let normal = rgb(0xcccccc);

        match tab {
            EditorTab::Params => Self::render_kv_panel(
                "QUERY PARAMETERS",
                "No query parameters defined. Add key-value pairs to append to the URL.",
                request.map(|r| r.params.iter().map(|p| (p.enabled, format!("{}: {}", p.key, p.value))).collect()).unwrap_or_default(),
                border, muted, normal,
            ),
            EditorTab::Headers => Self::render_kv_panel(
                "REQUEST HEADERS",
                "No custom headers defined. Headers will be inferred automatically.",
                request.map(|r| r.headers.iter().map(|h| (h.enabled, format!("{}: {}", h.key, h.value))).collect()).unwrap_or_default(),
                border, muted, normal,
            ),
            EditorTab::Auth => Self::render_box(
                "AUTHENTICATION CONFIGURATION",
                &request.map(|r| format!("{:?}", r.auth)).unwrap_or_else(|| "No Auth (Inherit / None)".to_string()),
                false, border, muted, normal,
            ),
            EditorTab::Body => Self::render_body_panel(request, border, muted, normal),
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

    fn render_kv_panel(title: &'static str, empty_msg: &'static str, items: Vec<(bool, String)>, border: Rgba, muted: Rgba, normal: Rgba) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_xs().text_color(muted).child(title))
            .when(items.is_empty(), |el| el.child(div().py_3().text_xs().text_color(muted).child(empty_msg)))
            .children(items.into_iter().map(|(enabled, text)| {
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_2()
                    .rounded_md()
                    .bg(rgb(0x272729))
                    .border_1()
                    .border_color(border)
                    .text_xs()
                    .text_color(normal)
                    .child(if enabled { "☑" } else { "☐" })
                    .child(text)
            }))
    }

    fn render_box(title: &'static str, content: &str, is_code: bool, border: Rgba, muted: Rgba, normal: Rgba) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_xs().text_color(muted).child(title))
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

    fn render_body_panel(request: Option<&AuraRequest>, border: Rgba, muted: Rgba, normal: Rgba) -> Div {
        let type_str = request.map(|r| r.body.type_name()).unwrap_or("None");
        let content = match request.map(|r| &r.body) {
            Some(BodyType::Json(s)) | Some(BodyType::Xml(s)) | Some(BodyType::Text(s)) => s.as_str(),
            Some(BodyType::Raw { content, .. }) => content.as_str(),
            _ => "// Empty body or binary format",
        };

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_xs().text_color(muted).child("REQUEST BODY"))
                    .child(div().px_2().py_0p5().rounded_sm().bg(rgb(0x333333)).text_xs().text_color(rgb(0x2997ff)).child(type_str)),
            )
            .child(Self::render_box("", content, true, border, muted, normal))
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
                    .child(div().text_xs().text_color(muted).child("TEST SUITE (Jasmine / Chai BDD)"))
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
