use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Application preferences persisted across app launches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuraPreferences {
    #[serde(default)]
    pub last_workspace_path: Option<PathBuf>,
}

impl Default for AuraPreferences {
    fn default() -> Self {
        Self {
            last_workspace_path: None,
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
