//! Protocol state helpers for GraphQL, WebSockets, SSE, gRPC, and SOAP/WSDL.

use super::AuraAppState;
use aura_core::graphql::GraphQLSchema;
use aura_core::grpc::{parse_proto_file, sample_json_for_message, ProtoSchema};
use aura_core::model::{
    BodyType, GrpcConfig, HeaderItem, HttpMethod, RequestProtocol, SseEvent, StreamMessage,
    WsMessageTemplate,
};
use aura_core::soap::{is_soap_envelope, soap_headers, wrap_soap_envelope, SoapVersion};
use chrono::Utc;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Default)]
pub struct WsdlImportDialogState {
    pub wsdl_text: String,
    pub error: Option<String>,
}

impl AuraAppState {
    pub fn active_protocol(&self) -> RequestProtocol {
        self.active_request()
            .map(|r| r.protocol)
            .unwrap_or(RequestProtocol::Http)
    }

    pub fn toggle_protocol_dropdown(&mut self) {
        self.is_protocol_dropdown_open = !self.is_protocol_dropdown_open;
    }

    pub fn close_protocol_dropdown(&mut self) {
        self.is_protocol_dropdown_open = false;
    }

    pub fn set_active_protocol(&mut self, proto: RequestProtocol) {
        self.is_protocol_dropdown_open = false;
        let proto_tab_id = self.tabs.get(self.active_tab_index).map(|t| t.id.clone());
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.protocol = proto;
            match proto {
                RequestProtocol::Graphql => {
                    tab.request.method = HttpMethod::POST;
                    if !matches!(tab.request.body, BodyType::GraphQL { .. }) {
                        tab.request.body = BodyType::default_for_type("GRAPHQL");
                    }
                    self.active_editor_tab = crate::stores::EditorTab::Body;
                }
                RequestProtocol::Websocket => {
                    if tab.request.url.starts_with("http://") {
                        tab.request.url = tab.request.url.replacen("http://", "ws://", 1);
                    } else if tab.request.url.starts_with("https://") {
                        tab.request.url = tab.request.url.replacen("https://", "wss://", 1);
                    }
                    self.active_editor_tab = crate::stores::EditorTab::Body;
                    self.active_response_tab = crate::stores::ResponseTab::Stream;
                }
                RequestProtocol::Sse => {
                    tab.request.method = HttpMethod::GET;
                    self.active_response_tab = crate::stores::ResponseTab::Stream;
                }
                RequestProtocol::Grpc => {
                    tab.request.method = HttpMethod::POST;
                    if tab.request.grpc.is_none() {
                        tab.request.grpc = Some(GrpcConfig {
                            payload: "{}".to_string(),
                            ..Default::default()
                        });
                    }
                    self.active_editor_tab = crate::stores::EditorTab::Body;
                    self.active_response_tab = crate::stores::ResponseTab::Stream;
                }
                RequestProtocol::Soap => {
                    tab.request.method = HttpMethod::POST;
                    if !matches!(tab.request.body, BodyType::Xml(_)) {
                        let envelope = wrap_soap_envelope(
                            SoapVersion::Soap11,
                            &self.soap_operation,
                            &self.soap_namespace,
                            "",
                            None,
                        );
                        tab.request.body = BodyType::Xml(envelope);
                    }
                    self.active_editor_tab = crate::stores::EditorTab::Body;
                }
                RequestProtocol::Http => {}
            }
            tab.mark_dirty(true);
            tab.sync_spec_from_request();
        }
        if let Some(tid) = proto_tab_id {
            self.load_proto_for_tab_if_needed(&tid);
        }
    }

    // --- GraphQL Helpers ---

    pub fn set_graphql_query(&mut self, query: String) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let vars = match &tab.request.body {
                BodyType::GraphQL { variables, .. } => variables.clone(),
                _ => "{}".to_string(),
            };
            tab.request.body = BodyType::GraphQL {
                query,
                variables: vars,
            };
            tab.mark_dirty(true);
            tab.sync_spec_from_request();
        }
    }

    pub fn set_graphql_variables(&mut self, variables: String) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let query = match &tab.request.body {
                BodyType::GraphQL { query, .. } => query.clone(),
                _ => "query {\n  \n}".to_string(),
            };
            tab.request.body = BodyType::GraphQL { query, variables };
            tab.mark_dirty(true);
            tab.sync_spec_from_request();
        }
    }

    pub fn insert_graphql_field(&mut self, field_snippet: &str) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let (mut query, vars) = match &tab.request.body {
                BodyType::GraphQL { query, variables } => (query.clone(), variables.clone()),
                _ => ("query {\n  \n}".to_string(), "{}".to_string()),
            };
            if let Some(last_brace) = query.rfind('}') {
                let prefix = query[..last_brace].trim_end();
                let mut next = String::from(prefix);
                if !next.ends_with('{') {
                    next.push('\n');
                } else {
                    next.push('\n');
                }
                next.push_str("  ");
                next.push_str(field_snippet);
                next.push('\n');
                next.push_str(&query[last_brace..]);
                query = next;
            } else {
                query.push('\n');
                query.push_str(field_snippet);
            }
            tab.request.body = BodyType::GraphQL {
                query,
                variables: vars,
            };
            tab.mark_dirty(true);
            tab.sync_spec_from_request();
        }
    }

    pub fn active_graphql_schema(&self) -> Option<&GraphQLSchema> {
        let tid = &self.active_tab()?.id;
        self.graphql_schemas.get(tid)
    }

    // --- WebSocket Helpers ---

    pub fn is_active_ws_connected(&self) -> bool {
        self.active_tab()
            .map(|t| self.ws_senders.contains_key(&t.id))
            .unwrap_or(false)
    }

    pub fn register_ws_connection(
        &mut self,
        tab_id: &str,
        tx: mpsc::UnboundedSender<aura_engine::WsCommand>,
    ) {
        self.ws_senders.insert(tab_id.to_string(), tx);
        self.push_stream_message(
            tab_id,
            "system",
            "WebSocket connection established",
        );
    }

    pub fn disconnect_ws_tab(&mut self, tab_id: &str) {
        if let Some(tx) = self.ws_senders.remove(tab_id) {
            let _ = tx.send(aura_engine::WsCommand::Close);
            self.push_stream_message(tab_id, "system", "WebSocket connection closed");
        }
    }

    pub fn send_active_ws_message(&mut self) -> Result<(), String> {
        let Some(tab) = self.tabs.get(self.active_tab_index) else {
            return Err("No active tab".to_string());
        };
        let tab_id = tab.id.clone();
        let ctx = self.active_template_context();
        let rendered = ctx.render(&self.ws_composer_text);
        let Some(tx) = self.ws_senders.get(&tab_id) else {
            return Err("WebSocket is not connected. Click Connect first.".to_string());
        };
        tx.send(aura_engine::WsCommand::SendText(rendered.clone()))
            .map_err(|_| "WebSocket send channel closed".to_string())?;
        self.push_stream_message(&tab_id, "out", rendered);
        self.active_response_tab = crate::stores::ResponseTab::Stream;
        Ok(())
    }

    pub fn save_active_ws_template(&mut self) {
        let payload = self.ws_composer_text.clone();
        if payload.trim().is_empty() {
            return;
        }
        let name = if self.ws_template_name.trim().is_empty() {
            let count = self
                .active_request()
                .map(|r| r.ws_templates.len() + 1)
                .unwrap_or(1);
            format!("Template {count}")
        } else {
            self.ws_template_name.trim().to_string()
        };
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.ws_templates.push(WsMessageTemplate {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                payload,
            });
            tab.mark_dirty(true);
        }
        self.ws_template_name.clear();
    }

    pub fn load_ws_template(&mut self, template_id: &str) {
        if let Some(tpl) = self
            .active_request()
            .and_then(|r| r.ws_templates.iter().find(|t| t.id == template_id))
            .cloned()
        {
            self.ws_composer_text = tpl.payload;
        }
    }

    pub fn delete_ws_template(&mut self, template_id: &str) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.request.ws_templates.retain(|t| t.id != template_id);
            tab.mark_dirty(true);
        }
    }

    pub fn push_stream_message(&mut self, tab_id: &str, dir: &str, payload: impl Into<String>) {
        let msg = StreamMessage {
            direction: dir.to_string(),
            payload: payload.into(),
            timestamp: Utc::now(),
        };
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            let resp = tab.response.get_or_insert_with(|| aura_core::model::AuraResponse {
                status: 101,
                status_text: "Switching Protocols / Stream".to_string(),
                duration_ms: 0,
                size_bytes: 0,
                headers: Default::default(),
                body: String::new(),
                timestamp: Utc::now(),
                assertions: Vec::new(),
                console_logs: Vec::new(),
                script_errors: Vec::new(),
                sse_events: Vec::new(),
                stream_messages: Vec::new(),
            });
            resp.stream_messages.push(msg.clone());
            resp.size_bytes += msg.payload.len();
            if !resp.body.is_empty() {
                resp.body.push('\n');
            }
            resp.body.push_str(&format!("[{}] {}", msg.direction.to_uppercase(), msg.payload));
        }
    }

    pub fn push_sse_event(&mut self, tab_id: &str, ev: SseEvent) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            let resp = tab.response.get_or_insert_with(|| aura_core::model::AuraResponse {
                status: 200,
                status_text: "OK (Event Stream)".to_string(),
                duration_ms: 0,
                size_bytes: 0,
                headers: Default::default(),
                body: String::new(),
                timestamp: Utc::now(),
                assertions: Vec::new(),
                console_logs: Vec::new(),
                script_errors: Vec::new(),
                sse_events: Vec::new(),
                stream_messages: Vec::new(),
            });
            resp.size_bytes += ev.data.len();
            if !resp.body.is_empty() {
                resp.body.push('\n');
            }
            resp.body.push_str(&format!("[{}] {}", ev.event, ev.data));
            resp.sse_events.push(ev);
        }
    }

    pub fn clear_active_stream(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if let Some(ref mut resp) = tab.response {
                resp.stream_messages.clear();
                resp.sse_events.clear();
                resp.body.clear();
                resp.size_bytes = 0;
            }
        }
    }

    // --- gRPC Helpers ---

    pub fn active_proto_schema(&self) -> Option<&ProtoSchema> {
        let tid = &self.active_tab()?.id;
        self.grpc_schemas.get(tid)
    }

    pub fn load_proto_for_tab_if_needed(&mut self, tab_id: &str) {
        if self.grpc_schemas.contains_key(tab_id) {
            return;
        }
        let source = self
            .tabs
            .iter()
            .find(|t| t.id == tab_id)
            .and_then(|t| t.request.grpc.as_ref())
            .and_then(|g| {
                g.proto_content.clone().or_else(|| {
                    g.proto_path
                        .as_deref()
                        .and_then(|p| std::fs::read_to_string(p).ok())
                })
            });
        if let Some(src) = source {
            if let Ok(schema) = parse_proto_file(&src) {
                self.grpc_schemas.insert(tab_id.to_string(), schema);
            }
        }
    }

    pub fn apply_proto_schema(
        &mut self,
        tab_id: &str,
        proto_path: Option<String>,
        proto_content: Option<String>,
        schema: ProtoSchema,
    ) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            let cfg = tab.request.grpc.get_or_insert_with(GrpcConfig::default);
            if proto_path.is_some() {
                cfg.proto_path = proto_path;
            }
            if proto_content.is_some() {
                cfg.proto_content = proto_content;
            }
            if cfg.service.is_empty() {
                if let Some(first_svc) = schema.services.first() {
                    cfg.service = first_svc.full_name.clone();
                    if let Some(first_m) = first_svc.methods.first() {
                        cfg.method = first_m.name.clone();
                        if cfg.payload.trim().is_empty() || cfg.payload.trim() == "{}" {
                            let sample = sample_json_for_message(&schema, &first_m.input_type);
                            cfg.payload =
                                serde_json::to_string_pretty(&sample).unwrap_or_else(|_| "{}".to_string());
                        }
                    }
                }
            }
            tab.mark_dirty(true);
            tab.sync_spec_from_request();
        }
        self.grpc_schemas.insert(tab_id.to_string(), schema);
    }

    pub fn select_grpc_service(&mut self, service_full_name: String) {
        let schema_opt = self.active_proto_schema().cloned();
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let cfg = tab.request.grpc.get_or_insert_with(GrpcConfig::default);
            cfg.service = service_full_name.clone();
            if let Some(schema) = &schema_opt {
                if let Some(svc) = schema.services.iter().find(|s| s.full_name == service_full_name) {
                    if let Some(first_m) = svc.methods.first() {
                        cfg.method = first_m.name.clone();
                        let sample = sample_json_for_message(schema, &first_m.input_type);
                        cfg.payload =
                            serde_json::to_string_pretty(&sample).unwrap_or_else(|_| "{}".to_string());
                    }
                }
            }
            tab.mark_dirty(true);
            tab.sync_spec_from_request();
        }
    }

    pub fn select_grpc_method(&mut self, method_name: String) {
        let schema_opt = self.active_proto_schema().cloned();
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let cfg = tab.request.grpc.get_or_insert_with(GrpcConfig::default);
            cfg.method = method_name.clone();
            if let Some(schema) = &schema_opt {
                if let Some(m) = schema
                    .services
                    .iter()
                    .find(|s| s.full_name == cfg.service)
                    .and_then(|s| s.methods.iter().find(|m| m.name == method_name))
                {
                    let sample = sample_json_for_message(schema, &m.input_type);
                    cfg.payload =
                        serde_json::to_string_pretty(&sample).unwrap_or_else(|_| "{}".to_string());
                }
            }
            tab.mark_dirty(true);
            tab.sync_spec_from_request();
        }
    }

    pub fn generate_grpc_sample_payload(&mut self) {
        let schema_opt = self.active_proto_schema().cloned();
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let cfg = tab.request.grpc.get_or_insert_with(GrpcConfig::default);
            if let Some(schema) = &schema_opt {
                if let Some(m) = schema
                    .services
                    .iter()
                    .find(|s| s.full_name == cfg.service || s.name == cfg.service)
                    .and_then(|s| s.methods.iter().find(|m| m.name == cfg.method))
                {
                    let sample = sample_json_for_message(schema, &m.input_type);
                    cfg.payload =
                        serde_json::to_string_pretty(&sample).unwrap_or_else(|_| "{}".to_string());
                    tab.mark_dirty(true);
                }
            }
        }
    }

    // --- SOAP / WSDL Helpers ---

    pub fn wrap_active_xml_as_soap(&mut self, version: SoapVersion) {
        let op = self.soap_operation.clone();
        let ns = self.soap_namespace.clone();
        let action = self.soap_action.clone();
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let current = match &tab.request.body {
                BodyType::Xml(s) | BodyType::Text(s) | BodyType::Json(s) => s.clone(),
                BodyType::Raw { content, .. } => content.clone(),
                _ => String::new(),
            };
            let inner = if is_soap_envelope(&current) {
                aura_core::soap::extract_soap_body(&current).unwrap_or(current)
            } else {
                current
            };
            let wrapped = wrap_soap_envelope(version, &op, &ns, &inner, None);
            tab.request.protocol = RequestProtocol::Soap;
            tab.request.method = HttpMethod::POST;
            tab.request.body = BodyType::Xml(wrapped);
            for (hk, hv) in soap_headers(
                version,
                if action.trim().is_empty() {
                    None
                } else {
                    Some(action.as_str())
                },
            ) {
                if let Some(existing) = tab
                    .request
                    .headers
                    .iter_mut()
                    .find(|h| h.key.eq_ignore_ascii_case(&hk))
                {
                    existing.value = hv;
                    existing.enabled = true;
                } else {
                    tab.request.headers.push(HeaderItem::new(hk, hv));
                }
            }
            tab.mark_dirty(true);
            tab.sync_spec_from_request();
        }
    }

    pub fn open_wsdl_import_dialog(&mut self) {
        self.wsdl_import_dialog = Some(WsdlImportDialogState::default());
    }

    pub fn close_wsdl_import_dialog(&mut self) {
        self.wsdl_import_dialog = None;
    }

    pub fn import_wsdl_to_workspace(&mut self, wsdl_xml: &str) -> anyhow::Result<usize> {
        let collection = aura_sync::importers::import_wsdl(wsdl_xml)?;
        let count = collection.requests.len();
        if let Some(root) = self.workspace_store.workspace_root.clone() {
            let folder_name = collection
                .name
                .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
            let target_dir = root.join(if folder_name.is_empty() {
                "SOAP Service"
            } else {
                &folder_name
            });
            std::fs::create_dir_all(&target_dir)?;
            for req in &collection.requests {
                let sanitized = req
                    .name
                    .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
                let file_path = target_dir.join(format!("{sanitized}.aura"));
                let content = aura_parser::request_to_spec(req);
                std::fs::write(&file_path, content)?;
            }
            let _ = self.reconcile_workspace_sync();
        }
        if let Some(first_req) = collection.requests.first().cloned() {
            self.collections.push(collection);
            self.open_request(first_req);
        } else {
            self.collections.push(collection);
        }
        self.wsdl_import_dialog = None;
        self.status_message = Some(format!("Imported {count} SOAP operations from WSDL"));
        Ok(count)
    }
}
