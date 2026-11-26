use super::AuraWorkspaceView;
use crate::app_state::ActiveInputTarget;
use crate::input_registry::InputKey;
use crate::stores::WorkspaceTab;
use crate::views::{
    RequestEditorView, RequestHeaderView, ResponsePaneAction, ResponsePaneView, TabBarView,
};
use aura_core::model::HttpMethod;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader};
use gpui_kit::component::resizable::{h_resizable, resizable_panel, v_resizable, ResizablePanel};
use gpui_kit::component::Sizable;

impl AuraWorkspaceView {
    pub(super) fn render_main_pane(
        &self,
        active_tab: Option<&WorkspaceTab>,
        active_title: &str,
        active_method: HttpMethod,
        is_horizontal: bool,
        is_in_flight: bool,
        cx: &mut Context<Self>,
    ) -> ResizablePanel {
        resizable_panel().child(
            div()
                .flex()
                .flex_col()
                .size_full()
                .child(TabBarView::render(
                    &self.state,
                    cx,
                    |this, idx, _window, cx| {
                        this.state.select_tab(idx);
                        cx.notify();
                    },
                    |this, tab_id, _window, cx| {
                        this.state.close_tab(tab_id);
                        cx.notify();
                    },
                    |this, _window, cx| {
                        this.spawn_create_new_request(cx);
                    },
                ))
                .when(active_tab.is_none(), |el| {
                    el.child(Self::render_empty_tabs_state(cx))
                })
                .when(active_tab.is_some(), |el| {
                    el.child(self.render_active_header(active_method, is_in_flight, cx))
                        .child(self.render_editor_response_split(
                            active_tab,
                            active_title,
                            active_method,
                            is_horizontal,
                            cx,
                        ))
                }),
        )
    }

    fn render_empty_tabs_state(cx: &mut Context<Self>) -> impl IntoElement {
        use gpui_kit::component::ActiveTheme as _;
        div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .bg(cx.theme().background)
            .child(
                Empty::new().header(
                    EmptyHeader::new()
                        .description(EmptyDescription::new().child("No open requests")),
                ),
            )
            .child(
                Button::new("empty-new-request-btn")
                    .icon(crate::theme::icon(gpui_kit::assets::IconName::Plus))
                    .label("New Request")
                    .small()
                    .primary()
                    .rounded(ButtonRounded::Medium)
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.spawn_create_new_request(cx);
                    })),
            )
    }

    fn render_active_header(
        &self,
        active_method: HttpMethod,
        is_in_flight: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active_proto = self.state.active_protocol();
        let is_ws_connected = self.state.is_active_ws_connected();
        RequestHeaderView::render(
            active_proto,
            self.state.is_protocol_dropdown_open,
            is_ws_connected,
            active_method,
            self.inputs
                .get_input(&InputKey::Url)
                .expect("URL input initialized in sync_inputs"),
            is_in_flight,
            self.state.active_input_target == Some(ActiveInputTarget::Url),
            self.state.is_active_tab_dirty(),
            self.state.is_method_dropdown_open,
            cx,
            |this, _window, cx| {
                this.state.toggle_protocol_dropdown();
                cx.notify();
            },
            |this, proto, _window, cx| {
                this.state.set_active_protocol(proto);
                this.schedule_auto_save(cx);
                cx.notify();
            },
            |this, _window, cx| {
                this.state.close_protocol_dropdown();
                cx.notify();
            },
            |this, _window, cx| {
                this.state.toggle_method_dropdown();
                cx.notify();
            },
            |this, method, _window, cx| {
                this.state.set_active_method(method);
                this.schedule_auto_save(cx);
                cx.notify();
            },
            |this, _window, cx| {
                this.state.close_method_dropdown();
                cx.notify();
            },
            |this, _window, cx| {
                this.state.set_active_input(Some(ActiveInputTarget::Url));
                cx.notify();
            },
            |this, _window, cx| {
                this.dispatch_send_request(cx);
            },
            |this, _window, cx| {
                if this.state.active_protocol() == aura_core::model::RequestProtocol::Websocket {
                    if let Some(tid) = this.state.active_tab().map(|t| t.id.clone()) {
                        this.state.disconnect_ws_tab(&tid);
                        this.state.status_message = Some("WebSocket disconnected".to_string());
                    }
                } else {
                    tracing::info!(
                        target: "aura_ui::execution",
                        "User clicked Cancel button; aborting active request"
                    );
                    this.state.execution_service.cancel();
                    this.state.status_message = Some("Request cancelled by user".to_string());
                }
                cx.notify();
            },
            |this, _window, cx| {
                this.dispatch_save_active_tab(false, cx);
            },
        )
    }

    fn render_editor_response_split(
        &self,
        active_tab: Option<&WorkspaceTab>,
        active_title: &str,
        active_method: HttpMethod,
        is_horizontal: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let editor_panel = resizable_panel()
            .size_range(px(180.)..Pixels::MAX)
            .child(RequestEditorView::render(
                self.state.split_ratio,
                is_horizontal,
                active_title,
                active_method,
                self.state.active_editor_tab,
                active_tab.map(|t| &t.request),
                active_tab
                    .map(|t| t.variable_rows.as_slice())
                    .unwrap_or(&[]),
                self.state.setting_timeout_error.as_deref(),
                self.state.setting_max_redirects_error.as_deref(),
                self.state.active_input_target.as_ref(),
                &self.inputs,
                self.state.active_script_sub_tab,
                self.state.show_script_reference,
                active_tab
                    .and_then(|t| t.response.as_ref())
                    .map(|r| r.assertions.as_slice()),
                active_tab.and_then(|t| t.spec_buffer.as_deref()),
                active_tab.and_then(|t| t.spec_error.as_deref()),
                self.state.is_body_format_dropdown_open,
                self.state.active_graphql_schema(),
                self.state.is_graphql_introspecting,
                self.state.show_graphql_schema,
                self.state.active_proto_schema(),
                self.state.is_active_ws_connected(),
                &self.state.ws_composer_text,
                cx,
                |this, action, window, cx| {
                    this.handle_request_editor_action(action, window, cx);
                },
            ));
        let response_panel = resizable_panel()
            .size_range(px(180.)..Pixels::MAX)
            .child(ResponsePaneView::render(
                self.state.split_ratio,
                is_horizontal,
                active_tab.and_then(|t| t.response.as_ref()),
                self.state.active_response_tab,
                self.state.active_body_mode,
                self.state.response_language_override,
                self.state.is_response_lang_dropdown_open,
                self.state.response_soft_wrap,
                &self.state.response_tree_collapsed,
                &self.state.response_table_search,
                self.state.response_table_sort_col,
                self.state.response_table_sort_asc,
                self.state.response_bi_chart_type,
                self.state.response_bi_agg_op,
                self.state.response_bi_x_col.as_deref(),
                self.state.response_bi_y_col.as_deref(),
                self.state.response_bi_pivot_col.as_deref(),
                self.inputs.get_editor(&InputKey::ResponseBody),
                self.inputs.get_editor(&InputKey::ResponseRaw),
                self.inputs.get_editor(&InputKey::ResponseTests),
                self.inputs.get_editor(&InputKey::ResponseConsole),
                self.inputs.get_input(&InputKey::ResponseTableSearch),
                cx,
                |this, action, _window, cx| {
                    match action {
                        ResponsePaneAction::SelectTab(tab) => this.state.set_response_tab(tab),
                        ResponsePaneAction::SetBodyMode(mode) => {
                            this.state.set_response_body_mode(mode)
                        }
                        ResponsePaneAction::ToggleBodyMode => {
                            this.state.toggle_response_body_mode()
                        }
                        ResponsePaneAction::CycleLanguage(current) => {
                            this.state.cycle_response_language(current)
                        }
                        ResponsePaneAction::ToggleLanguageDropdown => {
                            this.state.toggle_response_lang_dropdown()
                        }
                        ResponsePaneAction::DismissLanguageDropdown => {
                            this.state.close_response_lang_dropdown()
                        }
                        ResponsePaneAction::SetLanguage(lang) => {
                            this.state.set_response_language(lang)
                        }
                        ResponsePaneAction::ToggleSoftWrap => {
                            this.state.toggle_response_soft_wrap()
                        }
                        ResponsePaneAction::CopyBody => {
                            if let Some(resp) = this
                                .state
                                .active_tab()
                                .and_then(|t| t.response.as_ref())
                            {
                                let content_type = resp
                                    .headers
                                    .iter()
                                    .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                                    .map(|(_, v)| v.as_str());
                                let detected = crate::services::FormatService::detect_language(
                                    &resp.body,
                                    content_type,
                                );
                                let lang =
                                    this.state.response_language_override.unwrap_or(detected);
                                let formatted =
                                    crate::services::FormatService::format_response_body(
                                        &resp.body,
                                        lang,
                                        this.state.active_body_mode,
                                    );
                                cx.write_to_clipboard(ClipboardItem::new_string(formatted));
                                this.state.status_message =
                                    Some("Response body copied to clipboard".to_string());
                            }
                        }
                        ResponsePaneAction::CopyText { label, text } => {
                            cx.write_to_clipboard(ClipboardItem::new_string(text));
                            this.state.status_message =
                                Some(format!("{label} copied to clipboard"));
                        }
                        ResponsePaneAction::ToggleTreeNode(id) => {
                            this.state.toggle_response_tree_node(&id);
                        }
                        ResponsePaneAction::ExpandAllTree => {
                            this.state.expand_all_response_tree();
                        }
                        ResponsePaneAction::ToggleTableSort(col) => {
                            this.state.toggle_response_table_sort(col);
                        }
                        ResponsePaneAction::SetBiChartType(ct) => {
                            this.state.set_response_bi_chart_type(ct);
                        }
                        ResponsePaneAction::CycleBiAggOp => {
                            this.state.cycle_response_bi_agg_op();
                        }
                        ResponsePaneAction::CycleBiXCol(cols) => {
                            this.state.cycle_response_bi_x_col(&cols);
                        }
                        ResponsePaneAction::CycleBiYCol(num_cols) => {
                            this.state.cycle_response_bi_y_col(&num_cols);
                        }
                        ResponsePaneAction::CycleBiPivotCol(cols) => {
                            this.state.cycle_response_bi_pivot_col(&cols);
                        }
                        ResponsePaneAction::ClearStream => {
                            this.state.clear_active_stream();
                        }
                    }
                    cx.notify();
                },
            ));

        if is_horizontal {
            h_resizable("editor-response-split")
                .child(editor_panel)
                .child(response_panel)
        } else {
            v_resizable("editor-response-split")
                .child(editor_panel)
                .child(response_panel)
        }
    }
}
