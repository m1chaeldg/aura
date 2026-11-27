use crate::preferences::{AiCliProfile, AiHttpKind, AiHttpProfile, AiPreferences};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Known AI agent CLI descriptor used during auto-discovery.
#[derive(Debug, Clone, Copy)]
pub struct KnownCliSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub candidates: &'static [&'static str],
    pub default_args: &'static [&'static str],
}

/// Built-in catalog of supported local AI agent CLIs.
pub const KNOWN_AI_CLIS: &[KnownCliSpec] = &[
    KnownCliSpec {
        id: "cli:agy",
        name: "Antigravity CLI",
        candidates: &["agy", "antigravity"],
        default_args: &["-p", "{prompt}"],
    },
    KnownCliSpec {
        id: "cli:claude",
        name: "Claude Code CLI",
        candidates: &["claude"],
        default_args: &["-p", "{prompt}"],
    },
    KnownCliSpec {
        id: "cli:codex",
        name: "OpenAI Codex CLI",
        candidates: &["codex"],
        default_args: &["exec", "{prompt}"],
    },
    KnownCliSpec {
        id: "cli:copilot",
        name: "GitHub Copilot CLI",
        candidates: &["copilot", "github-copilot-cli"],
        default_args: &["-p", "{prompt}"],
    },
    KnownCliSpec {
        id: "cli:cursor",
        name: "Cursor Agent CLI",
        candidates: &["cursor-agent", "agent"],
        default_args: &["-p", "{prompt}"],
    },
    KnownCliSpec {
        id: "cli:gemini",
        name: "Gemini CLI",
        candidates: &["gemini"],
        default_args: &["-p", "{prompt}"],
    },
];

/// Summary of a discovery pass across local CLIs and local HTTP runtimes.
#[derive(Debug, Clone, Default)]
pub struct AiDiscoveryResult {
    pub cli_profiles: Vec<AiCliProfile>,
    pub local_http_profiles: Vec<AiHttpProfile>,
}

/// Searches a list of directories for an executable matching `name`.
/// On Windows, also checks `PATHEXT` extensions (`.exe`, `.cmd`, `.bat`, `.com`).
pub fn find_executable_in_paths(name: &str, search_dirs: &[PathBuf]) -> Option<PathBuf> {
    let direct = Path::new(name);
    if direct.components().count() > 1 && is_executable_file(direct) {
        return Some(direct.to_path_buf());
    }

    let extensions: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".to_string())
            .split(';')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_ascii_lowercase())
            .collect()
    } else {
        Vec::new()
    };

    for dir in search_dirs {
        if !dir.is_dir() {
            continue;
        }
        let candidate = dir.join(name);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
        if cfg!(windows) {
            let lower = name.to_ascii_lowercase();
            for ext in &extensions {
                if !lower.ends_with(ext) {
                    let with_ext = dir.join(format!("{name}{ext}"));
                    if is_executable_file(&with_ext) {
                        return Some(with_ext);
                    }
                }
            }
        }
    }
    None
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Collects `PATH` directories plus common user-local CLI install directories.
pub fn collect_cli_search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();

    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local").join("bin"));
        dirs.push(home.join(".npm-global").join("bin"));
        dirs.push(home.join(".cargo").join("bin"));
        dirs.push(home.join(".claude").join("bin"));
        dirs.push(home.join(".cursor").join("bin"));
        if cfg!(windows) {
            dirs.push(home.join("AppData").join("Roaming").join("npm"));
            dirs.push(home.join("AppData").join("Local").join("Programs"));
        }
    }

    dirs
}

/// Discovers installed AI agent CLIs synchronously (run on a background thread).
pub fn discover_local_clis() -> Vec<AiCliProfile> {
    let search_dirs = collect_cli_search_dirs();
    discover_local_clis_in_dirs(&search_dirs)
}

/// Pure testable CLI discovery against explicit directories.
pub fn discover_local_clis_in_dirs(search_dirs: &[PathBuf]) -> Vec<AiCliProfile> {
    let mut discovered = Vec::new();
    for spec in KNOWN_AI_CLIS {
        for candidate in spec.candidates {
            if let Some(found_path) = find_executable_in_paths(candidate, search_dirs) {
                discovered.push(AiCliProfile {
                    id: spec.id.to_string(),
                    name: spec.name.to_string(),
                    command: found_path.to_string_lossy().to_string(),
                    args: spec.default_args.iter().map(|s| (*s).to_string()).collect(),
                    enabled: true,
                    auto_discovered: true,
                    timeout_secs: 120,
                });
                break;
            }
        }
    }
    discovered
}

/// Probes common local OpenAI-compatible runtimes (Ollama, LM Studio).
pub async fn discover_local_http_runtimes() -> Vec<AiHttpProfile> {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(800))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut found = Vec::new();
    let probes = [
        ("http:ollama", "Ollama (Local)", "http://localhost:11434/v1", "llama3.2"),
        ("http:lmstudio", "LM Studio (Local)", "http://localhost:1234/v1", "local-model"),
    ];

    for (id, name, base_url, fallback_model) in probes {
        let models_url = format!("{base_url}/models");
        if let Ok(resp) = client.get(&models_url).send().await {
            if resp.status().is_success() {
                let model = resp
                    .json::<serde_json::Value>()
                    .await
                    .ok()
                    .and_then(|v| {
                        v.get("data")
                            .and_then(|d| d.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|m| m.get("id"))
                            .and_then(|id| id.as_str())
                            .map(ToString::to_string)
                    })
                    .unwrap_or_else(|| fallback_model.to_string());

                found.push(AiHttpProfile {
                    id: id.to_string(),
                    name: name.to_string(),
                    kind: AiHttpKind::OpenAiCompatible,
                    base_url: base_url.to_string(),
                    model,
                    api_key: String::new(),
                    enabled: true,
                    auto_discovered: true,
                });
            }
        }
    }

    found
}

/// Runs full auto-discovery (CLIs + local HTTP runtimes) off the main UI thread.
pub async fn discover_all() -> AiDiscoveryResult {
    let cli_profiles = tokio::task::spawn_blocking(discover_local_clis)
        .await
        .unwrap_or_default();
    let local_http_profiles = discover_local_http_runtimes().await;
    AiDiscoveryResult {
        cli_profiles,
        local_http_profiles,
    }
}

/// Merges newly discovered profiles into existing `AiPreferences` without overwriting
/// user-customized profiles or manual settings.
pub fn merge_discovery_into_preferences(prefs: &mut AiPreferences, discovery: &AiDiscoveryResult) {
    for cli in &discovery.cli_profiles {
        if let Some(existing) = prefs.cli_profiles.iter_mut().find(|p| p.id == cli.id) {
            if existing.auto_discovered {
                existing.command = cli.command.clone();
            }
        } else {
            prefs.cli_profiles.push(cli.clone());
        }
    }

    for http in &discovery.local_http_profiles {
        if let Some(existing) = prefs.http_profiles.iter_mut().find(|p| p.id == http.id) {
            if existing.auto_discovered && existing.model.is_empty() {
                existing.model = http.model.clone();
            }
        } else {
            prefs.http_profiles.push(http.clone());
        }
    }

    if prefs.active_provider_id.is_none() {
        prefs.active_provider_id = prefs
            .cli_profiles
            .iter()
            .find(|p| p.enabled)
            .map(|p| p.id.clone())
            .or_else(|| {
                prefs
                    .http_profiles
                    .iter()
                    .find(|p| p.enabled && (!p.api_key.is_empty() || p.auto_discovered))
                    .map(|p| p.id.clone())
            });
    }
}
