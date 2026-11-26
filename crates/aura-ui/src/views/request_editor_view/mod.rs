//! RequestEditor View: Modal-free sub-tabs (Params, Headers, Auth, Body, Scripts, Tests, Settings) and content panels.

use crate::app_state::{ActiveInputTarget, ScriptSubTab};
use crate::input_registry::InputRegistry;
use crate::stores::EditorTab;
use aura_core::model::{AuraRequest, HttpMethod, TestAssertion};
use gpui::*;
use gpui_kit::component::input::{Input, InputState, Textarea, TextareaState};
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{Selectable, Sizable};


mod auth_panel;
mod body_panel;
mod form_body_panel;
mod headers_panel;
mod params_panel;
mod scripts_panel;
mod settings_panel;
mod spec_panel;
mod tests_panel;
mod variables_panel;
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
    // Variables
    AddVariableDirect,
    ToggleVariable(usize),
    RemoveVariable(usize),
    // Settings
    ToggleSettingFollowRedirects,
    ToggleSettingVerifySsl,
    ToggleSettingInheritAuth,
    // Spec
    FocusSpec,
    FormatSpec,
    CopySpec,
    PasteSpec,
    RevertSpec,
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
            EditorTab::Spec,
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
            EditorTab::Spec => "Spec",
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render<V: 'static>(
        _split_ratio: f32,
        _is_horizontal: bool,
        _active_title: &str,
        _active_method: HttpMethod,
        active_tab: EditorTab,
        request: Option<&AuraRequest>,
        variable_rows: &[(String, String, bool)],
        setting_timeout_error: Option<&str>,
        setting_max_redirects_error: Option<&str>,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        script_sub_tab: ScriptSubTab,
        test_assertions: Option<&[TestAssertion]>,
        spec_buffer: Option<&str>,
        spec_error: Option<&str>,
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
        let vars_count = variable_rows.len();

        div()
            .flex()
            .flex_col()
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
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
                                EditorTab::Variables if vars_count > 0 => {
                                    Some(format!(" ({vars_count})"))
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
                        variable_rows,
                        setting_timeout_error,
                        setting_max_redirects_error,
                        active_input,
                        inputs,
                        script_sub_tab,
                        test_assertions,
                        spec_buffer,
                        spec_error,
                        cx,
                        on_action,
                    )),
            )
    }

    pub(super) fn render_inline_input(
        id: impl Into<ElementId>,
        input: Option<&Entity<InputState>>,
        fallback: &str,
        _is_focused: bool,
        mask_toggle: bool,
        bg: Rgba,
        border: Rgba,
        text_color: Rgba,
    ) -> AnyElement {
        if let Some(state) = input {
            let mut inp = Input::new(state)
                .id(id)
                .xsmall()
                .flex_1()
                .w_full();
            if mask_toggle {
                inp = inp.mask_toggle();
            }
            div().flex_1().child(inp).into_any_element()
        } else {
            div()
                .id(id)
                .flex_1()
                .px_2()
                .py_0p5()
                .rounded_sm()
                .bg(bg)
                .border_1()
                .border_color(border)
                .text_xs()
                .text_color(text_color)
                .child(fallback.to_string())
                .into_any_element()
        }
    }

    pub(super) fn render_inline_textarea(
        id: impl Into<ElementId>,
        ta: Option<&Entity<TextareaState>>,
        fallback: &str,
        height: Pixels,
        _is_focused: bool,
        border: Rgba,
        text_color: Rgba,
    ) -> AnyElement {
        if let Some(state) = ta {
            Textarea::new(state)
                .h(height)
                .w_full()
                .font_family("monospace")
                .text_xs()
                .into_any_element()
        } else {
            div()
                .id(id)
                .p_2()
                .rounded_md()
                .bg(rgb(0x18181a))
                .border_1()
                .border_color(border)
                .min_h(height)
                .font_family("monospace")
                .text_xs()
                .text_color(text_color)
                .child(fallback.to_string())
                .into_any_element()
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_panel_content<V: 'static>(
        tab: EditorTab,
        request: Option<&AuraRequest>,
        variable_rows: &[(String, String, bool)],
        setting_timeout_error: Option<&str>,
        setting_max_redirects_error: Option<&str>,
        active_input: Option<&ActiveInputTarget>,
        inputs: &InputRegistry,
        script_sub_tab: ScriptSubTab,
        test_assertions: Option<&[TestAssertion]>,
        spec_buffer: Option<&str>,
        spec_error: Option<&str>,
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
                inputs,
                border,
                muted,
                normal,
                cx,
                on_action,
            ),
            EditorTab::Params => Self::render_params_panel(
                request,
                active_input,
                inputs,
                border,
                muted,
                normal,
                cx,
                on_action,
            ),
            EditorTab::Body => Self::render_body_panel(
                request,
                active_input,
                inputs,
                border,
                muted,
                normal,
                cx,
                on_action,
            ),
            EditorTab::Auth => Self::render_auth_panel(
                request,
                active_input,
                inputs,
                border,
                muted,
                normal,
                cx,
                on_action,
            ),
            EditorTab::Variables => Self::render_variables_panel(
                variable_rows,
                active_input,
                inputs,
                border,
                muted,
                normal,
                cx,
                on_action,
            ),
            EditorTab::Scripts => Self::render_scripts_panel(
                request,
                active_input,
                inputs,
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
                inputs,
                test_assertions,
                border,
                muted,
                normal,
                cx,
                on_action,
            ),
            EditorTab::Settings => Self::render_settings_panel(
                request,
                setting_timeout_error,
                setting_max_redirects_error,
                active_input,
                inputs,
                border,
                muted,
                normal,
                cx,
                on_action,
            ),
            EditorTab::Spec => {
                let default_spec;
                let content = if let Some(buf) = spec_buffer {
                    buf
                } else if let Some(r) = request {
                    default_spec = aura_parser::request_to_spec(r);
                    &default_spec
                } else {
                    ""
                };
                Self::render_spec_panel(
                    content,
                    spec_error,
                    active_input,
                    inputs,
                    border,
                    muted,
                    normal,
                    cx,
                    on_action,
                )
            }
        }
    }

    // =========================================================================
    // Headers Panel (Modal-Free Interactive Key-Value Grid)
    // =========================================================================

}
