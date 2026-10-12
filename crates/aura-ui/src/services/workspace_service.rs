//! Workspace Service: Handles background directory traversal, file watching, and autosave.

use anyhow::{anyhow, Result};
use aura_core::model::{AuraRequest, HttpMethod};
use aura_sync::workspace_meta::WorkspaceMetadata;
use aura_sync::request_to_toml;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use parking_lot::Mutex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

/// Disk event emitted by the background watcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceDiskEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceService {
    suppression_tokens: Arc<Mutex<HashSet<PathBuf>>>,
}

impl WorkspaceService {
    pub fn new() -> Self {
        Self {
            suppression_tokens: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Registers a path to be ignored by the file watcher for its next echo.
    pub fn suppress_path(&self, path: &Path) {
        self.suppression_tokens.lock().insert(path.to_path_buf());
    }

    /// Checks and consumes a suppression token for the given path.
    pub fn is_suppressed(&self, path: &Path) -> bool {
        self.suppression_tokens.lock().remove(path)
    }

    /// Native folder picker dialog.
    pub async fn pick_folder() -> Option<PathBuf> {
        let handle = rfd::AsyncFileDialog::new().pick_folder().await;
        handle.map(|h| h.path().to_path_buf())
    }

    /// Asynchronously creates a new request `.aura` file and updates `workspace.json`.
    pub async fn create_request(
        &self,
        root: &Path,
        parent_dir: &Path,
        name: &str,
        method: HttpMethod,
    ) -> Result<PathBuf> {
        tokio::fs::create_dir_all(parent_dir).await?;

        let file_name = if name.ends_with(".aura") {
            name.to_string()
        } else {
            format!("{}.aura", name)
        };

        let file_path = parent_dir.join(&file_name);
        let req_name = file_name.trim_end_matches(".aura");
        let req = AuraRequest::new(req_name, method, "https://postman-echo.com/get");
        let toml_str = request_to_toml(&req)?;

        self.suppress_path(&file_path);
        tokio::fs::write(&file_path, toml_str).await?;

        self.sync_meta_file(root).await?;

        tracing::info!(target: "aura_ui::workspace", path = %file_path.display(), "Created new request file");
        Ok(file_path)
    }

    /// Asynchronously creates a folder and updates `workspace.json`.
    pub async fn create_folder(
        &self,
        root: &Path,
        parent_dir: &Path,
        name: &str,
    ) -> Result<PathBuf> {
        let folder_path = parent_dir.join(name);
        self.suppress_path(&folder_path);
        tokio::fs::create_dir_all(&folder_path).await?;

        self.sync_meta_file(root).await?;

        tracing::info!(target: "aura_ui::workspace", path = %folder_path.display(), "Created workspace folder");
        Ok(folder_path)
    }

    /// Asynchronously renames a request file or folder and updates `workspace.json`.
    pub async fn rename_item(
        &self,
        root: &Path,
        old_path: &Path,
        new_name: &str,
    ) -> Result<PathBuf> {
        let parent = old_path.parent().unwrap_or(root);
        let final_name = if old_path.is_file() && !new_name.ends_with(".aura") {
            format!("{}.aura", new_name)
        } else {
            new_name.to_string()
        };

        let new_path = parent.join(&final_name);

        self.suppress_path(old_path);
        self.suppress_path(&new_path);
        tokio::fs::rename(old_path, &new_path).await?;

        self.sync_meta_file(root).await?;

        tracing::info!(
            target: "aura_ui::workspace",
            from = %old_path.display(),
            to = %new_path.display(),
            "Renamed workspace item"
        );
        Ok(new_path)
    }

    /// Asynchronously deletes a file or directory and updates `workspace.json`.
    pub async fn delete_item(&self, root: &Path, target_path: &Path) -> Result<()> {
        self.suppress_path(target_path);

        if target_path.is_dir() {
            tokio::fs::remove_dir_all(target_path).await?;
        } else if target_path.exists() {
            tokio::fs::remove_file(target_path).await?;
        }

        self.sync_meta_file(root).await?;

        tracing::info!(target: "aura_ui::workspace", path = %target_path.display(), "Deleted workspace item");
        Ok(())
    }

    /// Asynchronously moves/reorders an item.
    pub async fn reorder_item(
        &self,
        root: &Path,
        source_path: &Path,
        target_parent: &Path,
        insert_index: usize,
    ) -> Result<()> {
        let file_name = source_path
            .file_name()
            .ok_or_else(|| anyhow!("Invalid source path"))?;
        let target_path = target_parent.join(file_name);

        if source_path != target_path {
            self.suppress_path(source_path);
            self.suppress_path(&target_path);
            tokio::fs::rename(source_path, &target_path).await?;
        }

        let existing_meta = WorkspaceMetadata::load_from_disk(root).ok();
        let mut meta = WorkspaceMetadata::reconcile_with_disk(root, existing_meta)?;
        if let (Ok(source_rel), Ok(target_rel)) = (
            source_path.strip_prefix(root),
            target_parent.strip_prefix(root),
        ) {
            let _ = meta.move_item(source_rel, target_rel, insert_index);
        }
        meta.save_to_disk(root)?;

        Ok(())
    }

    /// Spawns a filesystem watcher notifying changes debounced over a channel.
    pub fn start_watcher(
        &self,
        root: &Path,
    ) -> Result<(tokio_util::sync::CancellationToken, UnboundedReceiver<WorkspaceDiskEvent>)> {
        let (tx, rx) = unbounded_channel();
        let suppression_tokens = Arc::clone(&self.suppression_tokens);
        let cancel_token = tokio_util::sync::CancellationToken::new();
        let cancel_for_thread = cancel_token.clone();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                for path in event.paths {
                    // Ignore workspace.json writes, hidden files, and build directories
                    if path.file_name().and_then(|s| s.to_str()) == Some("workspace.json")
                        || path.components().any(|c| {
                            let s = c.as_os_str().to_string_lossy();
                            s == "node_modules"
                                || s == "target"
                                || (s.starts_with('.') && s != "." && s != "..")
                        })
                    {
                        continue;
                    }

                    if suppression_tokens.lock().remove(&path) {
                        continue;
                    }

                    let disk_event = match event.kind {
                        EventKind::Create(_) => Some(WorkspaceDiskEvent::Created(path)),
                        EventKind::Modify(_) => Some(WorkspaceDiskEvent::Modified(path)),
                        EventKind::Remove(_) => Some(WorkspaceDiskEvent::Deleted(path)),
                        EventKind::Any => Some(WorkspaceDiskEvent::Modified(path)),
                        _ => Some(WorkspaceDiskEvent::Modified(path)),
                    };

                    if let Some(e) = disk_event {
                        let _ = tx.send(e);
                    }
                }
            }
        })?;

        watcher.watch(root, RecursiveMode::Recursive)?;

        // Detach watcher to background thread until cancelled
        std::thread::spawn(move || {
            let _keep_alive = watcher;
            while !cancel_for_thread.is_cancelled() {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        });

        Ok((cancel_token, rx))
    }

    /// Reconciles disk state and updates `workspace.json`.
    async fn sync_meta_file(&self, root: &Path) -> Result<()> {
        let existing = WorkspaceMetadata::load_from_disk(root).ok();
        let meta = WorkspaceMetadata::reconcile_with_disk(root, existing)?;
        meta.save_to_disk(root)?;
        Ok(())
    }

    /// Save a request asynchronously to disk (legacy support)
    pub async fn persist_request(
        &self,
        request: &AuraRequest,
        target_dir: &Path,
    ) -> Result<PathBuf> {
        let toml_str = request_to_toml(request)?;
        let sanitized_name = request
            .name
            .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        let file_path = target_dir.join(format!("{}.aura", sanitized_name));
        self.suppress_path(&file_path);
        tokio::fs::write(&file_path, toml_str).await?;
        Ok(file_path)
    }
}
