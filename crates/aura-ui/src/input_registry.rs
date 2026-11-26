//! Centralized registry of native GPUI `InputState` and `TextareaState` entities.
//!
//! Keeps `gpui_kit::component::input` controls synchronized with `AuraAppState`
//! without clobbering cursor positions, text selections, or undo history during
//! user editing.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gpui::{AppContext, Context, Entity, Subscription, Window};
use gpui_kit::component::input::{EditorState, InputEvent, InputState, TextareaState};

use crate::app_state::ActiveInputTarget;
use crate::editor_support::{AuraCompletionProvider, CompletionLanguage, EditorContextData};

/// Unique key identifying every text input and textarea control in Aura.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InputKey {
    Url,
    ParamKey(usize),
    ParamValue(usize),
    HeaderKey(usize),
    HeaderValue(usize),
    VariableKey(usize),
    VariableValue(usize),
    SettingTimeout,
    SettingMaxRedirects,
    FormParamKey(usize),
    FormParamValue(usize),
    AuthBearerToken,
    AuthBasicUsername,
    AuthBasicPassword,
    AuthApiKeyName,
    AuthApiKeyValue,
    AuthAwsField(usize),
    AuthOAuth2Field(usize),
    BodyText,
    PreRequestScript,
    PostResponseScript,
    TestScript,
    SpecText,
    CommandPaletteSearch,
    RenameDialog,
    MoveToSearch,
    InlineRename,
    EditorDialogField(usize),
    ResponseBody,
    ResponseRaw,
    ResponseJson,
    LogViewerText,
}

impl InputKey {
    /// Returns the syntax-highlighting language configured for code-editor
    /// controls, or `None` for plain inputs and textareas.
    pub fn editor_language(&self) -> Option<&'static str> {
        match self {
            InputKey::PreRequestScript
            | InputKey::PostResponseScript
            | InputKey::TestScript => Some("javascript"),
            InputKey::SpecText => Some("toml"),
            _ => None,
        }
    }

    /// Maps an `InputKey` to its corresponding `ActiveInputTarget`, if any.
    pub fn to_active_target(&self) -> Option<ActiveInputTarget> {
        match self {
            InputKey::Url => Some(ActiveInputTarget::Url),
            InputKey::ParamKey(i) => Some(ActiveInputTarget::ParamKey(*i)),
            InputKey::ParamValue(i) => Some(ActiveInputTarget::ParamValue(*i)),
            InputKey::HeaderKey(i) => Some(ActiveInputTarget::HeaderKey(*i)),
            InputKey::HeaderValue(i) => Some(ActiveInputTarget::HeaderValue(*i)),
            InputKey::VariableKey(i) => Some(ActiveInputTarget::VariableKey(*i)),
            InputKey::VariableValue(i) => Some(ActiveInputTarget::VariableValue(*i)),
            InputKey::SettingTimeout => Some(ActiveInputTarget::SettingTimeout),
            InputKey::SettingMaxRedirects => Some(ActiveInputTarget::SettingMaxRedirects),
            InputKey::FormParamKey(i) => Some(ActiveInputTarget::FormParamKey(*i)),
            InputKey::FormParamValue(i) => Some(ActiveInputTarget::FormParamValue(*i)),
            InputKey::AuthBearerToken => Some(ActiveInputTarget::AuthBearerToken),
            InputKey::AuthBasicUsername => Some(ActiveInputTarget::AuthBasicUsername),
            InputKey::AuthBasicPassword => Some(ActiveInputTarget::AuthBasicPassword),
            InputKey::AuthApiKeyName => Some(ActiveInputTarget::AuthApiKeyName),
            InputKey::AuthApiKeyValue => Some(ActiveInputTarget::AuthApiKeyValue),
            InputKey::AuthAwsField(i) => Some(ActiveInputTarget::AuthAwsField(*i)),
            InputKey::AuthOAuth2Field(i) => Some(ActiveInputTarget::AuthOAuth2Field(*i)),
            InputKey::BodyText => Some(ActiveInputTarget::BodyText),
            InputKey::PreRequestScript => Some(ActiveInputTarget::PreRequestScript),
            InputKey::PostResponseScript => Some(ActiveInputTarget::PostResponseScript),
            InputKey::TestScript => Some(ActiveInputTarget::TestScript),
            InputKey::SpecText => Some(ActiveInputTarget::SpecText),
            InputKey::CommandPaletteSearch
            | InputKey::RenameDialog
            | InputKey::MoveToSearch
            | InputKey::InlineRename
            | InputKey::EditorDialogField(_)
            | InputKey::ResponseBody
            | InputKey::ResponseRaw
            | InputKey::ResponseJson
            | InputKey::LogViewerText => None,
        }
    }
}

/// Manages lifecycle and two-way synchronization of native `InputState`,
/// `TextareaState`, and `EditorState` entities.
pub struct InputRegistry {
    inputs: HashMap<InputKey, Entity<InputState>>,
    textareas: HashMap<InputKey, Entity<TextareaState>>,
    editors: HashMap<InputKey, Entity<EditorState>>,
    editor_context: Rc<RefCell<EditorContextData>>,
    subscriptions: Vec<Subscription>,
}

impl Default for InputRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl InputRegistry {
    pub fn new() -> Self {
        Self {
            inputs: HashMap::new(),
            textareas: HashMap::new(),
            editors: HashMap::new(),
            editor_context: Rc::new(RefCell::new(EditorContextData::default())),
            subscriptions: Vec::new(),
        }
    }

    /// Updates the live request variables, query parameters, and headers used by
    /// editor autocomplete providers.
    pub fn set_editor_context(&self, data: EditorContextData) {
        *self.editor_context.borrow_mut() = data;
    }

    /// Ensures a single-line `InputState` exists for `key` and matches `value`.
    pub fn ensure_input<V: 'static>(
        &mut self,
        key: InputKey,
        value: &str,
        placeholder: &str,
        masked: bool,
        window: &mut Window,
        cx: &mut Context<V>,
        on_event: impl Fn(&mut V, &InputKey, &Entity<InputState>, &InputEvent, &mut Context<V>)
            + 'static,
    ) -> Entity<InputState> {
        if let Some(entity) = self.inputs.get(&key) {
            let entity = entity.clone();
            let current = entity.read(cx).value();
            if current.as_ref() != value {
                let val_owned = value.to_string();
                entity.update(cx, |state, cx| {
                    state.set_value(val_owned, window, cx);
                });
            }
            return entity;
        }

        let placeholder_owned = placeholder.to_string();
        let value_owned = value.to_string();
        let entity = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(placeholder_owned)
                .masked(masked)
                .default_value(value_owned)
        });

        let key_clone = key.clone();
        let sub = cx.subscribe(&entity, move |this, state, event, cx| {
            on_event(this, &key_clone, &state, event, cx);
        });
        self.subscriptions.push(sub);
        self.inputs.insert(key, entity.clone());
        entity
    }

    /// Ensures a multi-line `TextareaState` exists for `key` and matches `value`.
    pub fn ensure_textarea<V: 'static>(
        &mut self,
        key: InputKey,
        value: &str,
        placeholder: &str,
        window: &mut Window,
        cx: &mut Context<V>,
        on_event: impl Fn(&mut V, &InputKey, &Entity<TextareaState>, &InputEvent, &mut Context<V>)
            + 'static,
    ) -> Entity<TextareaState> {
        if let Some(entity) = self.textareas.get(&key) {
            let entity = entity.clone();
            let current = entity.read(cx).value();
            if current.as_ref() != value {
                let val_owned = value.to_string();
                entity.update(cx, |state, cx| {
                    state.set_value(val_owned, window, cx);
                });
            }
            return entity;
        }

        let placeholder_owned = placeholder.to_string();
        let value_owned = value.to_string();
        let entity = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder(placeholder_owned)
                .default_value(value_owned)
        });

        let key_clone = key.clone();
        let sub = cx.subscribe(&entity, move |this, state, event, cx| {
            on_event(this, &key_clone, &state, event, cx);
        });
        self.subscriptions.push(sub);
        self.textareas.insert(key, entity.clone());
        entity
    }

    /// Ensures a code `EditorState` exists for `key` with `language` and matches `value`.
    #[allow(clippy::too_many_arguments)]
    pub fn ensure_editor<V: 'static>(
        &mut self,
        key: InputKey,
        value: &str,
        placeholder: &str,
        language: &str,
        window: &mut Window,
        cx: &mut Context<V>,
        on_event: impl Fn(&mut V, &InputKey, &Entity<EditorState>, &InputEvent, &mut Context<V>)
            + 'static,
    ) -> Entity<EditorState> {
        if let Some(entity) = self.editors.get(&key) {
            let entity = entity.clone();
            let current = entity.read(cx).value();
            if current.as_ref() != value {
                let val_owned = value.to_string();
                entity.update(cx, |state, cx| {
                    state.set_value(val_owned, window, cx);
                });
            }
            return entity;
        }

        let placeholder_owned = placeholder.to_string();
        let value_owned = value.to_string();
        let language_owned = language.to_string();
        let completion_lang = match &key {
            InputKey::PreRequestScript => Some(CompletionLanguage::JavaScript {
                include_response: false,
            }),
            InputKey::PostResponseScript | InputKey::TestScript => {
                Some(CompletionLanguage::JavaScript {
                    include_response: true,
                })
            }
            InputKey::SpecText => Some(CompletionLanguage::TomlSpec),
            _ => None,
        };
        let editor_context = Rc::clone(&self.editor_context);
        let entity = cx.new(|cx| {
            let mut state = EditorState::new(window, cx)
                .language(language_owned)
                .line_number(true)
                .soft_wrap(true)
                .placeholder(placeholder_owned)
                .default_value(value_owned);
            if let Some(lang) = completion_lang {
                state.lsp_mut().completion_provider =
                    Some(Rc::new(AuraCompletionProvider::new(lang, editor_context)));
            }
            state
        });

        let key_clone = key.clone();
        let sub = cx.subscribe(&entity, move |this, state, event, cx| {
            on_event(this, &key_clone, &state, event, cx);
        });
        self.subscriptions.push(sub);
        self.editors.insert(key, entity.clone());
        entity
    }

    pub fn get_input(&self, key: &InputKey) -> Option<&Entity<InputState>> {
        self.inputs.get(key)
    }

    pub fn get_textarea(&self, key: &InputKey) -> Option<&Entity<TextareaState>> {
        self.textareas.get(key)
    }

    pub fn get_editor(&self, key: &InputKey) -> Option<&Entity<EditorState>> {
        self.editors.get(key)
    }

    /// Focuses an existing single-line input if present.
    pub fn focus_input<V: 'static>(
        &self,
        key: &InputKey,
        window: &mut Window,
        cx: &mut Context<V>,
    ) {
        if let Some(entity) = self.inputs.get(key) {
            entity.update(cx, |state, cx| {
                state.focus(window, cx);
            });
        }
    }

    /// Focuses an existing multi-line textarea if present.
    pub fn focus_textarea<V: 'static>(
        &self,
        key: &InputKey,
        window: &mut Window,
        cx: &mut Context<V>,
    ) {
        if let Some(entity) = self.textareas.get(key) {
            entity.update(cx, |state, cx| {
                state.focus(window, cx);
            });
        }
    }

    /// Focuses an existing code editor if present.
    pub fn focus_editor<V: 'static>(
        &self,
        key: &InputKey,
        window: &mut Window,
        cx: &mut Context<V>,
    ) {
        if let Some(entity) = self.editors.get(key) {
            entity.update(cx, |state, cx| {
                state.focus(window, cx);
            });
        }
    }
}
