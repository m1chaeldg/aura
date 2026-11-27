//! Workspace-persisted Load Test Profiles and ordered workspace request discovery.

use crate::workspace_meta::{is_ignored_workspace_dir, WorkspaceMetadata, WorkspaceOrderItem};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Target scope for a load test within a workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LoadTargetScope {
    #[default]
    ActiveRequest,
    Workspace,
    Folder {
        /// Path relative to the workspace root (using `/` separators).
        relative_path: String,
    },
}

/// Per-request inclusion and `[flow]` overrides stored in a Load Test Profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoadProfileRequestOverride {
    /// Path relative to the workspace root (using `/` separators).
    pub relative_path: String,
    #[serde(default = "default_true")]
    pub included: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight_percent: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub once_per_run: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition_if: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_while: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Named Load Test Profile persisted under `<workspace>/.aura/load-profiles/<name>.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceLoadProfile {
    pub name: String,
    #[serde(default)]
    pub target_scope: LoadTargetScope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_name: Option<String>,
    #[serde(default = "default_profile_mode")]
    pub profile_mode: String,
    #[serde(default = "default_iterations")]
    pub total_iterations: usize,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default)]
    pub delay_ms: u64,
    #[serde(default)]
    pub ramp_up_secs: f64,
    #[serde(default)]
    pub ramp_down_secs: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<f64>,
    #[serde(default)]
    pub stages: Vec<(usize, f64)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_rps: Option<f64>,
    #[serde(default = "default_think_mode")]
    pub think_mode: String,
    #[serde(default)]
    pub think_a: f64,
    #[serde(default)]
    pub think_b: f64,
    #[serde(default)]
    pub sync_barrier_enabled: bool,
    #[serde(default = "default_barrier_size")]
    pub sync_barrier_group_size: usize,
    #[serde(default = "default_barrier_timeout")]
    pub sync_barrier_timeout_ms: u64,
    #[serde(default)]
    pub weighted_routing: bool,
    #[serde(default)]
    pub data_feeder_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_feeder_path: Option<String>,
    #[serde(default = "default_sharing")]
    pub data_sharing: String,
    #[serde(default = "default_eof")]
    pub data_eof: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_size_bytes: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_size_bytes: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sla_p95_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sla_p99_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sla_error_rate_percent: Option<f64>,
    #[serde(default)]
    pub request_overrides: Vec<LoadProfileRequestOverride>,
}

fn default_profile_mode() -> String {
    "FixedIterations".to_string()
}
fn default_iterations() -> usize {
    20
}
fn default_concurrency() -> usize {
    4
}
fn default_think_mode() -> String {
    "None".to_string()
}
fn default_barrier_size() -> usize {
    4
}
fn default_barrier_timeout() -> u64 {
    5000
}
fn default_sharing() -> String {
    "AllVUs".to_string()
}
fn default_eof() -> String {
    "Recycle".to_string()
}

impl Default for WorkspaceLoadProfile {
    fn default() -> Self {
        Self {
            name: "Default Profile".to_string(),
            target_scope: LoadTargetScope::Workspace,
            environment_name: None,
            profile_mode: default_profile_mode(),
            total_iterations: default_iterations(),
            concurrency: default_concurrency(),
            delay_ms: 0,
            ramp_up_secs: 5.0,
            ramp_down_secs: 5.0,
            duration_secs: Some(20.0),
            stages: vec![(5, 10.0), (20, 15.0), (5, 10.0)],
            target_rps: None,
            think_mode: default_think_mode(),
            think_a: 200.0,
            think_b: 600.0,
            sync_barrier_enabled: false,
            sync_barrier_group_size: default_barrier_size(),
            sync_barrier_timeout_ms: default_barrier_timeout(),
            weighted_routing: false,
            data_feeder_enabled: false,
            data_feeder_path: None,
            data_sharing: default_sharing(),
            data_eof: default_eof(),
            max_duration_ms: None,
            min_size_bytes: None,
            max_size_bytes: None,
            sla_p95_ms: None,
            sla_p99_ms: None,
            sla_error_rate_percent: None,
            request_overrides: Vec::new(),
        }
    }
}

/// Directory inside a workspace where load profiles are stored.
pub fn load_profiles_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".aura").join("load-profiles")
}

/// Sanitizes a profile name into a safe filename stem.
pub fn sanitize_profile_filename(name: &str) -> String {
    let cleaned: String = name
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
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "profile".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Lists all saved load profiles in `<workspace_root>/.aura/load-profiles/`.
pub fn list_load_profiles(workspace_root: &Path) -> Result<Vec<WorkspaceLoadProfile>> {
    let dir = load_profiles_dir(workspace_root);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut profiles = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            if let Ok(profile) = serde_json::from_str::<WorkspaceLoadProfile>(&content) {
                profiles.push(profile);
            }
        }
    }
    profiles.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(profiles)
}

/// Loads a single named load profile from `<workspace_root>/.aura/load-profiles/`.
pub fn load_load_profile(workspace_root: &Path, name: &str) -> Result<WorkspaceLoadProfile> {
    let dir = load_profiles_dir(workspace_root);
    let direct = dir.join(format!("{}.json", sanitize_profile_filename(name)));
    if direct.exists() {
        let content = fs::read_to_string(&direct)?;
        return Ok(serde_json::from_str(&content)?);
    }
    for p in list_load_profiles(workspace_root)? {
        if p.name.eq_ignore_ascii_case(name.trim()) {
            return Ok(p);
        }
    }
    Err(anyhow!("Load profile '{}' not found in {}", name, dir.display()))
}

/// Saves a load profile to `<workspace_root>/.aura/load-profiles/<name>.json`.
pub fn save_load_profile(workspace_root: &Path, profile: &WorkspaceLoadProfile) -> Result<PathBuf> {
    let dir = load_profiles_dir(workspace_root);
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    let path = dir.join(format!("{}.json", sanitize_profile_filename(&profile.name)));
    let json = serde_json::to_string_pretty(profile)?;
    fs::write(&path, json)?;
    Ok(path)
}

/// Deletes a named load profile from `<workspace_root>/.aura/load-profiles/`.
pub fn delete_load_profile(workspace_root: &Path, name: &str) -> Result<()> {
    let path = load_profiles_dir(workspace_root)
        .join(format!("{}.json", sanitize_profile_filename(name)));
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Converts an absolute path inside `workspace_root` to a `/`‑separated relative path string.
pub fn to_workspace_relative_string(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Resolves a workspace-relative (or absolute) path against `workspace_root`.
pub fn resolve_workspace_path(workspace_root: &Path, raw: &str) -> PathBuf {
    let p = PathBuf::from(raw.trim());
    if p.is_absolute() {
        p
    } else {
        workspace_root.join(p)
    }
}

/// Discovers `.aura` and `.toml` request files in `root` (or `scope_folder` inside `root`),
/// respecting `workspace.json` ordering when present.
pub fn collect_ordered_request_paths(
    root: &Path,
    scope_folder: Option<&Path>,
) -> Result<Vec<PathBuf>> {
    if !root.exists() {
        return Err(anyhow!("Path does not exist: {}", root.display()));
    }
    if root.is_file() {
        return Ok(vec![root.to_path_buf()]);
    }

    let existing_meta = WorkspaceMetadata::load_from_disk(root).ok();
    let reconciled = WorkspaceMetadata::reconcile_with_disk(root, existing_meta)?;

    let mut ordered = Vec::new();
    let mut seen = HashSet::new();
    collect_from_order_items(root, &reconciled.order, &mut ordered, &mut seen);

    // Fallback: include any request files not covered by reconcile_with_disk
    collect_remaining_files(root, &mut ordered, &mut seen)?;

    if let Some(folder) = scope_folder {
        let abs_folder = if folder.is_absolute() {
            folder.to_path_buf()
        } else {
            root.join(folder)
        };
        ordered.retain(|p| p.starts_with(&abs_folder));
    }

    Ok(ordered)
}

fn collect_from_order_items(
    parent_dir: &Path,
    items: &[WorkspaceOrderItem],
    out: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
) {
    for item in items {
        match item {
            WorkspaceOrderItem::Folder { name, order } => {
                if is_ignored_workspace_dir(name) {
                    continue;
                }
                let sub = parent_dir.join(name);
                if sub.is_dir() {
                    collect_from_order_items(&sub, order, out, seen);
                }
            }
            WorkspaceOrderItem::Request { name } => {
                let candidate = parent_dir.join(name);
                if candidate.is_file() && is_request_file(&candidate) && seen.insert(candidate.clone())
                {
                    out.push(candidate);
                }
            }
        }
    }
}

fn collect_remaining_files(
    dir: &Path,
    out: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
) -> Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|x| x.path()))
        .collect();
    entries.sort();

    for path in entries {
        if path.is_dir() {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if !is_ignored_workspace_dir(name) && name != "target" {
                collect_remaining_files(&path, out, seen)?;
            }
        } else if is_request_file(&path) && seen.insert(path.clone()) {
            out.push(path);
        }
    }
    Ok(())
}

fn is_request_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("aura") | Some("toml")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("{prefix}_{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_load_profile_save_list_load_delete() {
        let root = temp_dir("aura_lp_crud");
        let mut prof = WorkspaceLoadProfile::default();
        prof.name = "Smoke 25 VUs".to_string();
        prof.concurrency = 25;
        prof.target_scope = LoadTargetScope::Folder {
            relative_path: "auth".to_string(),
        };
        prof.environment_name = Some("Staging".to_string());
        prof.request_overrides.push(LoadProfileRequestOverride {
            relative_path: "auth/login.aura".to_string(),
            included: true,
            weight_percent: Some(40),
            once_per_run: Some(true),
            loop_count: Some(1),
            condition_if: None,
            loop_while: None,
        });

        let saved = save_load_profile(&root, &prof).unwrap();
        assert!(saved.exists());

        let list = list_load_profiles(&root).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Smoke 25 VUs");
        assert_eq!(list[0].concurrency, 25);

        let loaded = load_load_profile(&root, "Smoke 25 VUs").unwrap();
        assert_eq!(loaded, prof);

        delete_load_profile(&root, "Smoke 25 VUs").unwrap();
        assert!(list_load_profiles(&root).unwrap().is_empty());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_collect_ordered_request_paths_respects_workspace_json_and_folder_scope() {
        let root = temp_dir("aura_lp_order");
        let auth_dir = root.join("auth");
        let users_dir = root.join("users");
        let env_dir = root.join("environment");
        fs::create_dir_all(&auth_dir).unwrap();
        fs::create_dir_all(&users_dir).unwrap();
        fs::create_dir_all(&env_dir).unwrap();

        fs::write(auth_dir.join("login.aura"), "").unwrap();
        fs::write(auth_dir.join("refresh.aura"), "").unwrap();
        fs::write(users_dir.join("list.aura"), "").unwrap();
        fs::write(env_dir.join("dev.toml"), "").unwrap();

        // Explicit workspace.json ordering: users/list.aura before auth/refresh.aura before auth/login.aura
        let meta = WorkspaceMetadata {
            version: 1,
            name: "test".to_string(),
            order: vec![
                WorkspaceOrderItem::Folder {
                    name: "users".to_string(),
                    order: vec![WorkspaceOrderItem::Request {
                        name: "list.aura".to_string(),
                    }],
                },
                WorkspaceOrderItem::Folder {
                    name: "auth".to_string(),
                    order: vec![
                        WorkspaceOrderItem::Request {
                            name: "refresh.aura".to_string(),
                        },
                        WorkspaceOrderItem::Request {
                            name: "login.aura".to_string(),
                        },
                    ],
                },
            ],
        };
        meta.save_to_disk(&root).unwrap();

        let all = collect_ordered_request_paths(&root, None).unwrap();
        let rels: Vec<String> = all
            .iter()
            .map(|p| to_workspace_relative_string(&root, p))
            .collect();
        assert_eq!(
            rels,
            vec!["users/list.aura", "auth/refresh.aura", "auth/login.aura"]
        );

        let auth_only = collect_ordered_request_paths(&root, Some(Path::new("auth"))).unwrap();
        let auth_rels: Vec<String> = auth_only
            .iter()
            .map(|p| to_workspace_relative_string(&root, p))
            .collect();
        assert_eq!(auth_rels, vec!["auth/refresh.aura", "auth/login.aura"]);

        let _ = fs::remove_dir_all(&root);
    }
}
