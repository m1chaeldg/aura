//! Environment Profile TOML persistence (`<workspace>/environment/<name>.toml`).
//! Disabled environment variables serialize with trailing tilde (`key~ = "value"`),
//! matching Aura's `.aura` / TOML convention for params and headers.

use anyhow::{Context, Result};
use aura_core::model::{EnvironmentProfile, VariableItem};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Serialized TOML representation of an `EnvironmentProfile`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlEnvironmentSchema {
    pub name: String,
    #[serde(default)]
    pub variables: BTreeMap<String, String>,
    /// Secret variables: values are never stored in plaintext.
    /// Stores either `"keyring"` (local OS Keychain) or a secret URI (`vault://`, `aws-sm://`, `keepass://`, `keyring://`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secrets: BTreeMap<String, String>,
}

/// Sanitize an environment name into a safe filesystem stem.
pub fn sanitize_env_filename(name: &str) -> String {
    let sanitized: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = sanitized.trim().replace(' ', "_");
    if trimmed.is_empty() {
        "environment".to_string()
    } else {
        trimmed
    }
}

/// Returns the `<workspace_root>/environment` directory path.
pub fn environments_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join("environment")
}

/// Serialize an `EnvironmentProfile` into TOML, encoding disabled keys with trailing `~`.
/// Variables with `is_secret = true` or a secret URI value are serialized under `[secrets]`
/// without ever writing plaintext secret values to disk.
pub fn environment_to_toml(env: &EnvironmentProfile) -> Result<String> {
    let mut vars_map = BTreeMap::new();
    let mut secrets_map = BTreeMap::new();
    for v in &env.variables {
        let raw_key = v.key.trim();
        if raw_key.is_empty() {
            continue;
        }
        let key = if v.enabled {
            raw_key.to_string()
        } else {
            format!("{}~", raw_key)
        };
        if v.is_secret || VariableItem::is_secret_uri(&v.value) {
            let ref_val = if VariableItem::is_secret_uri(&v.value) {
                v.value.trim().to_string()
            } else {
                "keyring".to_string()
            };
            secrets_map.insert(key, ref_val);
        } else {
            vars_map.insert(key, v.value.clone());
        }
    }

    let schema = TomlEnvironmentSchema {
        name: env.name.clone(),
        variables: vars_map,
        secrets: secrets_map,
    };
    toml::to_string_pretty(&schema).context("Failed to serialize environment to TOML")
}

fn parse_tilde_key(k: String) -> (String, bool) {
    if let Some(stripped) = k.strip_suffix('~') {
        (stripped.to_string(), false)
    } else if let Some(stripped) = k.strip_prefix('~') {
        (stripped.to_string(), false)
    } else {
        (k, true)
    }
}

/// Parse a TOML string into an `EnvironmentProfile`.
/// Keys with trailing `~` (`key~`) or leading `~` (`~key`) are marked `enabled = false`.
pub fn toml_to_environment(toml_str: &str, existing_id: Option<&str>) -> Result<EnvironmentProfile> {
    let schema: TomlEnvironmentSchema =
        toml::from_str(toml_str).context("Failed to parse TOML environment")?;

    let mut variables = Vec::with_capacity(schema.variables.len() + schema.secrets.len());
    for (k, v) in schema.variables {
        let (clean_key, enabled) = parse_tilde_key(k);
        let is_secret = VariableItem::is_secret_uri(&v);
        variables.push(VariableItem {
            key: clean_key,
            value: v,
            enabled,
            is_secret,
        });
    }
    for (k, v) in schema.secrets {
        let (clean_key, enabled) = parse_tilde_key(k);
        let resolved_ref = if v.trim().is_empty() || v.trim().eq_ignore_ascii_case("keyring") {
            VariableItem::env_keychain_uri(&schema.name, &clean_key)
        } else {
            v
        };
        variables.push(VariableItem {
            key: clean_key,
            value: resolved_ref,
            enabled,
            is_secret: true,
        });
    }

    Ok(EnvironmentProfile {
        id: existing_id
            .map(str::to_string)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        name: schema.name,
        variables,
    })
}

/// Load all `.toml` environment profiles from `<workspace_root>/environment/`.
pub fn load_environments_from_dir(workspace_root: &Path) -> Vec<(PathBuf, EnvironmentProfile)> {
    let dir = environments_dir(workspace_root);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&path) {
            let fallback_id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| format!("env:{s}"));
            if let Ok(mut env) = toml_to_environment(&content, fallback_id.as_deref()) {
                if env.name.trim().is_empty() {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        env.name = stem.to_string();
                    }
                }
                out.push((path, env));
            }
        }
    }
    out.sort_by(|a, b| a.1.name.to_lowercase().cmp(&b.1.name.to_lowercase()));
    out
}

/// Save an `EnvironmentProfile` to `<workspace_root>/environment/<name>.toml`.
/// If `old_path` is provided and differs from the new path, removes `old_path` after writing.
pub fn save_environment_to_dir(
    workspace_root: &Path,
    env: &EnvironmentProfile,
    old_path: Option<&Path>,
) -> Result<PathBuf> {
    let dir = environments_dir(workspace_root);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create environment directory '{}'", dir.display()))?;

    let filename = format!("{}.toml", sanitize_env_filename(&env.name));
    let target_path = dir.join(filename);
    let toml_content = environment_to_toml(env)?;
    std::fs::write(&target_path, toml_content)
        .with_context(|| format!("Failed to write environment file '{}'", target_path.display()))?;

    if let Some(prev) = old_path {
        if prev != target_path && prev.exists() {
            let _ = std::fs::remove_file(prev);
        }
    }

    Ok(target_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_toml_roundtrip_with_disabled_tilde() {
        let env = EnvironmentProfile {
            id: "env-1".to_string(),
            name: "Production".to_string(),
            variables: vec![
                VariableItem {
                    key: "BASE_URL".to_string(),
                    value: "https://api.example.com".to_string(),
                    enabled: true,
                    is_secret: false,
                },
                VariableItem {
                    key: "DEBUG_TOKEN".to_string(),
                    value: "dev-only".to_string(),
                    enabled: false,
                    is_secret: false,
                },
            ],
        };

        let toml_str = environment_to_toml(&env).unwrap();
        assert!(toml_str.contains("BASE_URL = \"https://api.example.com\""));
        assert!(toml_str.contains("\"DEBUG_TOKEN~\" = \"dev-only\""));

        let parsed = toml_to_environment(&toml_str, Some("env-1")).unwrap();
        assert_eq!(parsed.name, "Production");
        assert_eq!(parsed.variables.len(), 2);
        let base = parsed.variables.iter().find(|v| v.key == "BASE_URL").unwrap();
        assert!(base.enabled);
        let dbg = parsed.variables.iter().find(|v| v.key == "DEBUG_TOKEN").unwrap();
        assert!(!dbg.enabled);
        assert_eq!(dbg.value, "dev-only");
    }

    #[test]
    fn test_toml_to_environment_supports_leading_tilde() {
        let raw = "name = \"Staging\"\n[variables]\n\"~OLD_KEY\" = \"123\"\n";
        let parsed = toml_to_environment(raw, None).unwrap();
        assert_eq!(parsed.variables[0].key, "OLD_KEY");
        assert!(!parsed.variables[0].enabled);
    }

    #[test]
    fn test_environment_toml_secrets_never_persist_plaintext() {
        let env = EnvironmentProfile {
            id: "env-sec".to_string(),
            name: "Prod API".to_string(),
            variables: vec![
                VariableItem {
                    key: "BASE_URL".to_string(),
                    value: "https://api.prod.internal".to_string(),
                    enabled: true,
                    is_secret: false,
                },
                VariableItem {
                    key: "LOCAL_SECRET".to_string(),
                    value: "super_plaintext_secret_value".to_string(),
                    enabled: true,
                    is_secret: true,
                },
                VariableItem {
                    key: "VAULT_SECRET".to_string(),
                    value: "vault://prod@secret/data/api#token".to_string(),
                    enabled: true,
                    is_secret: true,
                },
                VariableItem {
                    key: "DISABLED_KP".to_string(),
                    value: "keepass://work@API/Stripe#Password".to_string(),
                    enabled: false,
                    is_secret: true,
                },
            ],
        };

        let toml_str = environment_to_toml(&env).unwrap();
        assert!(!toml_str.contains("super_plaintext_secret_value"));
        assert!(toml_str.contains("[secrets]"));
        assert!(toml_str.contains("LOCAL_SECRET = \"keyring\""));
        assert!(toml_str.contains("VAULT_SECRET = \"vault://prod@secret/data/api#token\""));
        assert!(toml_str.contains("\"DISABLED_KP~\" = \"keepass://work@API/Stripe#Password\""));

        let parsed = toml_to_environment(&toml_str, Some("env-sec")).unwrap();
        let local = parsed.variables.iter().find(|v| v.key == "LOCAL_SECRET").unwrap();
        assert!(local.is_secret);
        assert!(local.enabled);
        assert_eq!(local.value, "keyring://env-prod_api/LOCAL_SECRET");

        let vault = parsed.variables.iter().find(|v| v.key == "VAULT_SECRET").unwrap();
        assert!(vault.is_secret);
        assert_eq!(vault.value, "vault://prod@secret/data/api#token");

        let disabled = parsed.variables.iter().find(|v| v.key == "DISABLED_KP").unwrap();
        assert!(disabled.is_secret);
        assert!(!disabled.enabled);
        assert_eq!(disabled.value, "keepass://work@API/Stripe#Password");
    }
}
