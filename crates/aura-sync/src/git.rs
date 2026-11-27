//! Built-in Git backend for Aura workspaces (status, diff, staging, commit, branch, push/pull).
//! Designed to execute off the GPUI UI thread via `aura-ui` background workers.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitFileStatusKind {
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Conflicted,
}

impl GitFileStatusKind {
    pub fn short_code(&self) -> &'static str {
        match self {
            Self::Modified => "M",
            Self::Added => "A",
            Self::Deleted => "D",
            Self::Renamed => "R",
            Self::Copied => "C",
            Self::Untracked => "U",
            Self::Conflicted => "!",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Modified => "Modified",
            Self::Added => "Added",
            Self::Deleted => "Deleted",
            Self::Renamed => "Renamed",
            Self::Copied => "Copied",
            Self::Untracked => "Untracked",
            Self::Conflicted => "Conflict",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitFileEntry {
    pub path: String,
    pub staged: bool,
    pub kind: GitFileStatusKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitBranchInfo {
    pub name: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitCommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub relative_time: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitStatusSnapshot {
    pub is_repo: bool,
    pub repo_root: Option<PathBuf>,
    pub branch: String,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub staged: Vec<GitFileEntry>,
    pub unstaged: Vec<GitFileEntry>,
    pub branches: Vec<GitBranchInfo>,
    pub recent_commits: Vec<GitCommitInfo>,
    pub has_remote: bool,
}

impl GitStatusSnapshot {
    pub fn total_changes(&self) -> usize {
        self.staged.len() + self.unstaged.len()
    }
}

fn build_git_command(dir: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir);
    // Avoid blocking GUI flows with interactive terminal credential prompts
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn run_git(dir: &Path, args: &[&str]) -> Result<String> {
    if !dir.exists() {
        return Err(anyhow!("Workspace directory does not exist: {}", dir.display()));
    }
    let output = build_git_command(dir)
        .args(args)
        .output()
        .context("Failed to execute `git` command (ensure git is installed on PATH)")?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(if stdout.trim().is_empty() && !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else {
            stdout
        })
    } else {
        let msg = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else if !stdout.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            format!("git {:?} exited with {}", args, output.status)
        };
        Err(anyhow!("{msg}"))
    }
}

/// Initializes a new Git repository inside `dir`.
pub fn git_init(dir: &Path) -> Result<String> {
    let out = run_git(dir, &["init"])?;
    Ok(if out.trim().is_empty() {
        format!("Initialized Git repository in {}", dir.display())
    } else {
        out.trim().to_string()
    })
}

/// Queries full repository status, branches, remotes, and recent commit log.
pub fn git_status(dir: &Path) -> Result<GitStatusSnapshot> {
    if !dir.exists() {
        return Ok(GitStatusSnapshot::default());
    }

    let root_res = run_git(dir, &["rev-parse", "--show-toplevel"]);
    let Ok(root_str) = root_res else {
        return Ok(GitStatusSnapshot::default());
    };
    let repo_root = PathBuf::from(root_str.trim());

    let porcelain = run_git(
        &repo_root,
        &["status", "--porcelain=v1", "-b", "--untracked-files=all"],
    )?;

    let mut snap = GitStatusSnapshot {
        is_repo: true,
        repo_root: Some(repo_root.clone()),
        branch: "HEAD".to_string(),
        ..Default::default()
    };

    for (idx, line) in porcelain.lines().enumerate() {
        if idx == 0 && line.starts_with("## ") {
            parse_branch_header(&line[3..], &mut snap);
            continue;
        }
        if line.len() < 4 {
            continue;
        }
        let bytes = line.as_bytes();
        let x = bytes[0] as char;
        let y = bytes[1] as char;
        let raw_path = &line[3..];
        let path = clean_porcelain_path(raw_path);

        if x == '?' && y == '?' {
            snap.unstaged.push(GitFileEntry {
                path,
                staged: false,
                kind: GitFileStatusKind::Untracked,
            });
            continue;
        }

        if let Some(kind) = map_conflict_codes(x, y) {
            snap.unstaged.push(GitFileEntry {
                path,
                staged: false,
                kind,
            });
            continue;
        }

        if let Some(staged_kind) = map_status_char(x) {
            snap.staged.push(GitFileEntry {
                path: path.clone(),
                staged: true,
                kind: staged_kind,
            });
        }
        if let Some(work_kind) = map_status_char(y) {
            snap.unstaged.push(GitFileEntry {
                path,
                staged: false,
                kind: work_kind,
            });
        }
    }

    // List local branches
    if let Ok(branch_out) = run_git(
        &repo_root,
        &["for-each-ref", "--format=%(refname:short)|%(HEAD)", "refs/heads"],
    ) {
        for line in branch_out.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some((name, head_marker)) = trimmed.rsplit_once('|') {
                snap.branches.push(GitBranchInfo {
                    name: name.to_string(),
                    is_current: head_marker.trim() == "*",
                });
            }
        }
    }
    if snap.branches.is_empty() && !snap.branch.is_empty() && snap.branch != "HEAD" {
        snap.branches.push(GitBranchInfo {
            name: snap.branch.clone(),
            is_current: true,
        });
    }

    // Check configured remotes
    if let Ok(remotes) = run_git(&repo_root, &["remote"]) {
        snap.has_remote = !remotes.trim().is_empty();
    }

    // Recent commit log (ignore error on empty repo with unborn HEAD)
    if let Ok(log_out) = run_git(
        &repo_root,
        &[
            "log",
            "-n",
            "12",
            "--pretty=format:%H\x1f%h\x1f%s\x1f%an\x1f%ar",
        ],
    ) {
        for line in log_out.lines() {
            let parts: Vec<&str> = line.split('\x1f').collect();
            if parts.len() == 5 {
                snap.recent_commits.push(GitCommitInfo {
                    hash: parts[0].to_string(),
                    short_hash: parts[1].to_string(),
                    message: parts[2].to_string(),
                    author: parts[3].to_string(),
                    relative_time: parts[4].to_string(),
                });
            }
        }
    }

    Ok(snap)
}

fn parse_branch_header(header: &str, snap: &mut GitStatusSnapshot) {
    let trimmed = header.trim();
    if let Some(rest) = trimmed.strip_prefix("No commits yet on ") {
        snap.branch = rest.trim().to_string();
        return;
    }
    if let Some(rest) = trimmed.strip_prefix("Initial commit on ") {
        snap.branch = rest.trim().to_string();
        return;
    }

    let (branch_part, counts_part) = if let Some(idx) = trimmed.rfind(" [") {
        if trimmed.ends_with(']') {
            (&trimmed[..idx], Some(&trimmed[idx + 2..trimmed.len() - 1]))
        } else {
            (trimmed, None)
        }
    } else {
        (trimmed, None)
    };

    if let Some((local, upstream)) = branch_part.split_once("...") {
        snap.branch = local.trim().to_string();
        if !upstream.trim().is_empty() {
            snap.upstream = Some(upstream.trim().to_string());
        }
    } else {
        snap.branch = branch_part.trim().to_string();
    }

    if let Some(counts) = counts_part {
        for segment in counts.split(',') {
            let s = segment.trim();
            if let Some(n) = s.strip_prefix("ahead ") {
                snap.ahead = n.trim().parse::<usize>().unwrap_or(0);
            } else if let Some(n) = s.strip_prefix("behind ") {
                snap.behind = n.trim().parse::<usize>().unwrap_or(0);
            }
        }
    }
}

fn clean_porcelain_path(raw: &str) -> String {
    let path_str = if let Some((_, new_path)) = raw.split_once(" -> ") {
        new_path
    } else {
        raw
    };
    let trimmed = path_str.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        trimmed[1..trimmed.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else {
        trimmed.to_string()
    }
}

fn map_status_char(c: char) -> Option<GitFileStatusKind> {
    match c {
        'M' | 'T' => Some(GitFileStatusKind::Modified),
        'A' => Some(GitFileStatusKind::Added),
        'D' => Some(GitFileStatusKind::Deleted),
        'R' => Some(GitFileStatusKind::Renamed),
        'C' => Some(GitFileStatusKind::Copied),
        '?' => Some(GitFileStatusKind::Untracked),
        _ => None,
    }
}

fn map_conflict_codes(x: char, y: char) -> Option<GitFileStatusKind> {
    match (x, y) {
        ('U', _) | (_, 'U') | ('A', 'A') | ('D', 'D') => Some(GitFileStatusKind::Conflicted),
        _ => None,
    }
}

/// Returns a unified diff preview for a staged, modified, or untracked file.
pub fn git_diff(dir: &Path, file_path: &str, staged: bool) -> Result<String> {
    let root = repo_root_or(dir)?;
    let args: Vec<&str> = if staged {
        vec!["diff", "--cached", "--no-color", "--", file_path]
    } else {
        vec!["diff", "--no-color", "--", file_path]
    };
    let diff = run_git(&root, &args)?;
    if !diff.trim().is_empty() {
        return Ok(diff);
    }

    // If untracked (or brand new staged file before first commit edge case), synthesize unified diff preview
    if !staged {
        let abs = root.join(file_path);
        if abs.is_file() {
            let bytes = std::fs::read(&abs).context("Failed to read file for diff preview")?;
            if bytes.len() > 128 * 1024 {
                return Ok(format!(
                    "--- /dev/null\n+++ b/{file_path}\n@@ -0,0 +1,1 @@\n+[File too large to preview ({} bytes)]",
                    bytes.len()
                ));
            }
            match std::str::from_utf8(&bytes) {
                Ok(text) => {
                    let line_count = text.lines().count().max(1);
                    let mut preview = format!(
                        "--- /dev/null\n+++ b/{file_path}\n@@ -0,0 +1,{line_count} @@\n"
                    );
                    for l in text.lines() {
                        preview.push('+');
                        preview.push_str(l);
                        preview.push('\n');
                    }
                    return Ok(preview);
                }
                Err(_) => {
                    return Ok(format!("Binary file `{file_path}` ({} bytes)", bytes.len()));
                }
            }
        }
    }

    Ok("(No line-level changes to display)".to_string())
}

/// Stages either a specific path or all changes when `path` is `None`.
pub fn git_stage(dir: &Path, path: Option<&str>) -> Result<()> {
    let root = repo_root_or(dir)?;
    match path {
        Some(p) => {
            run_git(&root, &["add", "--", p])?;
        }
        None => {
            run_git(&root, &["add", "-A"])?;
        }
    }
    Ok(())
}

/// Unstages either a specific path or all staged files when `path` is `None`.
pub fn git_unstage(dir: &Path, path: Option<&str>) -> Result<()> {
    let root = repo_root_or(dir)?;
    let has_head = run_git(&root, &["rev-parse", "--verify", "HEAD"]).is_ok();
    match (has_head, path) {
        (true, Some(p)) => {
            run_git(&root, &["reset", "HEAD", "--", p])?;
        }
        (true, None) => {
            run_git(&root, &["reset", "HEAD"])?;
        }
        (false, Some(p)) => {
            run_git(&root, &["rm", "--cached", "-q", "--", p])?;
        }
        (false, None) => {
            run_git(&root, &["rm", "--cached", "-q", "-r", "."])?;
        }
    }
    Ok(())
}

/// Discards local unstaged working-tree edits (or deletes an untracked file).
pub fn git_discard_file(dir: &Path, file_path: &str, is_untracked: bool) -> Result<()> {
    let root = repo_root_or(dir)?;
    if is_untracked {
        let abs = root.join(file_path);
        if abs.is_dir() {
            std::fs::remove_dir_all(&abs)?;
        } else if abs.exists() {
            std::fs::remove_file(&abs)?;
        }
    } else {
        run_git(&root, &["checkout", "--", file_path])?;
    }
    Ok(())
}

/// Commits staged changes (or auto-stages all if `stage_all_if_none` is true and index is empty).
pub fn git_commit(dir: &Path, message: &str, stage_all_if_none: bool) -> Result<String> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("Commit message cannot be empty"));
    }
    let root = repo_root_or(dir)?;
    let snap = git_status(&root)?;
    if snap.staged.is_empty() {
        if stage_all_if_none && !snap.unstaged.is_empty() {
            git_stage(&root, None)?;
        } else {
            return Err(anyhow!("No staged changes to commit"));
        }
    }
    let out = run_git(&root, &["commit", "-m", trimmed])?;
    Ok(out.trim().to_string())
}

/// Switches HEAD to an existing local branch.
pub fn git_checkout_branch(dir: &Path, branch: &str) -> Result<String> {
    let b = branch.trim();
    if b.is_empty() {
        return Err(anyhow!("Branch name cannot be empty"));
    }
    let root = repo_root_or(dir)?;
    let out = run_git(&root, &["checkout", b])?;
    Ok(out.trim().to_string())
}

/// Creates a new branch at HEAD and checks it out immediately.
pub fn git_create_branch(dir: &Path, branch: &str) -> Result<String> {
    let b = branch.trim();
    if b.is_empty() {
        return Err(anyhow!("New branch name cannot be empty"));
    }
    let root = repo_root_or(dir)?;
    let out = run_git(&root, &["checkout", "-b", b])?;
    Ok(out.trim().to_string())
}

/// Pulls updates from the tracking upstream remote (`git pull`).
pub fn git_pull(dir: &Path) -> Result<String> {
    let root = repo_root_or(dir)?;
    let out = run_git(&root, &["pull"])?;
    Ok(out.trim().to_string())
}

/// Pushes the current branch to its upstream remote, falling back to `origin <branch>` if needed.
pub fn git_push(dir: &Path) -> Result<String> {
    let root = repo_root_or(dir)?;
    match run_git(&root, &["push"]) {
        Ok(out) => Ok(if out.trim().is_empty() {
            "Pushed successfully".to_string()
        } else {
            out.trim().to_string()
        }),
        Err(err) => {
            let msg = format!("{err:#}");
            if msg.contains("no upstream branch") || msg.contains("has no upstream") {
                let snap = git_status(&root)?;
                if snap.has_remote && !snap.branch.is_empty() && snap.branch != "HEAD" {
                    let out = run_git(&root, &["push", "-u", "origin", &snap.branch])?;
                    return Ok(out.trim().to_string());
                }
            }
            Err(anyhow!("{msg}"))
        }
    }
}

fn repo_root_or(dir: &Path) -> Result<PathBuf> {
    let out = run_git(dir, &["rev-parse", "--show-toplevel"])?;
    Ok(PathBuf::from(out.trim()))
}
