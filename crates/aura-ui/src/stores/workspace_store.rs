//! Workspace Domain Store: Manages collections, folders, and request files.

use anyhow::Result;
use aura_core::model::{AuraRequest, Collection, HttpMethod};
use aura_sync::workspace_meta::WorkspaceMetadata;
use aura_sync::toml_to_request;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Node in the hierarchical workspace sidebar tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceTreeNode {
    Folder {
        name: String,
        path: PathBuf,
        is_expanded: bool,
        children: Vec<WorkspaceTreeNode>,
    },
    Request {
        name: String,
        path: PathBuf,
        method: HttpMethod,
    },
}

impl WorkspaceTreeNode {
    pub fn name(&self) -> &str {
        match self {
            WorkspaceTreeNode::Folder { name, .. } => name.as_str(),
            WorkspaceTreeNode::Request { name, .. } => name.as_str(),
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            WorkspaceTreeNode::Folder { path, .. } => path.as_path(),
            WorkspaceTreeNode::Request { path, .. } => path.as_path(),
        }
    }

    pub fn is_folder(&self) -> bool {
        matches!(self, WorkspaceTreeNode::Folder { .. })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceStore {
    pub collections: Vec<Collection>,
    pub workspace_root: Option<PathBuf>,
    pub metadata: Option<WorkspaceMetadata>,
    pub tree: Vec<WorkspaceTreeNode>,
}

impl WorkspaceStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the active workspace with root, metadata, and tree.
    pub fn set_workspace(
        &mut self,
        root: PathBuf,
        metadata: WorkspaceMetadata,
        tree: Vec<WorkspaceTreeNode>,
    ) {
        self.workspace_root = Some(root);
        self.metadata = Some(metadata);
        self.tree = tree;
    }

    /// Applies external sync updates while preserving folder expansion state.
    pub fn apply_external_sync(
        &mut self,
        metadata: WorkspaceMetadata,
        mut new_tree: Vec<WorkspaceTreeNode>,
    ) {
        let mut expansion_states = HashMap::new();
        collect_expansion_states(&self.tree, &mut expansion_states);
        restore_expansion_states(&mut new_tree, &expansion_states);

        self.metadata = Some(metadata);
        self.tree = new_tree;
    }

    /// Toggles folder expanded/collapsed state.
    pub fn toggle_folder_expanded(&mut self, path: &Path) {
        toggle_folder_recursive(&mut self.tree, path);
    }

    /// Renames a node and updates descendant paths if it is a directory.
    pub fn apply_rename(&mut self, old_path: &Path, new_path: &Path, new_name: &str) {
        rename_node_recursive(&mut self.tree, old_path, new_path, new_name);
        if let (Some(root), Some(meta)) = (&self.workspace_root, &mut self.metadata) {
            if let Ok(rel) = old_path.strip_prefix(root) {
                let _ = meta.rename_item(rel, new_name);
            }
        }
    }

    /// Deletes a node by path.
    pub fn apply_delete(&mut self, target_path: &Path) {
        delete_node_recursive(&mut self.tree, target_path);
        if let (Some(root), Some(meta)) = (&self.workspace_root, &mut self.metadata) {
            if let Ok(rel) = target_path.strip_prefix(root) {
                let _ = meta.remove_item(rel);
            }
        }
    }

    /// Reorders a node into a target parent container at an insert index.
    pub fn apply_reorder(&mut self, source_path: &Path, target_parent: &Path, insert_index: usize) {
        if let Some(removed_node) = extract_node_recursive(&mut self.tree, source_path) {
            insert_node_recursive(&mut self.tree, target_parent, insert_index, removed_node);
        }
        if let (Some(root), Some(meta)) = (&self.workspace_root, &mut self.metadata) {
            if let (Ok(source_rel), Ok(target_rel)) = (
                source_path.strip_prefix(root),
                target_parent.strip_prefix(root),
            ) {
                let _ = meta.move_item(source_rel, target_rel, insert_index);
            }
        }
    }

    /// Load collections from a directory (legacy support)
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
                    let is_req_file = sub_path.is_file()
                        && (sub_path.extension().and_then(|s| s.to_str()) == Some("aura")
                            || sub_path.extension().and_then(|s| s.to_str()) == Some("toml"));
                    if is_req_file {
                        if let Ok(content) = fs::read_to_string(&sub_path) {
                            if let Ok(req) = aura_parser::parse_request(&content) {
                                req_files.push(req);
                            } else if let Ok(req) = toml_to_request(&content, None) {
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

fn collect_expansion_states(nodes: &[WorkspaceTreeNode], map: &mut HashMap<PathBuf, bool>) {
    for node in nodes {
        if let WorkspaceTreeNode::Folder { path, is_expanded, children, .. } = node {
            map.insert(path.clone(), *is_expanded);
            collect_expansion_states(children, map);
        }
    }
}

fn restore_expansion_states(nodes: &mut [WorkspaceTreeNode], map: &HashMap<PathBuf, bool>) {
    for node in nodes {
        if let WorkspaceTreeNode::Folder { path, is_expanded, children, .. } = node {
            if let Some(&expanded) = map.get(path) {
                *is_expanded = expanded;
            }
            restore_expansion_states(children, map);
        }
    }
}

fn toggle_folder_recursive(nodes: &mut [WorkspaceTreeNode], target: &Path) -> bool {
    for node in nodes {
        if let WorkspaceTreeNode::Folder { path, is_expanded, children, .. } = node {
            if path == target {
                *is_expanded = !*is_expanded;
                return true;
            }
            if toggle_folder_recursive(children, target) {
                return true;
            }
        }
    }
    false
}

fn rename_node_recursive(
    nodes: &mut [WorkspaceTreeNode],
    old_path: &Path,
    new_path: &Path,
    new_name: &str,
) -> bool {
    for node in nodes {
        match node {
            WorkspaceTreeNode::Folder { name, path, children, .. } => {
                if path == old_path {
                    *name = new_name.to_string();
                    *path = new_path.to_path_buf();
                    update_descendant_paths(children, old_path, new_path);
                    return true;
                }
                if rename_node_recursive(children, old_path, new_path, new_name) {
                    return true;
                }
            }
            WorkspaceTreeNode::Request { name, path, .. } => {
                if path == old_path {
                    *name = new_name.to_string();
                    *path = new_path.to_path_buf();
                    return true;
                }
            }
        }
    }
    false
}

fn update_descendant_paths(nodes: &mut [WorkspaceTreeNode], old_prefix: &Path, new_prefix: &Path) {
    for node in nodes {
        match node {
            WorkspaceTreeNode::Folder { path, children, .. } => {
                if let Ok(suffix) = path.strip_prefix(old_prefix) {
                    *path = new_prefix.join(suffix);
                }
                update_descendant_paths(children, old_prefix, new_prefix);
            }
            WorkspaceTreeNode::Request { path, .. } => {
                if let Ok(suffix) = path.strip_prefix(old_prefix) {
                    *path = new_prefix.join(suffix);
                }
            }
        }
    }
}

fn delete_node_recursive(nodes: &mut Vec<WorkspaceTreeNode>, target: &Path) -> bool {
    if let Some(pos) = nodes.iter().position(|n| n.path() == target) {
        nodes.remove(pos);
        return true;
    }
    for node in nodes {
        if let WorkspaceTreeNode::Folder { children, .. } = node {
            if delete_node_recursive(children, target) {
                return true;
            }
        }
    }
    false
}

fn extract_node_recursive(nodes: &mut Vec<WorkspaceTreeNode>, target: &Path) -> Option<WorkspaceTreeNode> {
    if let Some(pos) = nodes.iter().position(|n| n.path() == target) {
        return Some(nodes.remove(pos));
    }
    for node in nodes {
        if let WorkspaceTreeNode::Folder { children, .. } = node {
            if let Some(found) = extract_node_recursive(children, target) {
                return Some(found);
            }
        }
    }
    None
}

fn insert_node_recursive(
    nodes: &mut Vec<WorkspaceTreeNode>,
    target_parent: &Path,
    insert_index: usize,
    node_to_insert: WorkspaceTreeNode,
) -> bool {
    // If target parent is empty or matches root
    if target_parent.as_os_str().is_empty() {
        let idx = insert_index.min(nodes.len());
        nodes.insert(idx, node_to_insert);
        return true;
    }

    for node in nodes.iter_mut() {
        if let WorkspaceTreeNode::Folder { path, children, .. } = node {
            if path == target_parent {
                let idx = insert_index.min(children.len());
                children.insert(idx, node_to_insert);
                return true;
            }
            if insert_node_recursive(children, target_parent, insert_index, node_to_insert.clone()) {
                return true;
            }
        }
    }

    false
}
