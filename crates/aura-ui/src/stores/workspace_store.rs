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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipboardPayload {
    Request(PathBuf),
    Folder(PathBuf),
}

impl ClipboardPayload {
    pub fn path(&self) -> &Path {
        match self {
            ClipboardPayload::Request(p) => p.as_path(),
            ClipboardPayload::Folder(p) => p.as_path(),
        }
    }

    pub fn is_folder(&self) -> bool {
        matches!(self, ClipboardPayload::Folder(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DragPlacement {
    Above,
    Inside,
    Below,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidebarDragState {
    pub dragging_path: PathBuf,
    pub hover_target_path: PathBuf,
    pub placement: DragPlacement,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextMenuState {
    pub target_path: PathBuf,
    pub is_folder: bool,
    pub position: (f32, f32),
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
    pub selected_path: Option<PathBuf>,
    pub clipboard_item: Option<ClipboardPayload>,
    pub active_context_menu: Option<ContextMenuState>,
    pub drag_state: Option<SidebarDragState>,
}

impl WorkspaceStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn visible_nodes(&self) -> Vec<&WorkspaceTreeNode> {
        let mut result = Vec::new();
        collect_visible_nodes(&self.tree, &mut result);
        result
    }

    pub fn select_node(&mut self, path: PathBuf) {
        self.selected_path = Some(path);
    }

    pub fn select_next_node(&mut self) {
        let visible = self.visible_nodes();
        if visible.is_empty() {
            return;
        }
        match &self.selected_path {
            None => {
                self.selected_path = Some(visible[0].path().to_path_buf());
            }
            Some(current) => {
                if let Some(pos) = visible.iter().position(|n| n.path() == current) {
                    let next_idx = (pos + 1).min(visible.len() - 1);
                    self.selected_path = Some(visible[next_idx].path().to_path_buf());
                } else {
                    self.selected_path = Some(visible[0].path().to_path_buf());
                }
            }
        }
    }

    pub fn select_prev_node(&mut self) {
        let visible = self.visible_nodes();
        if visible.is_empty() {
            return;
        }
        match &self.selected_path {
            None => {
                self.selected_path = Some(visible[0].path().to_path_buf());
            }
            Some(current) => {
                if let Some(pos) = visible.iter().position(|n| n.path() == current) {
                    let prev_idx = pos.saturating_sub(1);
                    self.selected_path = Some(visible[prev_idx].path().to_path_buf());
                } else {
                    self.selected_path = Some(visible[0].path().to_path_buf());
                }
            }
        }
    }

    pub fn expand_selected(&mut self) {
        if let Some(path) = self.selected_path.clone() {
            set_folder_expanded_recursive(&mut self.tree, &path, true);
        }
    }

    pub fn collapse_selected(&mut self) {
        if let Some(path) = self.selected_path.clone() {
            set_folder_expanded_recursive(&mut self.tree, &path, false);
        }
    }

    pub fn copy_selected(&mut self) {
        if let Some(selected) = &self.selected_path {
            if let Some(node) = find_node_recursive(&self.tree, selected) {
                match node {
                    WorkspaceTreeNode::Folder { path, .. } => {
                        self.clipboard_item = Some(ClipboardPayload::Folder(path.clone()));
                    }
                    WorkspaceTreeNode::Request { path, .. } => {
                        self.clipboard_item = Some(ClipboardPayload::Request(path.clone()));
                    }
                }
            } else if selected.extension().is_some() {
                self.clipboard_item = Some(ClipboardPayload::Request(selected.clone()));
            } else {
                self.clipboard_item = Some(ClipboardPayload::Folder(selected.clone()));
            }
        }
    }

    pub fn clear_clipboard(&mut self) {
        self.clipboard_item = None;
    }

    pub fn open_context_menu(&mut self, target_path: PathBuf, is_folder: bool, pos: (f32, f32)) {
        self.active_context_menu = Some(ContextMenuState {
            target_path,
            is_folder,
            position: pos,
        });
    }

    pub fn close_context_menu(&mut self) {
        self.active_context_menu = None;
    }

    pub fn set_drag_state(&mut self, state: Option<SidebarDragState>) {
        self.drag_state = state;
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

fn collect_visible_nodes<'a>(nodes: &'a [WorkspaceTreeNode], out: &mut Vec<&'a WorkspaceTreeNode>) {
    for node in nodes {
        out.push(node);
        if let WorkspaceTreeNode::Folder { is_expanded, children, .. } = node {
            if *is_expanded {
                collect_visible_nodes(children, out);
            }
        }
    }
}

fn find_node_recursive<'a>(nodes: &'a [WorkspaceTreeNode], target: &Path) -> Option<&'a WorkspaceTreeNode> {
    for node in nodes {
        if node.path() == target {
            return Some(node);
        }
        if let WorkspaceTreeNode::Folder { children, .. } = node {
            if let Some(found) = find_node_recursive(children, target) {
                return Some(found);
            }
        }
    }
    None
}

fn set_folder_expanded_recursive(nodes: &mut [WorkspaceTreeNode], target: &Path, expanded: bool) -> bool {
    for node in nodes {
        if let WorkspaceTreeNode::Folder { path, is_expanded, children, .. } = node {
            if path == target {
                *is_expanded = expanded;
                return true;
            }
            if set_folder_expanded_recursive(children, target, expanded) {
                return true;
            }
        }
    }
    false
}

