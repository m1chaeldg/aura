use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Frequency for automatic GitHub Release update checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum UpdateCheckFrequency {
    #[default]
    OnStartup,
    Daily,
    Weekly,
    Never,
}

impl UpdateCheckFrequency {
    pub const ALL: [Self; 4] = [Self::OnStartup, Self::Daily, Self::Weekly, Self::Never];

    pub fn label(&self) -> &'static str {
        match self {
            Self::OnStartup => "On Startup",
            Self::Daily => "Daily",
            Self::Weekly => "Weekly",
            Self::Never => "Never (Manual Only)",
        }
    }
}

/// Application preferences persisted across app launches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuraPreferences {
    #[serde(default)]
    pub last_workspace_path: Option<PathBuf>,
    #[serde(default = "default_auto_save_enabled")]
    pub auto_save_enabled: bool,
    #[serde(default = "default_auto_save_delay_ms")]
    pub auto_save_delay_ms: u64,
    #[serde(default = "default_split_ratio")]
    pub split_ratio: f32,
    #[serde(default = "default_layout_orientation")]
    pub layout_orientation: String,
    #[serde(default)]
    pub domain_client_certs: Vec<aura_core::model::ClientCertificateConfig>,
    #[serde(default)]
    pub vault_config: aura_engine::VaultConfig,
    #[serde(default)]
    pub global_proxy: aura_core::model::ProxyConfig,
    #[serde(default)]
    pub cookie_jar: aura_core::cookies::CookieJar,
    #[serde(default)]
    pub update_check_frequency: UpdateCheckFrequency,
    #[serde(default)]
    pub last_update_check_epoch: Option<i64>,
    #[serde(default)]
    pub ai: AiPreferences,
}

/// Protocol kind for HTTP-based AI endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AiHttpKind {
    #[default]
    OpenAiCompatible,
    Anthropic,
}

impl AiHttpKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::OpenAiCompatible => "OpenAI-Compatible",
            Self::Anthropic => "Anthropic Messages",
        }
    }
}

/// Configuration for a local AI Agent CLI (e.g. claude, codex, copilot, cursor, gemini, agy).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiCliProfile {
    pub id: String,
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_ai_true")]
    pub enabled: bool,
    #[serde(default)]
    pub auto_discovered: bool,
    #[serde(default = "default_ai_timeout_secs")]
    pub timeout_secs: u64,
}

/// Configuration for an HTTP AI endpoint (OpenAI, Anthropic, Ollama, LM Studio, custom).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiHttpProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: AiHttpKind,
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_ai_true")]
    pub enabled: bool,
    #[serde(default)]
    pub auto_discovered: bool,
}

/// Persisted AI agent & endpoint preferences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiPreferences {
    #[serde(default)]
    pub active_provider_id: Option<String>,
    #[serde(default)]
    pub cli_profiles: Vec<AiCliProfile>,
    #[serde(default = "default_ai_http_profiles")]
    pub http_profiles: Vec<AiHttpProfile>,
    #[serde(default = "default_ai_true")]
    pub redact_secrets: bool,
    #[serde(default)]
    pub auto_apply_edits: bool,
    #[serde(default = "default_ai_true")]
    pub attach_request_context: bool,
    #[serde(default = "default_ai_true")]
    pub attach_response_context: bool,
    #[serde(default)]
    pub custom_system_prompt: Option<String>,
}

fn default_ai_true() -> bool {
    true
}

fn default_ai_timeout_secs() -> u64 {
    120
}

fn default_ai_http_profiles() -> Vec<AiHttpProfile> {
    vec![
        AiHttpProfile {
            id: "http:openai".to_string(),
            name: "OpenAI (ChatGPT)".to_string(),
            kind: AiHttpKind::OpenAiCompatible,
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o".to_string(),
            api_key: "$OPENAI_API_KEY".to_string(),
            enabled: true,
            auto_discovered: false,
        },
        AiHttpProfile {
            id: "http:anthropic".to_string(),
            name: "Anthropic (Claude)".to_string(),
            kind: AiHttpKind::Anthropic,
            base_url: "https://api.anthropic.com/v1".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            api_key: "$ANTHROPIC_API_KEY".to_string(),
            enabled: true,
            auto_discovered: false,
        },
    ]
}

impl Default for AiPreferences {
    fn default() -> Self {
        Self {
            active_provider_id: None,
            cli_profiles: Vec::new(),
            http_profiles: default_ai_http_profiles(),
            redact_secrets: true,
            auto_apply_edits: false,
            attach_request_context: true,
            attach_response_context: true,
            custom_system_prompt: None,
        }
    }
}

fn default_auto_save_enabled() -> bool {
    true
}

fn default_auto_save_delay_ms() -> u64 {
    1000
}

fn default_split_ratio() -> f32 {
    0.5
}

fn default_layout_orientation() -> String {
    "horizontal".to_string()
}

impl Default for AuraPreferences {
    fn default() -> Self {
        Self {
            last_workspace_path: None,
            auto_save_enabled: true,
            auto_save_delay_ms: 1000,
            split_ratio: 0.5,
            layout_orientation: "horizontal".to_string(),
            domain_client_certs: Vec::new(),
            vault_config: aura_engine::VaultConfig::default(),
            global_proxy: aura_core::model::ProxyConfig::default(),
            cookie_jar: aura_core::cookies::CookieJar::default(),
            update_check_frequency: UpdateCheckFrequency::default(),
            last_update_check_epoch: None,
            ai: AiPreferences::default(),
        }
    }
}

impl AuraPreferences {
    /// Loads preferences from the default OS user config directory, or falls back to default.
    pub fn load() -> Self {
        let path = Self::default_config_path();
        match path {
            Some(ref p) => Self::load_from_file(p),
            None => Self::default(),
        }
    }

    /// Saves preferences to the default OS user config directory.
    pub fn save(&self) -> Result<PathBuf> {
        let path = Self::default_config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine OS config directory"))?;
        self.save_to_file(&path)?;
        Ok(path)
    }

    /// Loads preferences from a specified file path, falling back to default on error.
    pub fn load_from_file(path: &Path) -> Self {
        if !path.exists() {
            return Self::default();
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        serde_json::from_str(&content).unwrap_or_default()
    }

    /// Saves preferences to a specified file path, ensuring parent directory exists.
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config dir {}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    fn default_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("aura").join("preferences.json"))
    }
}
