//! Workspace, collection, folder, and environment variable models.

use super::{AuraRequest, AuthConfig};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Variable item in environment profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariableItem {
    pub key: String,
    pub value: String,
    pub enabled: bool,
    pub is_secret: bool,
}

impl VariableItem {
    /// Returns true if `value` is an external/keychain secret reference URI.
    pub fn is_secret_uri(val: &str) -> bool {
        let v = val.trim();
        v.starts_with("keyring://")
            || v.starts_with("vault://")
            || v.starts_with("aws-sm://")
            || v.starts_with("keepass://")
    }

    /// Canonical OS Keychain service name for an environment profile.
    pub fn env_keychain_service(env_name: &str) -> String {
        let slug: String = env_name
            .trim()
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let slug = slug.trim_matches('_');
        if slug.is_empty() {
            "env-default".to_string()
        } else {
            format!("env-{slug}")
        }
    }

    /// Canonical OS Keychain URI for a secret variable in an environment.
    pub fn env_keychain_uri(env_name: &str, key: &str) -> String {
        format!("keyring://{}/{}", Self::env_keychain_service(env_name), key.trim())
    }
}

/// Environment profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentProfile {
    pub id: String,
    pub name: String,
    pub variables: Vec<VariableItem>,
}

/// Collection Folder item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub auth: AuthConfig,
    pub requests: Vec<AuraRequest>,
    pub subfolders: Vec<Folder>,
}

/// Collection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub auth: AuthConfig,
    pub variables: HashMap<String, String>,
    pub requests: Vec<AuraRequest>,
    pub folders: Vec<Folder>,
}

/// Workspace root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    /// Absolute or relative path to workspace directory.
    pub root_path: PathBuf,
    pub collections: Vec<Collection>,
    pub environments: Vec<EnvironmentProfile>,
    pub active_environment_id: Option<String>,
}

impl Workspace {
    pub fn new(name: impl Into<String>, root_path: impl AsRef<Path>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            root_path: root_path.as_ref().to_path_buf(),
            collections: Vec::new(),
            environments: Vec::new(),
            active_environment_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    pub name: String,
    pub variables: IndexMap<String, VariableValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariableValue {
    pub value: String,
    pub is_secret: bool,
    pub keyring_ref: Option<String>,
}

impl VariableValue {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            is_secret: false,
            keyring_ref: None,
        }
    }

    pub fn secret(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            is_secret: true,
            keyring_ref: None,
        }
    }

    pub fn from_keyring(keyring_ref: impl Into<String>) -> Self {
        Self {
            value: String::new(),
            is_secret: true,
            keyring_ref: Some(keyring_ref.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionMeta {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub request_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub collections: Vec<CollectionMeta>,
    #[serde(default)]
    pub active_environment_id: Option<String>,
    #[serde(default)]
    pub open_tabs: Vec<String>,
    #[serde(default)]
    pub active_tab: Option<String>,
}
