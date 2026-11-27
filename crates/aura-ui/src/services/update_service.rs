//! Asynchronous GitHub Releases auto-updater service.

use crate::preferences::UpdateCheckFrequency;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GITHUB_REPO: &str = "m1chaeldg/aura";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub version: String,
    pub name: String,
    pub body: String,
    pub html_url: String,
    pub published_at: String,
    pub asset_name: Option<String>,
    pub asset_download_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCheckOutcome {
    UpToDate {
        current_version: String,
    },
    UpdateAvailable(ReleaseInfo),
}

#[derive(Debug, Deserialize)]
struct GitHubReleaseDto {
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    body: Option<String>,
    html_url: String,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    assets: Vec<GitHubAssetDto>,
}

#[derive(Debug, Deserialize)]
struct GitHubAssetDto {
    name: String,
    browser_download_url: String,
}

pub struct UpdateService;

impl UpdateService {
    /// Returns true if an automatic update check should run given the user's frequency preference.
    pub fn should_check_now(
        freq: UpdateCheckFrequency,
        last_check_epoch: Option<i64>,
        now_epoch: i64,
    ) -> bool {
        match freq {
            UpdateCheckFrequency::Never => false,
            UpdateCheckFrequency::OnStartup => true,
            UpdateCheckFrequency::Daily => {
                last_check_epoch.is_none_or(|last| now_epoch.saturating_sub(last) >= 86_400)
            }
            UpdateCheckFrequency::Weekly => {
                last_check_epoch.is_none_or(|last| now_epoch.saturating_sub(last) >= 7 * 86_400)
            }
        }
    }

    /// Parses a version string like `v0.2.1` or `0.2.1` into `(major, minor, patch)`.
    pub fn parse_semver(raw: &str) -> Option<(u64, u64, u64)> {
        let trimmed = raw.trim().trim_start_matches('v').trim_start_matches('V');
        let core = trimmed.split('-').next()?.split('+').next()?;
        let mut parts = core.split('.');
        let major = parts.next()?.parse::<u64>().ok()?;
        let minor = parts.next().unwrap_or("0").parse::<u64>().ok()?;
        let patch = parts.next().unwrap_or("0").parse::<u64>().ok()?;
        Some((major, minor, patch))
    }

    pub fn is_newer_version(candidate: &str, current: &str) -> bool {
        match (Self::parse_semver(candidate), Self::parse_semver(current)) {
            (Some(cand), Some(curr)) => cand > curr,
            _ => false,
        }
    }

    /// Expected release asset filename for the current OS & architecture.
    pub fn expected_asset_name() -> &'static str {
        if cfg!(target_os = "windows") {
            "aura-windows-x86_64.zip"
        } else if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") {
                "aura-macos-aarch64.tar.gz"
            } else {
                "aura-macos-x86_64.tar.gz"
            }
        } else {
            "aura-linux-x86_64.tar.gz"
        }
    }

    /// Queries GitHub Releases API for the latest release of `m1chaeldg/aura`.
    pub async fn check_latest_release() -> Result<UpdateCheckOutcome> {
        let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");
        let client = reqwest::Client::builder()
            .user_agent(format!("aura-rest-client/{CURRENT_VERSION}"))
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .context("Failed to initialize HTTP client for update check")?;

        let resp = client
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .context("Failed to reach GitHub Releases API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("GitHub Releases API returned {status}: {text}");
        }

        let dto: GitHubReleaseDto = resp
            .json()
            .await
            .context("Failed to parse GitHub Release JSON")?;

        let version = dto
            .tag_name
            .trim()
            .trim_start_matches('v')
            .trim_start_matches('V')
            .to_string();

        if !Self::is_newer_version(&dto.tag_name, CURRENT_VERSION) {
            return Ok(UpdateCheckOutcome::UpToDate {
                current_version: CURRENT_VERSION.to_string(),
            });
        }

        let expected_asset = Self::expected_asset_name();
        let matched_asset = dto
            .assets
            .iter()
            .find(|a| a.name == expected_asset)
            .or_else(|| {
                dto.assets.iter().find(|a| {
                    let lower = a.name.to_ascii_lowercase();
                    if cfg!(target_os = "windows") {
                        lower.contains("windows") && lower.ends_with(".zip")
                    } else if cfg!(target_os = "macos") {
                        lower.contains("macos") && lower.ends_with(".tar.gz")
                    } else {
                        lower.contains("linux") && lower.ends_with(".tar.gz")
                    }
                })
            });

        Ok(UpdateCheckOutcome::UpdateAvailable(ReleaseInfo {
            tag_name: dto.tag_name,
            version,
            name: dto.name.unwrap_or_else(|| "New Aura Release".to_string()),
            body: dto.body.unwrap_or_default(),
            html_url: dto.html_url,
            published_at: dto.published_at.unwrap_or_default(),
            asset_name: matched_asset.map(|a| a.name.clone()),
            asset_download_url: matched_asset.map(|a| a.browser_download_url.clone()),
        }))
    }

    /// Downloads the release archive and replaces the running executable in-place.
    pub async fn download_and_install_update(download_url: &str, asset_name: &str) -> Result<String> {
        let client = reqwest::Client::builder()
            .user_agent(format!("aura-rest-client/{CURRENT_VERSION}"))
            .timeout(std::time::Duration::from_secs(120))
            .build()?;

        let bytes = client
            .get(download_url)
            .send()
            .await
            .context("Failed to download release archive")?
            .error_for_status()
            .context("Release download returned HTTP error")?
            .bytes()
            .await
            .context("Failed to read downloaded archive bytes")?;

        let staging_dir = std::env::temp_dir().join(format!("aura-update-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&staging_dir)?;
        let archive_path = staging_dir.join(asset_name);
        std::fs::write(&archive_path, &bytes)?;

        let extract_dir = staging_dir.join("extracted");
        std::fs::create_dir_all(&extract_dir)?;

        if asset_name.ends_with(".zip") {
            #[cfg(target_os = "windows")]
            {
                let status = std::process::Command::new("powershell")
                    .args([
                        "-NoProfile",
                        "-Command",
                        &format!(
                            "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                            archive_path.display(),
                            extract_dir.display()
                        ),
                    ])
                    .status()
                    .context("Failed to invoke Expand-Archive")?;
                if !status.success() {
                    anyhow::bail!("Failed to extract zip archive");
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                let status = std::process::Command::new("unzip")
                    .args(["-o", &archive_path.to_string_lossy(), "-d", &extract_dir.to_string_lossy()])
                    .status()
                    .context("Failed to invoke unzip")?;
                if !status.success() {
                    anyhow::bail!("Failed to extract zip archive");
                }
            }
        } else {
            let status = std::process::Command::new("tar")
                .args([
                    "-xzf",
                    &archive_path.to_string_lossy(),
                    "-C",
                    &extract_dir.to_string_lossy(),
                ])
                .status()
                .context("Failed to invoke tar")?;
            if !status.success() {
                anyhow::bail!("Failed to extract tar.gz archive");
            }
        }

        let bin_name = if cfg!(target_os = "windows") {
            "aura.exe"
        } else {
            "aura"
        };
        let new_bin = find_binary_recursive(&extract_dir, bin_name)
            .ok_or_else(|| anyhow::anyhow!("Updated binary '{bin_name}' not found in archive"))?;

        let current_exe = std::env::current_exe().context("Could not locate current executable")?;
        let backup_exe = current_exe.with_extension("old");
        if backup_exe.exists() {
            let _ = std::fs::remove_file(&backup_exe);
        }
        std::fs::rename(&current_exe, &backup_exe)
            .context("Failed to move current executable aside for update")?;
        if let Err(e) = std::fs::copy(&new_bin, &current_exe) {
            let _ = std::fs::rename(&backup_exe, &current_exe);
            return Err(e).context("Failed to place new binary into executable path");
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&current_exe, std::fs::Permissions::from_mode(0o755));
        }

        let _ = std::fs::remove_dir_all(&staging_dir);
        Ok(format!(
            "Update installed to {}. Restart Aura to launch the new version.",
            current_exe.display()
        ))
    }

    pub fn open_url_in_browser(url: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(["/c", "start", "", url])
                .spawn()
                .context("Failed to open browser")?;
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(url)
                .spawn()
                .context("Failed to open browser")?;
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            std::process::Command::new("xdg-open")
                .arg(url)
                .spawn()
                .context("Failed to open browser")?;
        }
        Ok(())
    }
}

fn find_binary_recursive(dir: &PathBuf, target_name: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_binary_recursive(&path, target_name) {
                return Some(found);
            }
        } else if path.file_name().and_then(|s| s.to_str()) == Some(target_name) {
            return Some(path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_comparison_handles_v_prefix_and_prerelease() {
        assert!(UpdateService::is_newer_version("v0.2.0", "0.1.0"));
        assert!(UpdateService::is_newer_version("v0.1.1", "0.1.0"));
        assert!(!UpdateService::is_newer_version("v0.1.0", "0.1.0"));
        assert!(!UpdateService::is_newer_version("0.0.9", "0.1.0"));
    }

    #[test]
    fn update_frequency_schedule_respects_intervals() {
        let now = 1_700_000_000;
        assert!(UpdateService::should_check_now(UpdateCheckFrequency::OnStartup, Some(now - 10), now));
        assert!(!UpdateService::should_check_now(UpdateCheckFrequency::Never, None, now));
        assert!(!UpdateService::should_check_now(UpdateCheckFrequency::Daily, Some(now - 3600), now));
        assert!(UpdateService::should_check_now(UpdateCheckFrequency::Daily, Some(now - 90_000), now));
        assert!(!UpdateService::should_check_now(UpdateCheckFrequency::Weekly, Some(now - 90_000), now));
        assert!(UpdateService::should_check_now(UpdateCheckFrequency::Weekly, Some(now - 700_000), now));
    }
}
