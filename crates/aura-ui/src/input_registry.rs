//! Centralized registry of native GPUI `InputState` and `TextareaState` entities.
//!
//! Keeps `gpui_kit::component::input` controls synchronized with `AuraAppState`
//! without clobbering cursor positions, text selections, or undo history during
//! user editing.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use aura_core::templating::TemplateContext;
use gpui::{App, AppContext, Context, Entity, Subscription, Window};
use gpui_kit::component::input::{
    EditorState, InputEvent, InputState, TextDecorationCollection, TextareaState,
};

use crate::app_state::ActiveInputTarget;
use crate::editor_support::{AuraCompletionProvider, CompletionLanguage, EditorContextData};
use crate::templating_ui::{
    TemplateHoverProvider, sync_editor_template_decorations, sync_input_template_tokens,
};

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
    AuthSchemeField(usize),
    SettingCertField(usize),
    SettingProxyField(usize),
    BodyText,
    PreRequestScript,
    PostResponseScript,
    TestScript,
    AssertExpr(usize),
    AssertExpected(usize),
    ExtractorVar(usize),
    ExtractorExpr(usize),
    SpecText,
    CommandPaletteSearch,
    RenameDialog,
    MoveToSearch,
    InlineRename,
    EditorDialogField(usize),
    ResponseBody,
    ResponseRaw,
    ResponseJson,
    ResponseTests,
    ResponseConsole,
    ResponseTableSearch,
    LogViewerText,
    EnvName,
    EnvVarKey(usize),
    EnvVarValue(usize),
    EnvCode,
    GraphQLQuery,
    GraphQLVariables,
    WsMessageInput,
    WsTemplateName,
    GrpcService,
    GrpcMethod,
    GrpcPayload,
    SoapOperation,
    SoapNamespace,
    SoapAction,
    WsdlImportText,
    VaultAddr,
    VaultToken,
    VaultNamespace,
    VaultProfileName,
    VaultAwsRegion,
    VaultAwsProfile,
    AwsProfileName,
    KeePassProfileName,
    KeePassDbPath,
    KeePassKeyfilePath,
    KeePassPassword,
    KeychainService,
    KeychainKey,
    KeychainValue,
    DomainCertDomain,
    DomainCertPath,
    DomainCertKeyPath,
    DomainCertPassphrase,
    GlobalProxyHost,
    GlobalProxyPort,
    GlobalProxyUsername,
    GlobalProxyPassword,
    GlobalProxyBypass,
    CookieFilter,
    CookieDraftDomain,
    CookieDraftPath,
    CookieDraftName,
    CookieDraftValue,
    InterceptorBindAddr,
    InterceptorRuleMatch,
    InterceptorRuleTarget,
    MockHostPort,
    MockDraftMethod,
    MockDraftPath,
    MockDraftStatus,
    MockDraftBody,
    MockDraftHeaderRule,
    MockDraftQueryRule,
    MockDraftBodyRule,
    MockDraftDatasetPath,
    GitCommitMessage,
    GitNewBranchName,
    LoadIterations,
    LoadConcurrency,
    LoadDelayMs,
    LoadRampUpSecs,
    LoadRampDownSecs,
    LoadDurationSecs,
    LoadTargetRps,
    LoadStageVus(usize),
    LoadStageDurationSecs(usize),
    LoadThinkA,
    LoadThinkB,
    LoadBarrierGroupSize,
    LoadBarrierTimeoutMs,
    FlowWeightPercent,
    FlowLoopCount,
    FlowConditionIf,
    FlowLoopWhile,
    LoadDataFilePath,
    LoadMaxDurationMs,
    LoadMinSizeBytes,
    LoadMaxSizeBytes,
    LoadSlaP95Ms,
    LoadSlaP99Ms,
    LoadSlaErrorPct,
    LoadProfileName,
    LoadReqWeight(usize),
    LoadReqLoop(usize),
    LoadReqIf(usize),
    LoadReqWhile(usize),
    AiPromptInput,
    AiCustomCliName,
    AiCustomCliCmd,
    AiCustomCliArgs,
    AiHttpName,
    AiHttpUrl,
    AiHttpModel,
    AiHttpApiKey,
}

impl InputKey {
    /// Returns the syntax-highlighting language configured for code-editor
    /// controls, or `None` for plain inputs and textareas.
    pub fn editor_language(&self) -> Option<&'static str> {
        match self {
            InputKey::PreRequestScript
            | InputKey::PostResponseScript
            | InputKey::TestScript => Some("javascript"),
            InputKey::SpecText | InputKey::EnvCode => Some("toml"),
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
            InputKey::AuthSchemeField(i) => Some(ActiveInputTarget::AuthSchemeField(*i)),
            InputKey::SettingCertField(i) => Some(ActiveInputTarget::SettingCertField(*i)),
            InputKey::SettingProxyField(i) => Some(ActiveInputTarget::SettingProxyField(*i)),
            InputKey::BodyText => Some(ActiveInputTarget::BodyText),
            InputKey::PreRequestScript => Some(ActiveInputTarget::PreRequestScript),
            InputKey::PostResponseScript => Some(ActiveInputTarget::PostResponseScript),
            InputKey::TestScript => Some(ActiveInputTarget::TestScript),
            InputKey::AssertExpr(i) => Some(ActiveInputTarget::AssertExpr(*i)),
            InputKey::AssertExpected(i) => Some(ActiveInputTarget::AssertExpected(*i)),
            InputKey::ExtractorVar(i) => Some(ActiveInputTarget::ExtractorVar(*i)),
            InputKey::ExtractorExpr(i) => Some(ActiveInputTarget::ExtractorExpr(*i)),
            InputKey::SpecText => Some(ActiveInputTarget::SpecText),
            InputKey::CommandPaletteSearch
            | InputKey::RenameDialog
            | InputKey::MoveToSearch
            | InputKey::InlineRename
            | InputKey::EditorDialogField(_)
            | InputKey::ResponseBody
            | InputKey::ResponseRaw
            | InputKey::ResponseJson
            | InputKey::ResponseTests
            | InputKey::ResponseConsole
            | InputKey::ResponseTableSearch
            | InputKey::LogViewerText
            | InputKey::EnvName
            | InputKey::EnvVarKey(_)
            | InputKey::EnvVarValue(_)
            | InputKey::EnvCode
            | InputKey::GraphQLQuery
            | InputKey::GraphQLVariables
            | InputKey::WsMessageInput
            | InputKey::WsTemplateName
            | InputKey::GrpcService
            | InputKey::GrpcMethod
            | InputKey::GrpcPayload
            | InputKey::SoapOperation
            | InputKey::SoapNamespace
            | InputKey::SoapAction
            | InputKey::WsdlImportText
            | InputKey::VaultAddr
            | InputKey::VaultToken
            | InputKey::VaultNamespace
            | InputKey::VaultAwsRegion
            | InputKey::VaultAwsProfile
            | InputKey::KeychainService
            | InputKey::KeychainKey
            | InputKey::KeychainValue
            | InputKey::DomainCertDomain
            | InputKey::DomainCertPath
            | InputKey::DomainCertKeyPath
            | InputKey::DomainCertPassphrase
            | InputKey::GlobalProxyHost
            | InputKey::GlobalProxyPort
            | InputKey::GlobalProxyUsername
            | InputKey::GlobalProxyPassword
            | InputKey::GlobalProxyBypass
            | InputKey::CookieFilter
            | InputKey::CookieDraftDomain
            | InputKey::CookieDraftPath
            | InputKey::CookieDraftName
            | InputKey::CookieDraftValue
            | InputKey::InterceptorBindAddr
            | InputKey::InterceptorRuleMatch
            | InputKey::InterceptorRuleTarget
            | InputKey::MockHostPort
            | InputKey::MockDraftMethod
            | InputKey::MockDraftPath
            | InputKey::MockDraftStatus
            | InputKey::MockDraftBody
            | InputKey::MockDraftHeaderRule
            | InputKey::MockDraftQueryRule
            | InputKey::MockDraftBodyRule
            | InputKey::MockDraftDatasetPath
            | InputKey::GitCommitMessage
            | InputKey::GitNewBranchName
            | InputKey::LoadIterations
            | InputKey::LoadConcurrency
            | InputKey::LoadDelayMs
            | InputKey::LoadRampUpSecs
            | InputKey::LoadRampDownSecs
            | InputKey::LoadDurationSecs
            | InputKey::LoadTargetRps
            | InputKey::LoadStageVus(_)
            | InputKey::LoadStageDurationSecs(_)
            | InputKey::LoadThinkA
            | InputKey::LoadThinkB
            | InputKey::LoadBarrierGroupSize
            | InputKey::LoadBarrierTimeoutMs
            | InputKey::FlowWeightPercent
            | InputKey::FlowLoopCount
            | InputKey::FlowConditionIf
            | InputKey::FlowLoopWhile
            | InputKey::LoadDataFilePath
            | InputKey::LoadMaxDurationMs
            | InputKey::LoadMinSizeBytes
            | InputKey::LoadMaxSizeBytes
            | InputKey::LoadSlaP95Ms
            | InputKey::LoadSlaP99Ms
            | InputKey::LoadSlaErrorPct
            | InputKey::LoadProfileName
            | InputKey::LoadReqWeight(_)
            | InputKey::LoadReqLoop(_)
            | InputKey::LoadReqIf(_)
            | InputKey::LoadReqWhile(_)
            | InputKey::AiPromptInput
            | InputKey::AiCustomCliName
            | InputKey::AiCustomCliCmd
            | InputKey::AiCustomCliArgs
            | InputKey::AiHttpName
            | InputKey::AiHttpUrl
            | InputKey::AiHttpModel
            | InputKey::AiHttpApiKey
            | InputKey::VaultProfileName
            | InputKey::AwsProfileName
            | InputKey::KeePassProfileName
            | InputKey::KeePassDbPath
            | InputKey::KeePassKeyfilePath
            | InputKey::KeePassPassword => None,
        }
    }
}

/// Manages lifecycle and two-way synchronization of native `InputState`,
/// `TextareaState`, and `EditorState` entities.
pub struct InputRegistry {
    inputs: HashMap<InputKey, Entity<InputState>>,
    masked_inputs: HashSet<InputKey>,
    textareas: HashMap<InputKey, Entity<TextareaState>>,
    editors: HashMap<InputKey, Entity<EditorState>>,
    editor_configs: HashMap<InputKey, (String, bool)>,
    editor_decorations: HashMap<InputKey, TextDecorationCollection>,
    editor_context: Rc<RefCell<EditorContextData>>,
    template_context: Rc<RefCell<TemplateContext>>,
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
            masked_inputs: HashSet::new(),
            textareas: HashMap::new(),
            editors: HashMap::new(),
            editor_configs: HashMap::new(),
            editor_decorations: HashMap::new(),
            editor_context: Rc::new(RefCell::new(EditorContextData::default())),
            template_context: Rc::new(RefCell::new(TemplateContext::default())),
            subscriptions: Vec::new(),
        }
    }

    /// Updates the live request variables, query parameters, and headers used by
    /// editor autocomplete providers.
    pub fn set_editor_context(&self, data: EditorContextData) {
        *self.editor_context.borrow_mut() = data;
    }

    /// Updates the live cascading `{{...}}` template context used for color coding and tooltips.
    pub fn set_template_context(&self, ctx: TemplateContext) {
        *self.template_context.borrow_mut() = ctx;
    }

    /// Returns a handle to the shared live `{{...}}` template context.
    pub fn template_context(&self) -> Rc<RefCell<TemplateContext>> {
        Rc::clone(&self.template_context)
    }

    /// Re-synchronizes all live single-line `InputState` tokens and `EditorState` decorations
    /// against the current `template_context`.
    pub fn sync_all_template_highlights(&mut self, cx: &mut App) {
        let ctx = self.template_context.borrow().clone();
        for (key, entity) in &self.inputs {
            if !self.masked_inputs.contains(key) {
                sync_input_template_tokens(entity, &ctx, cx);
            }
        }
        for (key, entity) in &self.editors {
            let mut col = self.editor_decorations.remove(key);
            sync_editor_template_decorations(entity, &mut col, &ctx, cx);
            if let Some(col) = col {
                self.editor_decorations.insert(key.clone(), col);
            }
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
        if masked {
            self.masked_inputs.insert(key.clone());
        } else {
            self.masked_inputs.remove(&key);
        }

        if let Some(entity) = self.inputs.get(&key) {
            let entity = entity.clone();
            let current = entity.read(cx).value();
            if current.as_ref() != value {
                let val_owned = value.to_string();
                entity.update(cx, |state, cx| {
                    state.set_value(val_owned, window, cx);
                });
            }
            if !masked {
                let tpl_ctx = self.template_context.borrow().clone();
                sync_input_template_tokens(&entity, &tpl_ctx, cx);
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
        if !masked {
            let tpl_ctx = self.template_context.borrow().clone();
            sync_input_template_tokens(&entity, &tpl_ctx, cx);
        }
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
        let desired_cfg = (language.to_string(), true);
        if let Some(entity) = self.editors.get(&key) {
            let entity = entity.clone();
            if self.editor_configs.get(&key) != Some(&desired_cfg) {
                let lang_owned = language.to_string();
                entity.update(cx, |state, cx| {
                    state.set_highlighter(&lang_owned, cx);
                });
                self.editor_configs.insert(key.clone(), desired_cfg);
            }
            let current = entity.read(cx).value();
            if current.as_ref() != value {
                let val_owned = value.to_string();
                entity.update(cx, |state, cx| {
                    state.set_value(val_owned, window, cx);
                });
            }
            let tpl_ctx = self.template_context.borrow().clone();
            let mut col = self.editor_decorations.remove(&key);
            sync_editor_template_decorations(&entity, &mut col, &tpl_ctx, cx);
            if let Some(col) = col {
                self.editor_decorations.insert(key, col);
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
            InputKey::GraphQLQuery => Some(CompletionLanguage::GraphQL),
            _ => None,
        };
        let editor_context = Rc::clone(&self.editor_context);
        let tpl_hover_ctx = Rc::clone(&self.template_context);
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
            state.lsp_mut().hover_provider =
                Some(Rc::new(TemplateHoverProvider::new(tpl_hover_ctx)));
            state
        });

        let key_clone = key.clone();
        let sub = cx.subscribe(&entity, move |this, state, event, cx| {
            on_event(this, &key_clone, &state, event, cx);
        });
        self.subscriptions.push(sub);
        self.editor_configs.insert(key.clone(), desired_cfg);
        let tpl_ctx = self.template_context.borrow().clone();
        let mut col = None;
        sync_editor_template_decorations(&entity, &mut col, &tpl_ctx, cx);
        if let Some(col) = col {
            self.editor_decorations.insert(key.clone(), col);
        }
        self.editors.insert(key, entity.clone());
        entity
    }

    /// Ensures a read-only code `EditorState` exists for `key` with the given `language`
    /// and `soft_wrap`, recreating it if `language` or `soft_wrap` changed.
    #[allow(clippy::too_many_arguments)]
    pub fn ensure_readonly_editor<V: 'static>(
        &mut self,
        key: InputKey,
        value: &str,
        placeholder: &str,
        language: &str,
        soft_wrap: bool,
        window: &mut Window,
        cx: &mut Context<V>,
    ) -> Entity<EditorState> {
        let desired_cfg = (language.to_string(), soft_wrap);
        if self.editor_configs.get(&key) == Some(&desired_cfg) {
            if let Some(entity) = self.editors.get(&key) {
                let entity = entity.clone();
                let current = entity.read(cx).value();
                if current.as_ref() != value {
                    let val_owned = value.to_string();
                    entity.update(cx, |state, cx| {
                        state.set_value(val_owned, window, cx);
                    });
                }
                let tpl_ctx = self.template_context.borrow().clone();
                let mut col = self.editor_decorations.remove(&key);
                sync_editor_template_decorations(&entity, &mut col, &tpl_ctx, cx);
                if let Some(col) = col {
                    self.editor_decorations.insert(key, col);
                }
                return entity;
            }
        }

        self.editor_decorations.remove(&key);
        let placeholder_owned = placeholder.to_string();
        let value_owned = value.to_string();
        let language_owned = language.to_string();
        let tpl_hover_ctx = Rc::clone(&self.template_context);
        let entity = cx.new(|cx| {
            let mut state = EditorState::new(window, cx)
                .language(language_owned)
                .line_number(true)
                .soft_wrap(soft_wrap)
                .placeholder(placeholder_owned)
                .default_value(value_owned);
            state.lsp_mut().hover_provider =
                Some(Rc::new(TemplateHoverProvider::new(tpl_hover_ctx)));
            state
        });
        self.editor_configs.insert(key.clone(), desired_cfg);
        let tpl_ctx = self.template_context.borrow().clone();
        let mut col = None;
        sync_editor_template_decorations(&entity, &mut col, &tpl_ctx, cx);
        if let Some(col) = col {
            self.editor_decorations.insert(key.clone(), col);
        }
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
