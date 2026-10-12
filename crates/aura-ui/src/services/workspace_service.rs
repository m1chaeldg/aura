//! Workspace Service: Handles background directory traversal, file watching, and autosave.

use anyhow::Result;
use aura_core::model::AuraRequest;
use aura_sync::request_to_toml;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct WorkspaceService;

impl WorkspaceService {
    pub fn new() -> Self {
        Self
    }

    /// Save a request asynchronously to disk
    pub async fn persist_request(&self, request: &AuraRequest, target_dir: &Path) -> Result<PathBuf> {
        let toml_str = request_to_toml(request)?;
        let sanitized_name = request.name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        let file_path = target_dir.join(format!("{}.toml", sanitized_name));
        tokio::fs::write(&file_path, toml_str).await?;
        Ok(file_path)
    }
}
