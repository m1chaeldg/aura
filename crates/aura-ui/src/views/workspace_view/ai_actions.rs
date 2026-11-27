use super::AuraWorkspaceView;
use crate::preferences::{AiCliProfile, AiHttpKind, AiHttpProfile, AuraPreferences};
use crate::services::{
    AiChatMessage, AiChatRole, AiService, AuraAgentAction, discover_all,
    merge_discovery_into_preferences,
};
use crate::views::ai_assistant_view::{AiAssistantAction, AiAssistantView};
use gpui::*;
use gpui_kit::component::resizable::ResizablePanel;
use tokio_util::sync::CancellationToken;

impl AuraWorkspaceView {
    pub(super) fn render_ai_drawer_panel(&self, cx: &mut Context<Self>) -> ResizablePanel {
        AiAssistantView::render(&self.state, &self.inputs, cx, |this, action, _window, cx| {
            this.handle_ai_assistant_action(action, cx);
        })
    }

    pub fn handle_ai_assistant_action(
        &mut self,
        action: AiAssistantAction,
        cx: &mut Context<Self>,
    ) {
        match action {
            AiAssistantAction::Close => {
                self.state.is_ai_drawer_open = false;
            }
            AiAssistantAction::OpenSettings => {
                self.state.settings_tab_ai = true;
                self.state.is_settings_modal_open = true;
            }
            AiAssistantAction::RunDiscovery => {
                self.spawn_ai_discovery(cx);
            }
            AiAssistantAction::SelectProvider(id) => {
                self.state.ai_prefs.active_provider_id = Some(id);
                self.save_ai_preferences_sync();
            }
            AiAssistantAction::ToggleRedactSecrets => {
                self.state.ai_prefs.redact_secrets = !self.state.ai_prefs.redact_secrets;
                self.save_ai_preferences_sync();
            }
            AiAssistantAction::ToggleAutoApply => {
                self.state.ai_prefs.auto_apply_edits = !self.state.ai_prefs.auto_apply_edits;
                self.save_ai_preferences_sync();
            }
            AiAssistantAction::ToggleAttachRequest => {
                self.state.ai_prefs.attach_request_context =
                    !self.state.ai_prefs.attach_request_context;
                self.save_ai_preferences_sync();
            }
            AiAssistantAction::ToggleAttachResponse => {
                self.state.ai_prefs.attach_response_context =
                    !self.state.ai_prefs.attach_response_context;
                self.save_ai_preferences_sync();
            }
            AiAssistantAction::SendPrompt(prompt) => {
                let trimmed = prompt.trim().to_string();
                if !trimmed.is_empty() {
                    self.spawn_ai_send_prompt(trimmed, cx);
                }
            }
            AiAssistantAction::CancelPrompt => {
                if let Some(token) = self.state.ai_cancel_token.take() {
                    token.cancel();
                }
                self.state.ai_is_busy = false;
                self.state.ai_status_message = Some("AI request cancelled.".to_string());
            }
            AiAssistantAction::ClearHistory => {
                self.state.ai_chat_history.clear();
                self.state.ai_status_message = None;
            }
            AiAssistantAction::ApplyAction(agent_action) => {
                self.apply_ai_agent_action(&agent_action, cx);
            }
            AiAssistantAction::ApplyAllInMessage(msg_idx) => {
                if let Some(actions) = self
                    .state
                    .ai_chat_history
                    .get(msg_idx)
                    .map(|m| m.actions.clone())
                {
                    for act in &actions {
                        self.apply_ai_agent_action(act, cx);
                    }
                }
            }
        }
        cx.notify();
    }

    pub fn save_ai_preferences_sync(&self) {
        let mut prefs = AuraPreferences::load();
        prefs.ai = self.state.ai_prefs.clone();
        let _ = prefs.save();
    }

    pub fn spawn_ai_discovery(&mut self, cx: &mut Context<Self>) {
        if self.state.ai_discovery_in_flight {
            return;
        }
        self.state.ai_discovery_in_flight = true;
        self.state.ai_status_message = Some("Scanning PATH & local endpoints for AI CLIs...".to_string());
        cx.notify();

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let discovery = discover_all().await;
                let _ = this.update(&mut cx, |this, cx| {
                    this.state.ai_discovery_in_flight = false;
                    let cli_count = discovery.cli_profiles.len();
                    let http_count = discovery.local_http_profiles.len();
                    merge_discovery_into_preferences(&mut this.state.ai_prefs, &discovery);
                    this.save_ai_preferences_sync();
                    this.state.ai_status_message = Some(format!(
                        "Discovered {cli_count} CLI(s) and {http_count} local runtime(s)."
                    ));
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub fn spawn_ai_send_prompt(&mut self, prompt: String, cx: &mut Context<Self>) {
        if self.state.ai_is_busy {
            return;
        }
        let cancel = CancellationToken::new();
        self.state.ai_cancel_token = Some(cancel.clone());
        self.state.ai_is_busy = true;
        self.state.ai_status_message = Some(format!("Running {}...", self.state.active_ai_provider_label()));
        self.state.ai_prompt_draft.clear();
        self.state.ai_chat_history.push(AiChatMessage {
            role: AiChatRole::User,
            content: prompt.clone(),
            actions: Vec::new(),
        });

        let prefs = self.state.ai_prefs.clone();
        let history = self.state.ai_chat_history.clone();
        let context = self.state.build_ai_context_snapshot();
        cx.notify();

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let res = AiService::send_prompt(&prefs, &history, &prompt, &context, cancel).await;
                let _ = this.update(&mut cx, |this, cx| {
                    this.state.ai_is_busy = false;
                    this.state.ai_cancel_token = None;
                    match res {
                        Ok(ai_resp) => {
                            let auto_apply = this.state.ai_prefs.auto_apply_edits;
                            let actions = ai_resp.actions.clone();
                            this.state.ai_chat_history.push(AiChatMessage {
                                role: AiChatRole::Assistant,
                                content: ai_resp.display_text,
                                actions: ai_resp.actions,
                            });
                            this.state.ai_status_message = if actions.is_empty() {
                                None
                            } else {
                                Some(format!("{} proposed action(s) ready.", actions.len()))
                            };
                            if auto_apply {
                                for act in &actions {
                                    if !matches!(act, AuraAgentAction::RunCommand { .. }) {
                                        this.apply_ai_agent_action(act, cx);
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            this.state.ai_status_message = Some(format!("AI error: {err}"));
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub fn apply_ai_agent_action(&mut self, action: &AuraAgentAction, cx: &mut Context<Self>) {
        match self.state.apply_agent_action(action) {
            Ok(Some(cmd_id)) => {
                if cmd_id == "cmd:send_request" {
                    self.dispatch_send_request(cx);
                } else {
                    self.execute_app_command(&cmd_id, cx);
                }
                self.state.ai_status_message = Some(format!("Executed {cmd_id}"));
            }
            Ok(None) => {
                self.schedule_auto_save(cx);
                self.state.ai_status_message = Some(format!("Applied: {}", action.title()));
            }
            Err(err) => {
                self.state.ai_status_message = Some(format!("Apply failed: {err}"));
            }
        }
    }

    pub fn add_custom_ai_cli_profile(&mut self) {
        let name = self.state.ai_custom_cli_name.trim().to_string();
        let command = self.state.ai_custom_cli_cmd.trim().to_string();
        if name.is_empty() || command.is_empty() {
            self.state.ai_status_message = Some("Provide both a CLI name and command.".to_string());
            return;
        }
        let args: Vec<String> = self
            .state
            .ai_custom_cli_args
            .split_whitespace()
            .map(ToString::to_string)
            .collect();
        let id = format!("cli:custom:{}", uuid::Uuid::new_v4());
        self.state.ai_prefs.cli_profiles.push(AiCliProfile {
            id: id.clone(),
            name,
            command,
            args,
            enabled: true,
            auto_discovered: false,
            timeout_secs: 120,
        });
        self.state.ai_prefs.active_provider_id = Some(id);
        self.state.ai_custom_cli_name.clear();
        self.state.ai_custom_cli_cmd.clear();
        self.save_ai_preferences_sync();
    }

    pub fn add_custom_ai_http_profile(&mut self, kind: AiHttpKind) {
        let name = self.state.ai_http_name.trim().to_string();
        let base_url = self.state.ai_http_url.trim().to_string();
        let model = self.state.ai_http_model.trim().to_string();
        if name.is_empty() || base_url.is_empty() || model.is_empty() {
            self.state.ai_status_message =
                Some("Provide endpoint name, base URL, and model.".to_string());
            return;
        }
        let id = format!("http:custom:{}", uuid::Uuid::new_v4());
        self.state.ai_prefs.http_profiles.push(AiHttpProfile {
            id: id.clone(),
            name,
            kind,
            base_url,
            model,
            api_key: self.state.ai_http_api_key.trim().to_string(),
            enabled: true,
            auto_discovered: false,
        });
        self.state.ai_prefs.active_provider_id = Some(id);
        self.state.ai_http_name.clear();
        self.save_ai_preferences_sync();
    }
}
