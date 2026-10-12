//! Workspace Domain Store: Manages collections, folders, and request files.

use anyhow::Result;
use aura_core::model::{AuraRequest, Collection, HttpMethod};
use aura_sync::toml_to_request;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceStore {
    pub collections: Vec<Collection>,
    pub workspace_root: Option<PathBuf>,
}

impl WorkspaceStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load collections from a directory
    pub fn load_workspace_collections(&mut self, dir: &Path) -> Result<usize> {
        let mut count = 0;
        if !dir.exists() {
            return Ok(0);
        }

        self.workspace_root = Some(dir.to_path_buf());

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let col_name = path.file_name().unwrap().to_string_lossy().to_string();
                let mut col = Collection {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: col_name,
                    auth: aura_core::model::AuthConfig::Inherit,
                    variables: std::collections::HashMap::new(),
                    requests: Vec::new(),
                    folders: Vec::new(),
                };

                let mut req_files = Vec::new();
                for sub in fs::read_dir(&path)? {
                    let sub = sub?;
                    let sub_path = sub.path();
                    if sub_path.is_file()
                        && sub_path.extension().and_then(|s| s.to_str()) == Some("toml")
                    {
                        if let Ok(content) = fs::read_to_string(&sub_path) {
                            if let Ok(req) = toml_to_request(&content, None) {
                                req_files.push(req);
                            }
                        }
                    }
                }

                if req_files.is_empty() {
                    req_files.push(AuraRequest::new(
                        "Sample Request",
                        HttpMethod::GET,
                        "https://postman-echo.com/get",
                    ));
                }

                col.requests = req_files;
                count += col.requests.len();
                self.collections.push(col);
            }
        }

        Ok(count)
    }

    /// Flatten all requests across collections
    pub fn all_requests(&self) -> Vec<AuraRequest> {
        let mut reqs = Vec::new();
        for col in &self.collections {
            reqs.extend(col.requests.clone());
            for folder in &col.folders {
                reqs.extend(folder.requests.clone());
            }
        }
        reqs
    }

    pub fn add_collection(&mut self, collection: Collection) {
        self.collections.push(collection);
    }
}
