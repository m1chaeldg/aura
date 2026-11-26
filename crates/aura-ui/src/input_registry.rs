//! Centralized registry of native GPUI `InputState` and `TextareaState` entities.
//!
//! Keeps `gpui_kit::component::input` controls synchronized with `AuraAppState`
//! without clobbering cursor positions, text selections, or undo history during
//! user editing.

use std::collections::HashMap;

use gpui::{AppContext, Context, Entity, Subscription, Window};
use gpui_kit::component::input::{InputEvent, InputState, TextareaState};

use crate::app_state::ActiveInputTarget;

/// Unique key identifying every text input and textarea control in Aura.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InputKey {
    Url,
    ParamKey(usize),
    ParamValue(usize),
    HeaderKey(usize),
    HeaderValue(usize),
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
}

impl InputKey {
    /// Maps an `InputKey` to its corresponding `ActiveInputTarget`, if any.
    pub fn to_active_target(&self) -> Option<ActiveInputTarget> {
        match self {
            InputKey::Url => Some(ActiveInputTarget::Url),
            InputKey::ParamKey(i) => Some(ActiveInputTarget::ParamKey(*i)),
            InputKey::ParamValue(i) => Some(ActiveInputTarget::ParamValue(*i)),
            InputKey::HeaderKey(i) => Some(ActiveInputTarget::HeaderKey(*i)),
            InputKey::HeaderValue(i) => Some(ActiveInputTarget::HeaderValue(*i)),
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
            | InputKey::ResponseJson => None,
        }
    }
}

/// Manages lifecycle and two-way synchronization of native `InputState` and
/// `TextareaState` entities.
pub struct InputRegistry {
    inputs: HashMap<InputKey, Entity<InputState>>,
    textareas: HashMap<InputKey, Entity<TextareaState>>,
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
            subscriptions: Vec::new(),
        }
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

    pub fn get_input(&self, key: &InputKey) -> Option<&Entity<InputState>> {
        self.inputs.get(key)
    }

    pub fn get_textarea(&self, key: &InputKey) -> Option<&Entity<TextareaState>> {
        self.textareas.get(key)
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
}
