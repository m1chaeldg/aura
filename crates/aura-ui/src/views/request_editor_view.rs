//! RequestEditor View: Modal-free sub-tabs (Params, Headers, Auth, Body, Scripts, Tests, Settings) and content panels.

use crate::app_state::{ActiveInputTarget, ScriptSubTab};
use crate::stores::EditorTab;
use aura_core::model::{AuraRequest, AuthConfig, BodyType, HttpMethod, TestAssertion};
use base64::Engine;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::Selectable;

#[derive(Debug, Clone)]
pub enum RequestEditorAction {
    SelectTab(EditorTab),
    // Headers
    ToggleHeader(usize),
    RemoveHeader(usize),
    OpenAddHeader,
    OpenEditHeader(usize),
    AddHeaderDirect,
    FocusHeaderKey(usize),
    FocusHeaderVal(usize),
    AddPresetHeader(&'static str, &'static str),
    // Params
    ToggleParam(usize),
    RemoveParam(usize),
    OpenAddParam,
    OpenEditParam(usize),
    AddParamDirect,
    FocusParamKey(usize),
    FocusParamVal(usize),
    // Body
    SetBodyType(&'static str),
    OpenEditBody,
    FocusBody,
    FormatJsonBody,
    ClearBody,
    OpenAddFormParam,
    AddFormParamDirect,
    FocusFormParamKey(usize),
    FocusFormParamVal(usize),
    ToggleFormParam(usize),
    RemoveFormParam(usize),
    OpenAddMultipart,
    AddMultipartDirect,
    RemoveMultipart(usize),
    // Auth
    SetAuthType(&'static str),
    OpenEditAuth,
    FocusAuthField(ActiveInputTarget),
    ToggleApiKeyPlacement(bool),
    // Scripts
    SelectScriptSubTab(ScriptSubTab),
    FocusScript(ScriptSubTab),
    InsertScriptSnippet(bool, &'static str),
    // Tests
    FocusTests,
    InsertTestSnippet(&'static str),
    RunTests,
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
        active_input: Option<&ActiveInputTarget>,
        script_sub_tab: ScriptSubTab,
        test_assertions: Option<&[TestAssertion]>,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let border_color = rgb(0x333333);
        let bg_surface = rgb(0x272729);

        let headers_count = request
            .map(|r| r.headers.iter().filter(|h| h.enabled).count())
            .unwrap_or(0);
        let params_count = request
            .map(|r| r.params.iter().filter(|p| p.enabled).count())
            .unwrap_or(0);
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
                    .px_3()
                    .py_1()
                    .bg(bg_surface)
                    .border_b_1()
                    .border_color(border_color)
                    .child({
                        let mut bar = TabBar::new("request-editor-tabs");
                        for tab in Self::available_tabs().iter().copied() {
                            let is_active = tab == active_tab;
                            let badge = match tab {
                                EditorTab::Headers if headers_count > 0 => {
                                    Some(format!(" ({headers_count})"))
                                }
                                EditorTab::Params if params_count > 0 => {
                                    Some(format!(" ({params_count})"))
                                }
                                EditorTab::Body if body_name != "None" => {
                                    Some(format!(" ({body_name})"))
                                }
                                EditorTab::Auth if auth_name != "Inherit" && auth_name != "None" => {
                                    Some(format!(" ({auth_name})"))
                                }
                                _ => None,
                            };
                            let label = format!("{}{}", Self::tab_title(tab), badge.unwrap_or_default());
                            let item = Tab::new()
                                .label(label)
                                .selected(is_active)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_action(this, RequestEditorAction::SelectTab(tab), window, cx);
                                }));
                            bar = bar.child(item);
                        }
                        bar
                    }),
            )
            // Content Panel
            .child(
                div()
                    .id("editor-panel-scroll")
                    .flex_1()
                    .p_4()
                    .overflow_y_scroll()
                    .child(Self::render_panel_content(
                        active_tab,
                        request,
                        active_input,
                        script_sub_tab,
                        test_assertions,
                        cx,
                        on_action,
                    )),
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_panel_content<V: 'static>(
        tab: EditorTab,
        request: Option<&AuraRequest>,
        active_input: Option<&ActiveInputTarget>,
        script_sub_tab: ScriptSubTab,
        test_assertions: Option<&[TestAssertion]>,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let border = rgb(0x333333);
        let muted = rgb(0x7a7a7a);
        let normal = rgb(0xcccccc);

        match tab {
            EditorTab::Headers => Self::render_headers_panel(
                request,
                active_input,
                border,
                muted,
                normal,
                cx,
                on_action,
            ),
            EditorTab::Params => Self::render_params_panel(
                request,
                active_input,
                border,
                muted,
                normal,
                cx,
                on_action,
            ),
            EditorTab::Body => {
                Self::render_body_panel(request, active_input, border, muted, normal, cx, on_action)
            }
            EditorTab::Auth => {
                Self::render_auth_panel(request, active_input, border, muted, normal, cx, on_action)
            }
            EditorTab::Variables => Self::render_box(
                "ENVIRONMENT & REQUEST VARIABLES",
                "Variables referenced with {{var_name}} in URL, headers, and body.",
                false,
                border,
                muted,
                normal,
            ),
            EditorTab::Scripts => Self::render_scripts_panel(
                request,
                active_input,
                script_sub_tab,
                border,
                muted,
                normal,
                cx,
                on_action,
            ),
            EditorTab::Tests => Self::render_tests_panel(
                request,
                active_input,
                test_assertions,
                border,
                muted,
                normal,
                cx,
                on_action,
            ),
            EditorTab::Settings => Self::render_settings_panel(request, border, muted, normal),
        }
    }

    // =========================================================================
    // Headers Panel (Modal-Free Interactive Key-Value Grid)
    // =========================================================================

    fn render_headers_panel<V: 'static>(
        request: Option<&AuraRequest>,
        active_input: Option<&ActiveInputTarget>,
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
            // Title + Action row
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
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child("REQUEST HEADERS"),
                            )
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
                                on_action(this, RequestEditorAction::AddHeaderDirect, window, cx);
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
                    .child(Self::render_preset_chip(
                        "Content-Type: JSON",
                        "Content-Type",
                        "application/json",
                        cx,
                        on_action,
                    ))
                    .child(Self::render_preset_chip(
                        "Accept: */*",
                        "Accept",
                        "*/*",
                        cx,
                        on_action,
                    ))
                    .child(Self::render_preset_chip(
                        "User-Agent: Aura",
                        "User-Agent",
                        "Aura-Client/0.1.0",
                        cx,
                        on_action,
                    )),
            )
            // Empty state or table
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
                        .child(div().text_xs().text_color(muted).child(
                            "No custom headers defined. Click '+ Add Header' or pick a preset.",
                        ))
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
                                    on_action(
                                        this,
                                        RequestEditorAction::AddHeaderDirect,
                                        window,
                                        cx,
                                    );
                                })),
                        ),
                )
            })
            .children(headers.iter().enumerate().map(|(idx, header)| {
                let enabled = header.enabled;
                let key = header.key.clone();
                let val = header.value.clone();
                let is_key_focused =
                    matches!(active_input, Some(ActiveInputTarget::HeaderKey(i)) if *i == idx);
                let is_val_focused =
                    matches!(active_input, Some(ActiveInputTarget::HeaderValue(i)) if *i == idx);

                div()
                    .id(ElementId::NamedInteger("header-row".into(), idx as u64))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1p5()
                    .rounded_md()
                    .bg(if enabled {
                        rgb(0x272729)
                    } else {
                        rgb(0x202022)
                    })
                    .border_1()
                    .border_color(if is_key_focused || is_val_focused {
                        rgb(0x0066cc)
                    } else {
                        border
                    })
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
                    // Inline Editable Key Field
                    .child(
                        div()
                            .id(ElementId::NamedInteger(
                                "edit-header-key".into(),
                                idx as u64,
                            ))
                            .flex_1()
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .bg(if is_key_focused {
                                rgb(0x1d1d1f)
                            } else {
                                rgb(0x222224)
                            })
                            .border_1()
                            .border_color(if is_key_focused {
                                rgb(0x0071e3)
                            } else {
                                rgb(0x38383a)
                            })
                            .cursor_pointer()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(if is_key_focused {
                                rgb(0xffffff)
                            } else if enabled {
                                normal
                            } else {
                                muted
                            })
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_0p5()
                                    .child(if key.is_empty() {
                                        "Header Name".to_string()
                                    } else {
                                        key
                                    })
                                    .when(is_key_focused, |el| {
                                        el.child(div().w(px(2.0)).h(px(12.0)).bg(rgb(0x0066cc)))
                                    }),
                            )
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(
                                    this,
                                    RequestEditorAction::FocusHeaderKey(idx),
                                    window,
                                    cx,
                                );
                            })),
                    )
                    .child(div().text_xs().text_color(muted).child(":"))
                    // Inline Editable Value Field
                    .child(
                        div()
                            .id(ElementId::NamedInteger(
                                "edit-header-val".into(),
                                idx as u64,
                            ))
                            .flex_1()
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .bg(if is_val_focused {
                                rgb(0x1d1d1f)
                            } else {
                                rgb(0x222224)
                            })
                            .border_1()
                            .border_color(if is_val_focused {
                                rgb(0x0071e3)
                            } else {
                                rgb(0x38383a)
                            })
                            .cursor_pointer()
                            .text_xs()
                            .text_color(if is_val_focused {
                                rgb(0xffffff)
                            } else if enabled {
                                normal
                            } else {
                                muted
                            })
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_0p5()
                                    .child(if val.is_empty() {
                                        "Header Value".to_string()
                                    } else {
                                        val
                                    })
                                    .when(is_val_focused, |el| {
                                        el.child(div().w(px(2.0)).h(px(12.0)).bg(rgb(0x0066cc)))
                                    }),
                            )
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(
                                    this,
                                    RequestEditorAction::FocusHeaderVal(idx),
                                    window,
                                    cx,
                                );
                            })),
                    )
                    // Delete Button
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
                on_action(
                    this,
                    RequestEditorAction::AddPresetHeader(key, value),
                    window,
                    cx,
                );
            }))
    }

    // =========================================================================
    // Query Parameters Panel (Modal-Free Interactive Key-Value Grid)
    // =========================================================================

    fn render_params_panel<V: 'static>(
        request: Option<&AuraRequest>,
        active_input: Option<&ActiveInputTarget>,
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
            // Title + Action row
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
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child("QUERY PARAMETERS"),
                            )
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
                                on_action(this, RequestEditorAction::AddParamDirect, window, cx);
                            })),
                    ),
            )
            // Empty state or table
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
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted)
                                .child("No query parameters attached to URL."),
                        )
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
                                    on_action(
                                        this,
                                        RequestEditorAction::AddParamDirect,
                                        window,
                                        cx,
                                    );
                                })),
                        ),
                )
            })
            .children(params.iter().enumerate().map(|(idx, param)| {
                let enabled = param.enabled;
                let key = param.key.clone();
                let val = param.value.clone();
                let is_key_focused =
                    matches!(active_input, Some(ActiveInputTarget::ParamKey(i)) if *i == idx);
                let is_val_focused =
                    matches!(active_input, Some(ActiveInputTarget::ParamValue(i)) if *i == idx);

                div()
                    .id(ElementId::NamedInteger("param-row".into(), idx as u64))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1p5()
                    .rounded_md()
                    .bg(if enabled {
                        rgb(0x272729)
                    } else {
                        rgb(0x202022)
                    })
                    .border_1()
                    .border_color(if is_key_focused || is_val_focused {
                        rgb(0x0066cc)
                    } else {
                        border
                    })
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
                    // Inline Editable Key Field
                    .child(
                        div()
                            .id(ElementId::NamedInteger("edit-param-key".into(), idx as u64))
                            .flex_1()
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .bg(if is_key_focused {
                                rgb(0x1d1d1f)
                            } else {
                                rgb(0x222224)
                            })
                            .border_1()
                            .border_color(if is_key_focused {
                                rgb(0x0071e3)
                            } else {
                                rgb(0x38383a)
                            })
                            .cursor_pointer()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(if is_key_focused {
                                rgb(0xffffff)
                            } else if enabled {
                                normal
                            } else {
                                muted
                            })
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_0p5()
                                    .child(if key.is_empty() {
                                        "Key".to_string()
                                    } else {
                                        key
                                    })
                                    .when(is_key_focused, |el| {
                                        el.child(div().w(px(2.0)).h(px(12.0)).bg(rgb(0x0066cc)))
                                    }),
                            )
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(
                                    this,
                                    RequestEditorAction::FocusParamKey(idx),
                                    window,
                                    cx,
                                );
                            })),
                    )
                    .child(div().text_xs().text_color(muted).child("="))
                    // Inline Editable Value Field
                    .child(
                        div()
                            .id(ElementId::NamedInteger("edit-param-val".into(), idx as u64))
                            .flex_1()
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .bg(if is_val_focused {
                                rgb(0x1d1d1f)
                            } else {
                                rgb(0x222224)
                            })
                            .border_1()
                            .border_color(if is_val_focused {
                                rgb(0x0071e3)
                            } else {
                                rgb(0x38383a)
                            })
                            .cursor_pointer()
                            .text_xs()
                            .text_color(if is_val_focused {
                                rgb(0xffffff)
                            } else if enabled {
                                normal
                            } else {
                                muted
                            })
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_0p5()
                                    .child(if val.is_empty() {
                                        "Value".to_string()
                                    } else {
                                        val
                                    })
                                    .when(is_val_focused, |el| {
                                        el.child(div().w(px(2.0)).h(px(12.0)).bg(rgb(0x0066cc)))
                                    }),
                            )
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(
                                    this,
                                    RequestEditorAction::FocusParamVal(idx),
                                    window,
                                    cx,
                                );
                            })),
                    )
                    // Delete Button
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
                    )
            }))
    }

    // =========================================================================
    // Request Body Panel (Full-Height Direct Monospace Editor + Actions)
    // =========================================================================

    fn render_body_panel<V: 'static>(
        request: Option<&AuraRequest>,
        active_input: Option<&ActiveInputTarget>,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let body = request.map(|r| &r.body);
        let active_type = body.map(|b| b.type_name()).unwrap_or("None");
        const FORMATS: [&str; 8] = [
            "None",
            "JSON",
            "XML",
            "Text",
            "Raw",
            "FormUrlEncoded",
            "Multipart",
            "Binary",
        ];

        div()
            .flex()
            .flex_col()
            .gap_3()
            // Format Selector Bar
            .child(div().flex().items_center().gap_1().flex_wrap().children(
                FORMATS.iter().copied().map(|fmt| {
                    let is_active = match fmt {
                        "FormUrlEncoded" => {
                            active_type == "FormUrlEncoded" || active_type == "Form"
                        }
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
                        .bg(if is_active {
                            rgb(0x0066cc)
                        } else {
                            rgb(0x272729)
                        })
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
                }),
            ))
            // Body Content based on Format
            .child(match body {
                Some(BodyType::None) | None => div()
                    .p_6()
                    .rounded_md()
                    .bg(rgb(0x222224))
                    .border_1()
                    .border_color(border)
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted)
                            .child("This request does not have a body."),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x555555))
                            .child("Select JSON, Form, or Raw above to send payload data."),
                    ),
                Some(BodyType::Json(content)) => Self::render_full_height_body_editor(
                    "JSON (application/json)",
                    content,
                    true,
                    active_input,
                    border,
                    muted,
                    normal,
                    cx,
                    on_action,
                ),
                Some(BodyType::Xml(content)) => Self::render_full_height_body_editor(
                    "XML (application/xml)",
                    content,
                    false,
                    active_input,
                    border,
                    muted,
                    normal,
                    cx,
                    on_action,
                ),
                Some(BodyType::Text(content)) => Self::render_full_height_body_editor(
                    "Text (text/plain)",
                    content,
                    false,
                    active_input,
                    border,
                    muted,
                    normal,
                    cx,
                    on_action,
                ),
                Some(BodyType::Raw {
                    content,
                    content_type,
                }) => {
                    let label =
                        format!("Raw ({})", content_type.as_deref().unwrap_or("text/plain"));
                    Self::render_full_height_body_editor(
                        &label,
                        content,
                        false,
                        active_input,
                        border,
                        muted,
                        normal,
                        cx,
                        on_action,
                    )
                }
                Some(BodyType::FormUrlEncoded(items)) => Self::render_form_urlencoded_editor(
                    items,
                    active_input,
                    border,
                    muted,
                    normal,
                    cx,
                    on_action,
                ),
                Some(BodyType::Form(map)) => {
                    let items: Vec<aura_core::model::ParamItem> = map
                        .iter()
                        .map(|(k, v)| aura_core::model::ParamItem::new(k.clone(), v.clone()))
                        .collect();
                    Self::render_form_urlencoded_editor(
                        &items,
                        active_input,
                        border,
                        muted,
                        normal,
                        cx,
                        on_action,
                    )
                }
                Some(BodyType::Multipart(parts)) => {
                    Self::render_multipart_editor(parts, border, muted, normal, cx, on_action)
                }
                Some(BodyType::Binary {
                    file_path,
                    content_type,
                }) => {
                    let ct = content_type
                        .as_deref()
                        .unwrap_or("application/octet-stream");
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .p_4()
                        .rounded_md()
                        .bg(rgb(0x222224))
                        .border_1()
                        .border_color(border)
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(muted)
                                .child(format!("BINARY FILE ({ct})")),
                        )
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
                                .child(div().text_xs().text_color(normal).child(
                                    if file_path.is_empty() {
                                        "No binary file selected".to_string()
                                    } else {
                                        file_path.clone()
                                    },
                                )),
                        )
                }
            })
    }

    #[allow(clippy::too_many_arguments)]
    fn render_full_height_body_editor<V: 'static>(
        label: &str,
        content: &str,
        is_json: bool,
        active_input: Option<&ActiveInputTarget>,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let is_focused = matches!(active_input, Some(ActiveInputTarget::BodyText));
        let byte_len = content.len();

        div()
            .flex()
            .flex_col()
            .gap_2()
            // Top Toolbar: Label + Format JSON + Clear + Size indicator
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
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child(label.to_string()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x555555))
                                    .child(format!("({byte_len} bytes)")),
                            ),
                    )
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
                                            on_action(
                                                this,
                                                RequestEditorAction::FormatJsonBody,
                                                window,
                                                cx,
                                            );
                                        })),
                                )
                            })
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
            // Full-Height Direct Monospace Textarea
            .child(
                div()
                    .id("direct-body-editor")
                    .p_3()
                    .min_h(px(260.0))
                    .rounded_md()
                    .bg(rgb(0x18181a))
                    .border_1()
                    .border_color(if is_focused { rgb(0x0071e3) } else { border })
                    .font_family("monospace")
                    .text_xs()
                    .text_color(if is_focused { rgb(0xffffff) } else { normal })
                    .cursor_pointer()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(if content.is_empty() {
                                div()
                                    .text_color(rgb(0x555555))
                                    .child("// Click to edit request body payload...")
                            } else {
                                div().child(content.to_string())
                            })
                            .when(is_focused, |el| {
                                el.child(div().w(px(2.0)).h(px(14.0)).bg(rgb(0x0066cc)))
                            }),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        on_action(this, RequestEditorAction::FocusBody, window, cx);
                    })),
            )
    }

    fn render_form_urlencoded_editor<V: 'static>(
        items: &[aura_core::model::ParamItem],
        active_input: Option<&ActiveInputTarget>,
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
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(muted)
                            .child("X-WWW-FORM-URLENCODED FIELDS"),
                    )
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
                                on_action(
                                    this,
                                    RequestEditorAction::AddFormParamDirect,
                                    window,
                                    cx,
                                );
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
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted)
                                .child("No form fields defined. Click '+ Add Field' to start."),
                        ),
                )
            })
            .children(items.iter().enumerate().map(|(idx, item)| {
                let enabled = item.enabled;
                let k = item.key.clone();
                let v = item.value.clone();
                let is_key_focused =
                    matches!(active_input, Some(ActiveInputTarget::FormParamKey(i)) if *i == idx);
                let is_val_focused =
                    matches!(active_input, Some(ActiveInputTarget::FormParamValue(i)) if *i == idx);

                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1p5()
                    .rounded_md()
                    .bg(rgb(0x272729))
                    .border_1()
                    .border_color(if is_key_focused || is_val_focused {
                        rgb(0x0066cc)
                    } else {
                        border
                    })
                    .child(
                        div()
                            .id(ElementId::NamedInteger(
                                "toggle-form-param".into(),
                                idx as u64,
                            ))
                            .cursor_pointer()
                            .text_sm()
                            .text_color(if enabled { rgb(0x28a745) } else { muted })
                            .child(if enabled { "☑" } else { "☐" })
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(
                                    this,
                                    RequestEditorAction::ToggleFormParam(idx),
                                    window,
                                    cx,
                                );
                            })),
                    )
                    .child(
                        div()
                            .id(ElementId::NamedInteger("edit-form-key".into(), idx as u64))
                            .flex_1()
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .bg(if is_key_focused {
                                rgb(0x1d1d1f)
                            } else {
                                rgb(0x222224)
                            })
                            .border_1()
                            .border_color(if is_key_focused {
                                rgb(0x0071e3)
                            } else {
                                rgb(0x38383a)
                            })
                            .cursor_pointer()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(if is_key_focused {
                                rgb(0xffffff)
                            } else {
                                normal
                            })
                            .child(if k.is_empty() {
                                "Field Key".to_string()
                            } else {
                                k
                            })
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(
                                    this,
                                    RequestEditorAction::FocusFormParamKey(idx),
                                    window,
                                    cx,
                                );
                            })),
                    )
                    .child(div().text_xs().text_color(muted).child("="))
                    .child(
                        div()
                            .id(ElementId::NamedInteger("edit-form-val".into(), idx as u64))
                            .flex_1()
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .bg(if is_val_focused {
                                rgb(0x1d1d1f)
                            } else {
                                rgb(0x222224)
                            })
                            .border_1()
                            .border_color(if is_val_focused {
                                rgb(0x0071e3)
                            } else {
                                rgb(0x38383a)
                            })
                            .cursor_pointer()
                            .text_xs()
                            .text_color(if is_val_focused {
                                rgb(0xffffff)
                            } else {
                                normal
                            })
                            .child(if v.is_empty() {
                                "Field Value".to_string()
                            } else {
                                v
                            })
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(
                                    this,
                                    RequestEditorAction::FocusFormParamVal(idx),
                                    window,
                                    cx,
                                );
                            })),
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
                                on_action(
                                    this,
                                    RequestEditorAction::RemoveFormParam(idx),
                                    window,
                                    cx,
                                );
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
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(muted)
                            .child("MULTIPART/FORM-DATA FIELDS"),
                    )
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
                                on_action(
                                    this,
                                    RequestEditorAction::AddMultipartDirect,
                                    window,
                                    cx,
                                );
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
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted)
                                .child("No multipart fields defined. Click '+ Add Part' to start."),
                        ),
                )
            })
            .children(parts.iter().enumerate().map(|(idx, part)| {
                let name = part.name.clone();
                let is_file = part.file_path.is_some();
                let display_val = part
                    .value
                    .clone()
                    .or_else(|| part.file_path.clone())
                    .unwrap_or_default();

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
                                    .bg(if is_file {
                                        rgb(0x8e44ad)
                                    } else {
                                        rgb(0x0066cc)
                                    })
                                    .text_xs()
                                    .text_color(rgb(0xffffff))
                                    .child(if is_file { "FILE" } else { "TEXT" }),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xffffff))
                                    .child(name),
                            )
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
                                on_action(
                                    this,
                                    RequestEditorAction::RemoveMultipart(idx),
                                    window,
                                    cx,
                                );
                            })),
                    )
            }))
    }

    // =========================================================================
    // Authentication Panel (Modal-Free Full Space Inputs + Previews)
    // =========================================================================

    fn render_auth_panel<V: 'static>(
        request: Option<&AuraRequest>,
        active_input: Option<&ActiveInputTarget>,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let auth = request.map(|r| &r.auth);
        let active_auth_type = auth.map(|a| a.type_name()).unwrap_or("Inherit");
        const STRATEGIES: [&str; 7] = [
            "Inherit",
            "None",
            "Bearer Token",
            "Basic Auth",
            "API Key",
            "AWS SigV4",
            "OAuth 2.0",
        ];

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
            // Direct Inline Auth Configuration
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
                    let is_token_focused = matches!(active_input, Some(ActiveInputTarget::AuthBearerToken));
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
                                .flex_col()
                                .gap_1()
                                .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("BEARER TOKEN / JWT"))
                                .child(
                                    div()
                                        .id("input-bearer-token")
                                        .p_3()
                                        .min_h(px(50.0))
                                        .rounded_md()
                                        .bg(rgb(0x18181a))
                                        .border_1()
                                        .border_color(if is_token_focused { rgb(0x0071e3) } else { border })
                                        .font_family("monospace")
                                        .text_xs()
                                        .text_color(if is_token_focused { rgb(0xffffff) } else { normal })
                                        .cursor_pointer()
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_1()
                                                .child(if token.is_empty() {
                                                    div().text_color(rgb(0x555555)).child("Enter Bearer token or paste JWT here...")
                                                } else {
                                                    div().child(token.clone())
                                                })
                                                .when(is_token_focused, |el| el.child(div().w(px(2.0)).h(px(14.0)).bg(rgb(0x0066cc)))),
                                        )
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(this, RequestEditorAction::FocusAuthField(ActiveInputTarget::AuthBearerToken), window, cx);
                                        })),
                                ),
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
                                        el.child(div().text_xs().text_color(muted).child(format!("Subject (sub): {sub}")))
                                    })
                                    .when_some(jwt.issuer, |el, iss| {
                                        el.child(div().text_xs().text_color(muted).child(format!("Issuer (iss): {iss}")))
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
                    let is_user_focused = matches!(active_input, Some(ActiveInputTarget::AuthBasicUsername));
                    let is_pass_focused = matches!(active_input, Some(ActiveInputTarget::AuthBasicPassword));
                    let preview = {
                        let encoded = base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
                        format!("Authorization: Basic {encoded}")
                    };

                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("BASIC AUTHENTICATION"))
                        .child(
                            div()
                                .flex()
                                .gap_3()
                                // Username input
                                .child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(div().text_xs().text_color(muted).child("Username"))
                                        .child(
                                            div()
                                                .id("input-basic-username")
                                                .px_3()
                                                .py_2()
                                                .rounded_sm()
                                                .bg(rgb(0x18181a))
                                                .border_1()
                                                .border_color(if is_user_focused { rgb(0x0071e3) } else { border })
                                                .text_xs()
                                                .text_color(if is_user_focused { rgb(0xffffff) } else { normal })
                                                .cursor_pointer()
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap_1()
                                                        .child(if username.is_empty() {
                                                            div().text_color(rgb(0x555555)).child("Enter username...")
                                                        } else {
                                                            div().child(username.clone())
                                                        })
                                                        .when(is_user_focused, |el| el.child(div().w(px(2.0)).h(px(14.0)).bg(rgb(0x0066cc)))),
                                                )
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    on_action(this, RequestEditorAction::FocusAuthField(ActiveInputTarget::AuthBasicUsername), window, cx);
                                                })),
                                        ),
                                )
                                // Password input
                                .child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(div().text_xs().text_color(muted).child("Password"))
                                        .child(
                                            div()
                                                .id("input-basic-password")
                                                .px_3()
                                                .py_2()
                                                .rounded_sm()
                                                .bg(rgb(0x18181a))
                                                .border_1()
                                                .border_color(if is_pass_focused { rgb(0x0071e3) } else { border })
                                                .text_xs()
                                                .text_color(if is_pass_focused { rgb(0xffffff) } else { normal })
                                                .cursor_pointer()
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap_1()
                                                        .child(if password.is_empty() {
                                                            div().text_color(rgb(0x555555)).child("Enter password...")
                                                        } else {
                                                            div().child("••••••••".to_string())
                                                        })
                                                        .when(is_pass_focused, |el| el.child(div().w(px(2.0)).h(px(14.0)).bg(rgb(0x0066cc)))),
                                                )
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    on_action(this, RequestEditorAction::FocusAuthField(ActiveInputTarget::AuthBasicPassword), window, cx);
                                                })),
                                        ),
                                ),
                        )
                        // Live Base64 Preview
                        .child(
                            div()
                                .p_2p5()
                                .rounded_sm()
                                .bg(rgb(0x18181a))
                                .border_1()
                                .border_color(rgb(0x28a745))
                                .font_family("monospace")
                                .text_xs()
                                .text_color(rgb(0x28a745))
                                .child(preview),
                        )
                }
                Some(AuthConfig::ApiKey { key, value, in_header }) => {
                    let in_hdr = *in_header;
                    let is_key_focused = matches!(active_input, Some(ActiveInputTarget::AuthApiKeyName));
                    let is_val_focused = matches!(active_input, Some(ActiveInputTarget::AuthApiKeyValue));

                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("API KEY AUTHENTICATION"))
                        .child(
                            div()
                                .flex()
                                .gap_3()
                                // Key Name
                                .child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(div().text_xs().text_color(muted).child("Key Name"))
                                        .child(
                                            div()
                                                .id("input-apikey-name")
                                                .px_3()
                                                .py_2()
                                                .rounded_sm()
                                                .bg(rgb(0x18181a))
                                                .border_1()
                                                .border_color(if is_key_focused { rgb(0x0071e3) } else { border })
                                                .text_xs()
                                                .text_color(if is_key_focused { rgb(0xffffff) } else { normal })
                                                .cursor_pointer()
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap_1()
                                                        .child(if key.is_empty() {
                                                            div().text_color(rgb(0x555555)).child("e.g. X-API-Key")
                                                        } else {
                                                            div().child(key.clone())
                                                        })
                                                        .when(is_key_focused, |el| el.child(div().w(px(2.0)).h(px(14.0)).bg(rgb(0x0066cc)))),
                                                )
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    on_action(this, RequestEditorAction::FocusAuthField(ActiveInputTarget::AuthApiKeyName), window, cx);
                                                })),
                                        ),
                                )
                                // Key Value
                                .child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(div().text_xs().text_color(muted).child("Key Value"))
                                        .child(
                                            div()
                                                .id("input-apikey-val")
                                                .px_3()
                                                .py_2()
                                                .rounded_sm()
                                                .bg(rgb(0x18181a))
                                                .border_1()
                                                .border_color(if is_val_focused { rgb(0x0071e3) } else { border })
                                                .text_xs()
                                                .text_color(if is_val_focused { rgb(0xffffff) } else { normal })
                                                .cursor_pointer()
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap_1()
                                                        .child(if value.is_empty() {
                                                            div().text_color(rgb(0x555555)).child("secret_key_123...")
                                                        } else {
                                                            div().child(value.clone())
                                                        })
                                                        .when(is_val_focused, |el| el.child(div().w(px(2.0)).h(px(14.0)).bg(rgb(0x0066cc)))),
                                                )
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    on_action(this, RequestEditorAction::FocusAuthField(ActiveInputTarget::AuthApiKeyValue), window, cx);
                                                })),
                                        ),
                                ),
                        )
                        // Placement Toggle
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
                        )
                }
                Some(AuthConfig::AwsSigV4 { access_key, secret_key, region, service, session_token }) => {
                    let fields = [
                        ("Access Key", access_key.as_str(), 0),
                        ("Secret Key", secret_key.as_str(), 1),
                        ("Region", region.as_str(), 2),
                        ("Service", service.as_str(), 3),
                        ("Session Token", session_token.as_deref().unwrap_or(""), 4),
                    ];

                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("AWS SIGNATURE V4"))
                        .children(fields.into_iter().map(|(label, val, f_idx)| {
                            let is_f = matches!(active_input, Some(ActiveInputTarget::AuthAwsField(i)) if *i == f_idx);
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(div().text_xs().text_color(muted).child(label))
                                .child(
                                    div()
                                        .id(ElementId::NamedInteger("input-aws-field".into(), f_idx as u64))
                                        .px_3()
                                        .py_1p5()
                                        .rounded_sm()
                                        .bg(rgb(0x18181a))
                                        .border_1()
                                        .border_color(if is_f { rgb(0x0071e3) } else { border })
                                        .text_xs()
                                        .text_color(if is_f { rgb(0xffffff) } else { normal })
                                        .cursor_pointer()
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_1()
                                                .child(if val.is_empty() {
                                                    div().text_color(rgb(0x555555)).child(format!("Enter {label}..."))
                                                } else {
                                                    div().child(val.to_string())
                                                })
                                                .when(is_f, |el| el.child(div().w(px(2.0)).h(px(14.0)).bg(rgb(0x0066cc)))),
                                        )
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(this, RequestEditorAction::FocusAuthField(ActiveInputTarget::AuthAwsField(f_idx)), window, cx);
                                        })),
                                )
                        }))
                }
                Some(AuthConfig::OAuth2 { grant_type, access_token, token_url, client_id, client_secret }) => {
                    let fields = [
                        ("Access Token", access_token.as_str(), 0),
                        ("Grant Type", grant_type.as_str(), 1),
                        ("Token URL", token_url.as_deref().unwrap_or(""), 2),
                        ("Client ID", client_id.as_deref().unwrap_or(""), 3),
                        ("Client Secret", client_secret.as_deref().unwrap_or(""), 4),
                    ];

                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("OAUTH 2.0"))
                        .children(fields.into_iter().map(|(label, val, f_idx)| {
                            let is_f = matches!(active_input, Some(ActiveInputTarget::AuthOAuth2Field(i)) if *i == f_idx);
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(div().text_xs().text_color(muted).child(label))
                                .child(
                                    div()
                                        .id(ElementId::NamedInteger("input-oauth-field".into(), f_idx as u64))
                                        .px_3()
                                        .py_1p5()
                                        .rounded_sm()
                                        .bg(rgb(0x18181a))
                                        .border_1()
                                        .border_color(if is_f { rgb(0x0071e3) } else { border })
                                        .text_xs()
                                        .text_color(if is_f { rgb(0xffffff) } else { normal })
                                        .cursor_pointer()
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_1()
                                                .child(if val.is_empty() {
                                                    div().text_color(rgb(0x555555)).child(format!("Enter {label}..."))
                                                } else {
                                                    div().child(val.to_string())
                                                })
                                                .when(is_f, |el| el.child(div().w(px(2.0)).h(px(14.0)).bg(rgb(0x0066cc)))),
                                        )
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(this, RequestEditorAction::FocusAuthField(ActiveInputTarget::AuthOAuth2Field(f_idx)), window, cx);
                                        })),
                                )
                        }))
                }
            })
    }

    // =========================================================================
    // Scripts Panel (Sub-tabs for Pre & Post Scripts + Quick Snippets)
    // =========================================================================

    #[allow(clippy::too_many_arguments)]
    fn render_scripts_panel<V: 'static>(
        request: Option<&AuraRequest>,
        active_input: Option<&ActiveInputTarget>,
        sub_tab: ScriptSubTab,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let is_pre = sub_tab == ScriptSubTab::PreRequest;
        let script_content = request
            .map(|r| {
                if is_pre {
                    r.scripts.pre_request.as_str()
                } else {
                    r.scripts.post_response.as_str()
                }
            })
            .unwrap_or("");
        let is_focused = if is_pre {
            matches!(active_input, Some(ActiveInputTarget::PreRequestScript))
        } else {
            matches!(active_input, Some(ActiveInputTarget::PostResponseScript))
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            // Sub-tabs Bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .border_b_1()
                    .border_color(border)
                    .pb_2()
                    .child(
                        div()
                            .id("subtab-pre-request")
                            .px_3()
                            .py_1()
                            .rounded_sm()
                            .bg(if is_pre { rgb(0x0066cc) } else { rgb(0x272729) })
                            .hover(|s| s.bg(rgb(0x333333)))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(if is_pre { rgb(0xffffff) } else { muted })
                            .child("Pre-request Script")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::SelectScriptSubTab(ScriptSubTab::PreRequest), window, cx);
                            })),
                    )
                    .child(
                        div()
                            .id("subtab-post-response")
                            .px_3()
                            .py_1()
                            .rounded_sm()
                            .bg(if !is_pre { rgb(0x0066cc) } else { rgb(0x272729) })
                            .hover(|s| s.bg(rgb(0x333333)))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(if !is_pre { rgb(0xffffff) } else { muted })
                            .child("Post-response Script")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::SelectScriptSubTab(ScriptSubTab::PostResponse), window, cx);
                            })),
                    ),
            )
            // Snippets Row
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(div().text_xs().text_color(muted).child("Snippets:"))
                    .child(Self::render_snippet_chip("+ Set Env Var", is_pre, "aura.environment.set(\"key\", \"value\");\n", cx, on_action))
                    .child(Self::render_snippet_chip("+ Get Env Var", is_pre, "const val = aura.environment.get(\"key\");\n", cx, on_action))
                    .child(Self::render_snippet_chip("+ Timestamp", is_pre, "aura.variables.set(\"timestamp\", Date.now());\n", cx, on_action))
                    .child(Self::render_snippet_chip("+ Log Info", is_pre, "console.log(\"Executing script at:\", new Date());\n", cx, on_action)),
            )
            // Full-Height Multiline Code Area
            .child(
                div()
                    .id("direct-script-editor")
                    .p_3()
                    .min_h(px(260.0))
                    .rounded_md()
                    .bg(rgb(0x18181a))
                    .border_1()
                    .border_color(if is_focused { rgb(0x0071e3) } else { border })
                    .font_family("monospace")
                    .text_xs()
                    .text_color(if is_focused { rgb(0xffffff) } else { normal })
                    .cursor_pointer()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(if script_content.is_empty() {
                                div().text_color(rgb(0x555555)).child(if is_pre {
                                    "// Write pre-request script here (runs before sending)..."
                                } else {
                                    "// Write post-response script here (runs after receiving response)..."
                                })
                            } else {
                                div().child(script_content.to_string())
                            })
                            .when(is_focused, |el| el.child(div().w(px(2.0)).h(px(14.0)).bg(rgb(0x0066cc)))),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        on_action(this, RequestEditorAction::FocusScript(sub_tab), window, cx);
                    })),
            )
    }

    fn render_snippet_chip<V: 'static>(
        label: &'static str,
        is_pre: bool,
        snippet: &'static str,
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
                on_action(
                    this,
                    RequestEditorAction::InsertScriptSnippet(is_pre, snippet),
                    window,
                    cx,
                );
            }))
    }

    // =========================================================================
    // Tests Panel (Full Test Editor + Snippets + '▶ Run Tests')
    // =========================================================================

    #[allow(clippy::too_many_arguments)]
    fn render_tests_panel<V: 'static>(
        request: Option<&AuraRequest>,
        active_input: Option<&ActiveInputTarget>,
        assertions: Option<&[TestAssertion]>,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RequestEditorAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let tests = request.map(|r| r.scripts.tests.as_str()).unwrap_or("");
        let is_focused = matches!(active_input, Some(ActiveInputTarget::TestScript));

        let total_tests = assertions.map(|a| a.len()).unwrap_or(0);
        let passed_tests = assertions
            .map(|a| a.iter().filter(|x| x.passed).count())
            .unwrap_or(0);
        let has_results = total_tests > 0;

        div()
            .flex()
            .flex_col()
            .gap_3()
            // Top Bar: Title + Results Pill + '▶ Run Tests' Button
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
                            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("TEST SUITE (Jasmine / Chai BDD)"))
                            .when(has_results, |el| {
                                let all_passed = passed_tests == total_tests;
                                el.child(
                                    div()
                                        .px_2()
                                        .py_0p5()
                                        .rounded_sm()
                                        .bg(if all_passed { rgb(0x28a745) } else { rgb(0xe74c3c) })
                                        .text_xs()
                                        .text_color(rgb(0xffffff))
                                        .child(format!("{passed_tests}/{total_tests} Passed")),
                                )
                            }),
                    )
                    .child(
                        div()
                            .id("btn-run-tests")
                            .flex()
                            .items_center()
                            .gap_1()
                            .px_3()
                            .py_1()
                            .rounded_sm()
                            .bg(rgb(0x28a745))
                            .hover(|s| s.bg(rgb(0x218838)))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(rgb(0xffffff))
                            .child("▶ Run Tests")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                on_action(this, RequestEditorAction::RunTests, window, cx);
                            })),
                    ),
            )
            // Snippets Row
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(div().text_xs().text_color(muted).child("Snippets:"))
                    .child(Self::render_test_snippet_chip(
                        "+ Status is 200",
                        "describe('Status Code', () => {\n    it('is 200 OK', () => expect(response.status).toBe(200));\n});\n",
                        cx, on_action,
                    ))
                    .child(Self::render_test_snippet_chip(
                        "+ JSON Field Check",
                        "describe('Response Payload', () => {\n    it('has data field', () => {\n        const data = JSON.parse(response.body);\n        expect(data).toBeDefined();\n    });\n});\n",
                        cx, on_action,
                    ))
                    .child(Self::render_test_snippet_chip(
                        "+ Response Time < 500ms",
                        "describe('Performance', () => {\n    it('responds in under 500ms', () => expect(response.duration_ms).toBeLessThan(500));\n});\n",
                        cx, on_action,
                    ))
                    .child(Self::render_test_snippet_chip(
                        "+ Content-Type Header",
                        "describe('Headers', () => {\n    it('is application/json', () => expect(response.headers['content-type']).toContain('application/json'));\n});\n",
                        cx, on_action,
                    )),
            )
            // Full-Height Multiline Code Editor
            .child(
                div()
                    .id("direct-test-editor")
                    .p_3()
                    .min_h(px(260.0))
                    .rounded_md()
                    .bg(rgb(0x18181a))
                    .border_1()
                    .border_color(if is_focused { rgb(0x0071e3) } else { border })
                    .font_family("monospace")
                    .text_xs()
                    .text_color(if is_focused { rgb(0xffffff) } else { normal })
                    .cursor_pointer()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(if tests.is_empty() {
                                div().text_color(rgb(0x555555)).child("// Write Jasmine/Chai BDD assertions: describe(), it(), expect()...")
                            } else {
                                div().child(tests.to_string())
                            })
                            .when(is_focused, |el| el.child(div().w(px(2.0)).h(px(14.0)).bg(rgb(0x0066cc)))),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        on_action(this, RequestEditorAction::FocusTests, window, cx);
                    })),
            )
    }

    fn render_test_snippet_chip<V: 'static>(
        label: &'static str,
        snippet: &'static str,
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
                on_action(
                    this,
                    RequestEditorAction::InsertTestSnippet(snippet),
                    window,
                    cx,
                );
            }))
    }

    fn render_box(
        title: &'static str,
        content: &str,
        is_code: bool,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
    ) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(muted)
                    .child(title),
            )
            .child(
                div()
                    .p_3()
                    .rounded_md()
                    .bg(if is_code {
                        rgb(0x18181a)
                    } else {
                        rgb(0x272729)
                    })
                    .border_1()
                    .border_color(border)
                    .when(is_code, |el| el.font_family("monospace"))
                    .text_xs()
                    .text_color(normal)
                    .child(content.to_string()),
            )
    }

    fn render_settings_panel(
        request: Option<&AuraRequest>,
        border: Rgba,
        muted: Rgba,
        normal: Rgba,
    ) -> Div {
        let timeout = request.map(|r| r.settings.timeout_ms).unwrap_or(30000);
        let redirects = request.map(|r| r.settings.follow_redirects).unwrap_or(true);
        let ssl = request.map(|r| r.settings.verify_ssl).unwrap_or(true);
        let summary = format!(
            "Timeout: {timeout} ms\nFollow Redirects: {}\nVerify SSL: {}",
            if redirects { "Yes" } else { "No" },
            if ssl { "Yes" } else { "No" }
        );
        Self::render_box(
            "REQUEST EXECUTION SETTINGS",
            &summary,
            false,
            border,
            muted,
            normal,
        )
    }
}
