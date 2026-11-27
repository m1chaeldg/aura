use crate::app_state::AuraAppState;
use crate::input_registry::{InputKey, InputRegistry};
use crate::services::{AiChatRole, AuraAgentAction};
use crate::theme::icon;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::Input;
use gpui_kit::component::resizable::{ResizablePanel, resizable_panel};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};

/// Actions emitted by `AiAssistantView` to the workspace controller.
#[derive(Debug, Clone)]
pub enum AiAssistantAction {
    Close,
    OpenSettings,
    RunDiscovery,
    SelectProvider(String),
    ToggleRedactSecrets,
    ToggleAutoApply,
    ToggleAttachRequest,
    ToggleAttachResponse,
    SendPrompt(String),
    CancelPrompt,
    ClearHistory,
    ApplyAction(AuraAgentAction),
    ApplyAllInMessage(usize),
}

pub struct AiAssistantView;

impl AiAssistantView {
    pub fn render<V: 'static>(
        state: &AuraAppState,
        inputs: &InputRegistry,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, AiAssistantAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> ResizablePanel {
        let fg = cx.theme().foreground;
        let muted = cx.theme().muted_foreground;
        let border = cx.theme().border;
        let radius = cx.theme().radius;
        let card_bg = cx.theme().sidebar;
        let active_provider_id = state.ai_prefs.active_provider_id.clone();

        let prompt_input = inputs.get_input(&InputKey::AiPromptInput);

        resizable_panel().size(px(360.0)).min_size(px(280.0)).child(
            v_flex()
                .size_full()
                .min_h_0()
                .bg(cx.theme().background)
                .border_l_1()
                .border_color(border)
                // Header
                .child(
                    h_flex()
                        .px_3()
                        .py_2()
                        .border_b_1()
                        .border_color(border)
                        .justify_between()
                        .child(
                            h_flex()
                                .gap_1p5()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(fg)
                                        .child("AI Assistant"),
                                )
                                .child(
                                    Tag::primary()
                                        .xsmall()
                                        .child(state.active_ai_provider_label()),
                                ),
                        )
                        .child(
                            h_flex()
                                .gap_1()
                                .child(
                                    Button::new("ai-discover-btn")
                                        .icon(icon(IconName::RefreshCw))
                                        .xsmall()
                                        .ghost()
                                        .rounded(ButtonRounded::Small)
                                        .tooltip("Auto-discover local AI CLIs & endpoints")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(this, AiAssistantAction::RunDiscovery, window, cx);
                                        })),
                                )
                                .child(
                                    Button::new("ai-settings-btn")
                                        .icon(icon(IconName::Settings))
                                        .xsmall()
                                        .ghost()
                                        .rounded(ButtonRounded::Small)
                                        .tooltip("AI & Agent Settings")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(this, AiAssistantAction::OpenSettings, window, cx);
                                        })),
                                )
                                .child(
                                    Button::new("ai-clear-btn")
                                        .label("Clear")
                                        .xsmall()
                                        .ghost()
                                        .rounded(ButtonRounded::Small)
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(this, AiAssistantAction::ClearHistory, window, cx);
                                        })),
                                )
                                .child(
                                    Button::new("ai-close-btn")
                                        .icon(icon(IconName::Close))
                                        .xsmall()
                                        .ghost()
                                        .rounded(ButtonRounded::Small)
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(this, AiAssistantAction::Close, window, cx);
                                        })),
                                ),
                        ),
                )
                // Provider switcher pills
                .child(
                    h_flex()
                        .px_3()
                        .py_1p5()
                        .gap_1()
                        .flex_wrap()
                        .border_b_1()
                        .border_color(border)
                        .children(
                            state
                                .ai_prefs
                                .cli_profiles
                                .iter()
                                .filter(|p| p.enabled)
                                .map(|p| {
                                    let pid = p.id.clone();
                                    let is_sel = active_provider_id.as_deref() == Some(pid.as_str());
                                    let btn = Button::new(SharedString::from(format!("ai-prov-{pid}")))
                                        .label(p.name.clone())
                                        .xsmall()
                                        .rounded(ButtonRounded::Small)
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(
                                                this,
                                                AiAssistantAction::SelectProvider(pid.clone()),
                                                window,
                                                cx,
                                            );
                                        }));
                                    if is_sel { btn.primary() } else { btn.outline() }
                                }),
                        )
                        .children(
                            state
                                .ai_prefs
                                .http_profiles
                                .iter()
                                .filter(|p| p.enabled)
                                .map(|p| {
                                    let pid = p.id.clone();
                                    let is_sel = active_provider_id.as_deref() == Some(pid.as_str());
                                    let btn = Button::new(SharedString::from(format!("ai-prov-{pid}")))
                                        .label(p.name.clone())
                                        .xsmall()
                                        .rounded(ButtonRounded::Small)
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            on_action(
                                                this,
                                                AiAssistantAction::SelectProvider(pid.clone()),
                                                window,
                                                cx,
                                            );
                                        }));
                                    if is_sel { btn.primary() } else { btn.outline() }
                                }),
                        ),
                )
                // Context & Safety toggles
                .child(
                    h_flex()
                        .px_3()
                        .py_1p5()
                        .gap_2()
                        .flex_wrap()
                        .border_b_1()
                        .border_color(border)
                        .child(
                            Checkbox::new("ai-ctx-req")
                                .checked(state.ai_prefs.attach_request_context)
                                .label("Request")
                                .small()
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_action(this, AiAssistantAction::ToggleAttachRequest, window, cx);
                                })),
                        )
                        .child(
                            Checkbox::new("ai-ctx-resp")
                                .checked(state.ai_prefs.attach_response_context)
                                .label("Response")
                                .small()
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_action(this, AiAssistantAction::ToggleAttachResponse, window, cx);
                                })),
                        )
                        .child(
                            Checkbox::new("ai-ctx-redact")
                                .checked(state.ai_prefs.redact_secrets)
                                .label("Redact Secrets")
                                .small()
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_action(this, AiAssistantAction::ToggleRedactSecrets, window, cx);
                                })),
                        )
                        .child(
                            Checkbox::new("ai-ctx-auto-apply")
                                .checked(state.ai_prefs.auto_apply_edits)
                                .label("Auto-Apply")
                                .small()
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_action(this, AiAssistantAction::ToggleAutoApply, window, cx);
                                })),
                        ),
                )
                // Chat history
                .child(
                    div()
                        .id("ai-chat-scroll")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scrollbar()
                        .p_3()
                        .flex()
                        .flex_col()
                        .gap_2p5()
                        .when(state.ai_chat_history.is_empty(), |el| {
                            el.child(
                                v_flex()
                                    .gap_2()
                                    .p_3()
                                    .rounded(radius)
                                    .bg(card_bg)
                                    .border_1()
                                    .border_color(border)
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(fg)
                                            .child("Ask Aura AI to build or modify your request"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(muted)
                                            .child("Generate JSON bodies, pre/post/test scripts, .aura specs, or run Aura commands using your local CLIs or API endpoints."),
                                    ),
                            )
                        })
                        .children(state.ai_chat_history.iter().enumerate().map(|(msg_idx, msg)| {
                            let is_user = msg.role == AiChatRole::User;
                            v_flex()
                                .gap_1p5()
                                .p_2p5()
                                .rounded(radius)
                                .bg(card_bg)
                                .border_1()
                                .border_color(border)
                                .child(
                                    h_flex()
                                        .justify_between()
                                        .child(
                                            Tag::secondary().xsmall().child(if is_user {
                                                "You"
                                            } else {
                                                "Aura AI"
                                            }),
                                        )
                                        .when(msg.actions.len() > 1, |el| {
                                            el.child(
                                                Button::new(ElementId::NamedInteger(
                                                    "ai-apply-all".into(),
                                                    msg_idx as u64,
                                                ))
                                                .label(format!("Apply All ({})", msg.actions.len()))
                                                .xsmall()
                                                .primary()
                                                .rounded(ButtonRounded::Small)
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    on_action(
                                                        this,
                                                        AiAssistantAction::ApplyAllInMessage(msg_idx),
                                                        window,
                                                        cx,
                                                    );
                                                })),
                                            )
                                        }),
                                )
                                .when(!msg.content.trim().is_empty(), |el| {
                                    el.child(
                                        div()
                                            .text_xs()
                                            .text_color(fg)
                                            .child(msg.content.clone()),
                                    )
                                })
                                .children(msg.actions.iter().enumerate().map(|(act_idx, act)| {
                                    let action_clone = act.clone();
                                    let btn_id = format!("ai-apply-{msg_idx}-{act_idx}");
                                    v_flex()
                                        .gap_1()
                                        .p_2()
                                        .rounded(radius)
                                        .border_1()
                                        .border_color(border)
                                        .child(
                                            h_flex()
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .text_color(fg)
                                                        .child(act.title()),
                                                )
                                                .child(
                                                    Button::new(SharedString::from(btn_id))
                                                        .label(match act {
                                                            AuraAgentAction::RunCommand { .. } => "Run",
                                                            _ => "Apply",
                                                        })
                                                        .xsmall()
                                                        .primary()
                                                        .rounded(ButtonRounded::Small)
                                                        .on_click(cx.listener(
                                                            move |this, _, window, cx| {
                                                                on_action(
                                                                    this,
                                                                    AiAssistantAction::ApplyAction(
                                                                        action_clone.clone(),
                                                                    ),
                                                                    window,
                                                                    cx,
                                                                );
                                                            },
                                                        )),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(muted)
                                                .child(act.preview_snippet()),
                                        )
                                }))
                        })),
                )
                // Status + Quick Prompts + Input
                .when_some(state.ai_status_message.clone(), |el, status| {
                    el.child(
                        div()
                            .px_3()
                            .py_1()
                            .border_t_1()
                            .border_color(border)
                            .text_xs()
                            .text_color(muted)
                            .child(status),
                    )
                })
                .child(
                    h_flex()
                        .px_3()
                        .pt_2()
                        .gap_1()
                        .flex_wrap()
                        .border_t_1()
                        .border_color(border)
                        .children(
                            [
                                ("Gen Body", "Generate a realistic JSON request body for this endpoint."),
                                ("Write Tests", "Write Aura test assertions (`aura.test` / `aura.expect`) for this request and response."),
                                ("Pre-Script", "Write a pre-request script to prepare dynamic headers/tokens."),
                                ("Post-Script", "Write a post-response script to extract IDs/tokens into `aura.env`."),
                                ("From Spec", "Update the `.aura` specification and request fields cleanly."),
                            ]
                            .into_iter()
                            .enumerate()
                            .map(|(idx, (label, prompt))| {
                                let p = prompt.to_string();
                                Button::new(ElementId::NamedInteger("ai-quick-chip".into(), idx as u64))
                                    .label(label)
                                    .xsmall()
                                    .outline()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            AiAssistantAction::SendPrompt(p.clone()),
                                            window,
                                            cx,
                                        );
                                    }))
                            }),
                        ),
                )
                .child(
                    h_flex()
                        .p_3()
                        .gap_2()
                        .when_some(prompt_input, |el, input_state| {
                            el.child(
                                div().flex_1().child(
                                    Input::new(&input_state)
                                        .small()
                                        .cleanable(true),
                                ),
                            )
                        })
                        .child(if state.ai_is_busy {
                            Button::new("ai-cancel-btn")
                                .label("Cancel")
                                .small()
                                .danger()
                                .rounded(ButtonRounded::Medium)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_action(this, AiAssistantAction::CancelPrompt, window, cx);
                                }))
                        } else {
                            let draft = state.ai_prompt_draft.clone();
                            Button::new("ai-send-btn")
                                .icon(icon(IconName::Send))
                                .label("Send")
                                .small()
                                .primary()
                                .rounded(ButtonRounded::Medium)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    on_action(
                                        this,
                                        AiAssistantAction::SendPrompt(draft.clone()),
                                        window,
                                        cx,
                                    );
                                }))
                        }),
                ),
        )
    }
}
