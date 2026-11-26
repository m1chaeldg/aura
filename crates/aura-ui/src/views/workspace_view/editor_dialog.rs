use super::AuraWorkspaceView;
use crate::app_state::EditorDialogState;
use crate::input_registry::{InputKey, InputRegistry};
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::dialog::{Dialog, DialogFooter};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{ActiveTheme as _, Sizable};

struct DialogField<'a> {
    label: &'static str,
    input_idx: usize,
    value: &'a str,
    is_focused: bool,
    placeholder: &'static str,
}

impl AuraWorkspaceView {
    pub(super) fn render_editor_dialog(
        dialog: &EditorDialogState,
        inputs: &InputRegistry,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (title, subtitle, width, body_el) = Self::dialog_content(dialog, inputs, cx);
        let view_for_close = cx.entity().clone();

        Dialog::new(cx)
            .w(px(width))
            .title(title)
            .on_close(move |_, _window, app| {
                let _ = view_for_close.update(app, |this, cx| {
                    this.state.cancel_editor_dialog();
                    cx.notify();
                });
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(subtitle),
                    )
                    .child(body_el),
            )
            .footer(
                DialogFooter::new()
                    .child(
                        Button::new("cancel-editor-dialog-btn")
                            .label("Cancel")
                            .small()
                            .rounded(ButtonRounded::Medium)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.cancel_editor_dialog();
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("confirm-editor-dialog-btn")
                            .label("Save")
                            .primary()
                            .small()
                            .rounded(ButtonRounded::Medium)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.confirm_editor_dialog();
                                cx.notify();
                            })),
                    ),
            )
    }

    fn dialog_content(
        dialog: &EditorDialogState,
        inputs: &InputRegistry,
        cx: &App,
    ) -> (&'static str, &'static str, f32, Div) {
        let two_field = |k_lbl, k_val, k_ph, v_lbl, v_val, v_ph, is_val: bool| {
            Self::render_field_list(
                inputs,
                &[
                    DialogField { label: k_lbl, input_idx: 0, value: k_val, is_focused: !is_val, placeholder: k_ph },
                    DialogField { label: v_lbl, input_idx: 1, value: v_val, is_focused: is_val, placeholder: v_ph },
                ],
                true,
                cx,
            )
        };

        match dialog {
            EditorDialogState::AddHeader { key, value, is_value_focused } => (
                "Add Request Header",
                "Enter header key and value. Press Tab to switch fields, Enter to confirm.",
                460.0,
                two_field("Key / Header Name", key, "e.g. Content-Type", "Header Value", value, "e.g. application/json", *is_value_focused),
            ),
            EditorDialogState::EditHeader { key, value, is_value_focused, .. } => (
                "Edit Request Header",
                "Modify header key and value. Press Tab to switch fields, Enter to confirm.",
                460.0,
                two_field("Key / Header Name", key, "Header Name", "Header Value", value, "Header Value", *is_value_focused),
            ),
            EditorDialogState::AddParam { key, value, is_value_focused } => (
                "Add Query Parameter",
                "Enter parameter key and value to append to URL.",
                460.0,
                two_field("Parameter Key", key, "e.g. page", "Parameter Value", value, "e.g. 1", *is_value_focused),
            ),
            EditorDialogState::EditParam { key, value, is_value_focused, .. } => (
                "Edit Query Parameter",
                "Modify parameter key and value.",
                460.0,
                two_field("Parameter Key", key, "Key", "Parameter Value", value, "Value", *is_value_focused),
            ),
            EditorDialogState::AddFormParam { key, value, is_value_focused } => (
                "Add Form Parameter",
                "Enter key and value for URL-encoded form data.",
                460.0,
                two_field("Form Field Key", key, "key", "Form Field Value", value, "value", *is_value_focused),
            ),
            EditorDialogState::EditBasicAuth { username, password, is_password_focused } => (
                "Edit Basic Auth Credentials",
                "Enter username and password for HTTP Basic Authentication.",
                460.0,
                two_field("Username", username, "Username", "Password", password, "Password", *is_password_focused),
            ),
            EditorDialogState::EditBody { content } => (
                "Edit Request Body",
                "Type or paste your JSON, XML, or raw body payload. Enter confirms.",
                520.0,
                Self::render_field_list(
                    inputs,
                    &[DialogField { label: "Body Content", input_idx: 0, value: content, is_focused: true, placeholder: "{\n  \"key\": \"value\"\n}" }],
                    false,
                    cx,
                ),
            ),
            EditorDialogState::EditBearerAuth { token } => (
                "Edit Bearer Token",
                "Enter access token or JWT. Format: Bearer <token>",
                480.0,
                Self::render_field_list(
                    inputs,
                    &[DialogField { label: "Token", input_idx: 0, value: token, is_focused: true, placeholder: "eyJhbGciOi..." }],
                    false,
                    cx,
                ),
            ),
            EditorDialogState::EditApiKeyAuth { key, value, in_header, is_value_focused } => (
                "Edit API Key Authentication",
                "Enter the API key header/query name and secret value.",
                480.0,
                two_field("Key Name", key, "e.g. X-API-Key", "Key Value", value, "secret_key_123", *is_value_focused)
                    .child(Self::render_api_key_placement(*in_header, cx)),
            ),
            EditorDialogState::EditAwsSigV4 { access_key, secret_key, region, service, session_token, active_field } => {
                let af = *active_field;
                (
                    "Edit AWS Signature V4 Auth",
                    "Enter AWS IAM credentials and region. Tab navigates fields.",
                    500.0,
                    Self::render_field_list(
                        inputs,
                        &[
                            DialogField { label: "Access Key", input_idx: 0, value: access_key, is_focused: af == 0, placeholder: "AKIA..." },
                            DialogField { label: "Secret Key", input_idx: 1, value: secret_key, is_focused: af == 1, placeholder: "wJalrXUtnFEMI..." },
                            DialogField { label: "Region", input_idx: 2, value: region, is_focused: af == 2, placeholder: "us-east-1" },
                            DialogField { label: "Service", input_idx: 3, value: service, is_focused: af == 3, placeholder: "execute-api" },
                            DialogField { label: "Session Token (optional)", input_idx: 4, value: session_token, is_focused: af == 4, placeholder: "AQoDYXdz..." },
                        ],
                        false,
                        cx,
                    ),
                )
            }
            EditorDialogState::EditOAuth2 { grant_type, access_token, token_url, client_id, client_secret, active_field } => {
                let af = *active_field;
                (
                    "Edit OAuth 2.0 Auth",
                    "Enter OAuth token and parameters. Tab navigates fields.",
                    500.0,
                    Self::render_field_list(
                        inputs,
                        &[
                            DialogField { label: "Access Token", input_idx: 0, value: access_token, is_focused: af == 0, placeholder: "ya29..." },
                            DialogField { label: "Grant Type", input_idx: 1, value: grant_type, is_focused: af == 1, placeholder: "authorization_code" },
                            DialogField { label: "Token URL (optional)", input_idx: 2, value: token_url, is_focused: af == 2, placeholder: "https://auth.example.com/token" },
                            DialogField { label: "Client ID (optional)", input_idx: 3, value: client_id, is_focused: af == 3, placeholder: "client_123" },
                            DialogField { label: "Client Secret (optional)", input_idx: 4, value: client_secret, is_focused: af == 4, placeholder: "secret_456" },
                        ],
                        false,
                        cx,
                    ),
                )
            }
            EditorDialogState::AddMultipartField { name, value, file_path, is_file, active_field } => {
                let af = *active_field;
                let second = if *is_file {
                    DialogField { label: "File Path", input_idx: 2, value: file_path, is_focused: af == 2, placeholder: "path/to/file.bin" }
                } else {
                    DialogField { label: "Text Value", input_idx: 1, value: value, is_focused: af == 1, placeholder: "value" }
                };
                (
                    "Add Multipart Part",
                    "Enter multipart field name and text value or file path.",
                    480.0,
                    Self::render_field_list(
                        inputs,
                        &[
                            DialogField { label: "Field Name", input_idx: 0, value: name, is_focused: af == 0, placeholder: "name" },
                            second,
                        ],
                        true,
                        cx,
                    ),
                )
            }
        }
    }

    fn render_field_list(inputs: &InputRegistry, fields: &[DialogField<'_>], roomy: bool, cx: &App) -> Div {
        let mut container = if roomy { div().flex().flex_col().gap_3() } else { div().flex().flex_col().gap_2() };
        for f in fields {
            container = container.child(Self::render_dialog_input_row(
                f.label,
                inputs.get_input(&InputKey::EditorDialogField(f.input_idx)),
                f.value,
                f.is_focused,
                f.placeholder,
                cx,
            ));
        }
        container
    }

    fn render_api_key_placement(in_header: bool, cx: &App) -> Div {
        let primary = cx.theme().primary;
        let primary_fg = cx.theme().primary_foreground;
        let secondary = cx.theme().secondary;
        let secondary_fg = cx.theme().secondary_foreground;
        let muted = cx.theme().muted_foreground;
        let radius = cx.theme().radius;

        let chip = |label: &'static str, active: bool| {
            div()
                .px_2()
                .py_0p5()
                .rounded(radius)
                .bg(if active { primary } else { secondary })
                .text_xs()
                .text_color(if active { primary_fg } else { secondary_fg })
                .child(label)
        };
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(div().text_xs().text_color(muted).child("Placement:"))
            .child(chip("In Header", in_header))
            .child(chip("In Query Params", !in_header))
    }

    fn render_dialog_input_row(
        label: &'static str,
        input: Option<&Entity<InputState>>,
        val: &str,
        is_focused: bool,
        placeholder: &'static str,
        cx: &App,
    ) -> Div {
        let field_el = if let Some(state) = input {
            Input::new(state)
                .small()
                .w_full()
                .token(crate::templating_ui::render_template_token)
                .into_any_element()
        } else {
            div()
                .px_2()
                .py_1()
                .bg(cx.theme().background)
                .border_1()
                .border_color(cx.theme().border)
                .rounded(cx.theme().radius)
                .text_xs()
                .text_color(if val.is_empty() {
                    cx.theme().muted_foreground
                } else {
                    cx.theme().foreground
                })
                .child(if val.is_empty() {
                    placeholder.to_string()
                } else {
                    val.to_string()
                })
                .into_any_element()
        };

        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(if is_focused {
                        cx.theme().primary
                    } else {
                        cx.theme().muted_foreground
                    })
                    .child(label),
            )
            .child(field_el)
    }
}
