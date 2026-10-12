use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Item in the workspace hierarchy order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WorkspaceOrderItem {
    Folder {
        name: String,
        #[serde(default)]
        order: Vec<WorkspaceOrderItem>,
    },
    Request {
        name: String,
    },
}

impl WorkspaceOrderItem {
    pub fn name(&self) -> &str {
        match self {
            WorkspaceOrderItem::Folder { name, .. } => name.as_str(),
            WorkspaceOrderItem::Request { name } => name.as_str(),
        }
    }
}

/// Workspace metadata persisted in `workspace.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceMetadata {
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub order: Vec<WorkspaceOrderItem>,
}

impl WorkspaceMetadata {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            version: 1,
            name: name.into(),
            order: Vec::new(),
        }
    }

    /// Loads metadata from `workspace.json` in the workspace root.
    pub fn load_from_disk(root: &Path) -> Result<Self> {
        let meta_path = root.join("workspace.json");
        let content = fs::read_to_string(&meta_path)
            .with_context(|| format!("Failed to read {}", meta_path.display()))?;
        let meta: Self = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse {}", meta_path.display()))?;
        Ok(meta)
    }

    /// Saves metadata to `workspace.json` in the workspace root.
    pub fn save_to_disk(&self, root: &Path) -> Result<PathBuf> {
        if !root.exists() {
            fs::create_dir_all(root)?;
        }
        let meta_path = root.join("workspace.json");
        let json = serde_json::to_string_pretty(self)?;
        fs::write(&meta_path, json)?;
        Ok(meta_path)
    }

    /// Reconciles the metadata order with current files and directories on disk.
    pub fn reconcile_with_disk(root: &Path, existing: Option<WorkspaceMetadata>) -> Result<Self> {
        let workspace_name = match &existing {
            Some(m) if !m.name.is_empty() => m.name.clone(),
            _ => root
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Workspace".to_string()),
        };

        let mut meta = WorkspaceMetadata {
            version: 1,
            name: workspace_name,
            order: Vec::new(),
        };

        if !root.exists() || !root.is_dir() {
            return Ok(meta);
        }

        let existing_order = existing.map(|m| m.order).unwrap_or_default();
        meta.order = reconcile_directory(root, &existing_order)?;

        Ok(meta)
    }

    /// Reorders an item within a parent folder or root.
    pub fn reorder_item(
        &mut self,
        parent_rel_path: &Path,
        from_index: usize,
        to_index: usize,
    ) -> Result<()> {
        let list = find_order_list_mut(&mut self.order, parent_rel_path)?;
        if from_index >= list.len() || to_index >= list.len() {
            return Err(anyhow!("Index out of bounds for reordering"));
        }
        let item = list.remove(from_index);
        list.insert(to_index, item);
        Ok(())
    }

    /// Moves an item from source relative path to target parent container.
    pub fn move_item(
        &mut self,
        source_rel_path: &Path,
        target_parent_rel_path: &Path,
        insert_index: usize,
    ) -> Result<()> {
        let source_parent = source_rel_path.parent().unwrap_or(Path::new(""));
        let item_name = source_rel_path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("Invalid source relative path"))?;

        // 1. Remove from source parent
        let source_list = find_order_list_mut(&mut self.order, source_parent)?;
        let idx = source_list
            .iter()
            .position(|i| i.name() == item_name)
            .ok_or_else(|| anyhow!("Item '{}' not found at source", item_name))?;
        let item = source_list.remove(idx);

        // 2. Insert into target parent
        let target_list = find_order_list_mut(&mut self.order, target_parent_rel_path)?;
        let clamped_idx = insert_index.min(target_list.len());
        target_list.insert(clamped_idx, item);

        Ok(())
    }

    /// Renames an item matching the relative path.
    pub fn rename_item(&mut self, rel_path: &Path, new_name: &str) -> Result<()> {
        let parent = rel_path.parent().unwrap_or(Path::new(""));
        let old_name = rel_path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("Invalid relative path"))?;

        let list = find_order_list_mut(&mut self.order, parent)?;
        let item = list
            .iter_mut()
            .find(|i| i.name() == old_name)
            .ok_or_else(|| anyhow!("Item '{}' not found", old_name))?;

        match item {
            WorkspaceOrderItem::Folder { name, .. } => *name = new_name.to_string(),
            WorkspaceOrderItem::Request { name } => *name = new_name.to_string(),
        }

        Ok(())
    }

    /// Removes an item matching the relative path.
    pub fn remove_item(&mut self, rel_path: &Path) -> Result<()> {
        let parent = rel_path.parent().unwrap_or(Path::new(""));
        let target_name = rel_path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("Invalid relative path"))?;

        let list = find_order_list_mut(&mut self.order, parent)?;
        list.retain(|i| i.name() != target_name);
        Ok(())
    }
}

fn reconcile_directory(dir: &Path, existing_order: &[WorkspaceOrderItem]) -> Result<Vec<WorkspaceOrderItem>> {
    let mut discovered_folders = Vec::new();
    let mut discovered_requests = Vec::new();

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(Vec::new()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = match entry.file_name().into_string() {
            Ok(name) => name,
            Err(_) => continue,
        };

        if file_name.starts_with('.') || file_name == "node_modules" || file_name == "target" {
            continue;
        }

        if path.is_dir() {
            discovered_folders.push((file_name, path));
        } else if path.is_file() && file_name.ends_with(".aura") {
            discovered_requests.push(file_name);
        }
    }

    let mut result = Vec::new();
    let mut seen_folders = HashSet::new();
    let mut seen_requests = HashSet::new();

    // 1. Reconcile existing items in their established order
    for item in existing_order {
        match item {
            WorkspaceOrderItem::Folder { name, order } => {
                if let Some((_, sub_path)) = discovered_folders.iter().find(|(n, _)| n == name) {
                    seen_folders.insert(name.clone());
                    let sub_order = reconcile_directory(sub_path, order)?;
                    result.push(WorkspaceOrderItem::Folder {
                        name: name.clone(),
                        order: sub_order,
                    });
                }
            }
            WorkspaceOrderItem::Request { name } => {
                if discovered_requests.contains(name) {
                    seen_requests.insert(name.clone());
                    result.push(WorkspaceOrderItem::Request { name: name.clone() });
                }
            }
        }
    }

    // 2. Append new folders
    for (name, sub_path) in discovered_folders {
        if !seen_folders.contains(&name) {
            let sub_order = reconcile_directory(&sub_path, &[])?;
            result.push(WorkspaceOrderItem::Folder {
                name,
                order: sub_order,
            });
        }
    }

    // 3. Append new requests
    for name in discovered_requests {
        if !seen_requests.contains(&name) {
            result.push(WorkspaceOrderItem::Request { name });
        }
    }

    Ok(result)
}

fn find_order_list_mut<'a>(
    order: &'a mut Vec<WorkspaceOrderItem>,
    rel_path: &Path,
) -> Result<&'a mut Vec<WorkspaceOrderItem>> {
    let mut current = order;
    for component in rel_path.components() {
        let seg = component.as_os_str().to_str().unwrap_or("");
        if seg.is_empty() || seg == "." {
            continue;
        }
        let folder = current
            .iter_mut()
            .find_map(|item| match item {
                WorkspaceOrderItem::Folder { name, order } if name == seg => Some(order),
                _ => None,
            })
            .ok_or_else(|| anyhow!("Folder component '{}' not found", seg))?;
        current = folder;
    }
    Ok(current)
}
