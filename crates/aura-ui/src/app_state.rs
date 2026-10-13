//! Main application state for UI coordinating Domain Stores and Services.

use crate::keybindings::{Action, KeybindingPreset};
use crate::stores::{EditorTab, LayoutOrientation, ResponseTab, WorkspaceStore, WorkspaceTab};
use aura_core::model::{AuraRequest, Collection, Folder, HttpMethod};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Drag and drop payload for sidebar items
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DragItemPayload {
    pub path: PathBuf,
    pub is_folder: bool,
}

/// State for active inline renaming in the sidebar
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineRenameState {
    pub path: PathBuf,
    pub current_value: String,
}

/// Command palette entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPaletteItem {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub method: Option<HttpMethod>,
}

/// Main application state for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuraAppState {
    pub tabs: Vec<WorkspaceTab>,
    pub active_tab_index: usize,
    pub split_ratio: f32, // 0.1 to 0.9 (default 0.5)
    pub orientation: LayoutOrientation,
    pub active_editor_tab: EditorTab,
    pub active_response_tab: ResponseTab,
    pub is_code_mode: bool, // Dual-mode: Visual vs Code Editor (TOML)
    pub is_command_palette_open: bool,
    pub is_log_viewer_open: bool,
    pub log_filter_level: Option<aura_engine::LogLevel>,
    pub log_filter_subsystem: Option<String>,
    pub log_search: String,
    pub command_palette_search: String,
    pub selected_palette_index: usize,
    pub collections: Vec<Collection>,
    pub is_dragging_divider: bool,
    pub keybinding_preset: KeybindingPreset,
    pub status_message: Option<String>,
    pub workspace_store: WorkspaceStore,
    pub inline_rename: Option<InlineRenameState>,
    pub drag_target: Option<PathBuf>,
    pub pending_delete_path: Option<PathBuf>,
    #[serde(skip, default = "default_workspace_service")]
    pub workspace_service: std::sync::Arc<crate::services::WorkspaceService>,
}

fn default_workspace_service() -> std::sync::Arc<crate::services::WorkspaceService> {
    std::sync::Arc::new(crate::services::WorkspaceService::new())
}

impl Default for AuraAppState {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab_index: 0,
            split_ratio: 0.5,
            orientation: LayoutOrientation::Horizontal,
            active_editor_tab: EditorTab::Params,
            active_response_tab: ResponseTab::Json,
            is_code_mode: false,
            is_command_palette_open: false,
            is_log_viewer_open: false,
            log_filter_level: None,
            log_filter_subsystem: None,
            log_search: String::new(),
            command_palette_search: String::new(),
            selected_palette_index: 0,
            collections: Vec::new(),
            is_dragging_divider: false,
            keybinding_preset: KeybindingPreset::VsCode,
            status_message: None,
            workspace_store: WorkspaceStore::new(),
            inline_rename: None,
            drag_target: None,
            pending_delete_path: None,
            workspace_service: default_workspace_service(),
        }
    }
}

impl AuraAppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request_delete(&mut self, path: PathBuf) {
        self.pending_delete_path = Some(path);
    }

    pub fn cancel_delete(&mut self) {
        self.pending_delete_path = None;
    }

    pub async fn confirm_delete(&mut self) -> anyhow::Result<()> {
        if let Some(target_path) = self.pending_delete_path.take() {
            if let Some(root) = self.workspace_store.workspace_root.clone() {
                self.workspace_service.delete_item(&root, &target_path).await?;
                self.workspace_store.apply_delete(&target_path);
            } else if target_path.exists() {
                if target_path.is_dir() {
                    tokio::fs::remove_dir_all(&target_path).await?;
                } else {
                    tokio::fs::remove_file(&target_path).await?;
                }
                self.workspace_store.apply_delete(&target_path);
            }

            // Close matching or descendant open tabs
            let mut ids_to_close = Vec::new();
            for tab in &self.tabs {
                if let Some(ref fp) = tab.file_path {
                    if fp == &target_path || fp.starts_with(&target_path) {
                        ids_to_close.push(tab.id.clone());
                    }
                }
            }
            for id in ids_to_close {
                self.close_tab(&id);
            }
        }
        Ok(())
    }


    pub fn start_inline_rename(&mut self, path: PathBuf, initial_name: impl Into<String>) {
        self.inline_rename = Some(InlineRenameState {
            path,
            current_value: initial_name.into(),
        });
    }

    pub fn cancel_inline_rename(&mut self) {
        self.inline_rename = None;
    }

    /// Toggles folder expanded/collapsed state in the workspace store.
    pub fn toggle_folder(&mut self, path: &Path) {
        self.workspace_store.toggle_folder_expanded(path);
    }

    /// Opens a workspace directory, saving it to preferences and initializing the store tree.
    pub fn open_workspace_sync(&mut self, root: &Path) -> anyhow::Result<()> {
        let mut prefs = crate::preferences::AuraPreferences::load();
        prefs.last_workspace_path = Some(root.to_path_buf());
        let _ = prefs.save();

        let existing = aura_sync::workspace_meta::WorkspaceMetadata::load_from_disk(root).ok();
        let metadata = aura_sync::workspace_meta::WorkspaceMetadata::reconcile_with_disk(root, existing)?;
        let _ = metadata.save_to_disk(root);
        let tree = build_tree_from_metadata(root, &metadata.order)?;

        self.workspace_store.set_workspace(root.to_path_buf(), metadata, tree);
        Ok(())
    }

    /// Asynchronously opens a workspace directory.
    pub async fn open_workspace_directory(&mut self, root: &Path) -> anyhow::Result<()> {
        self.open_workspace_sync(root)
    }

    /// Opens a file in a tab, parsing request or replacing an unmodified untitled tab.
    pub fn open_file_in_tab(&mut self, path: &Path) -> anyhow::Result<()> {
        if let Some(pos) = self.tabs.iter().position(|t| t.file_path.as_deref() == Some(path)) {
            self.select_tab(pos);
            return Ok(());
        }

        let content = std::fs::read_to_string(path).unwrap_or_default();
        let file_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled");
        let req = if content.trim().is_empty() {
            AuraRequest::new(file_stem, HttpMethod::GET, "https://postman-echo.com/get")
        } else if let Ok(r) = aura_parser::parse_request(&content) {
            r
        } else {
            match aura_sync::toml_to_request(&content, None) {
                Ok(r) => r,
                Err(_) => AuraRequest::new(file_stem, HttpMethod::GET, "https://postman-echo.com/get"),
            }
        };

        for tab in &mut self.tabs {
            tab.is_active = false;
        }

        let mut new_tab = WorkspaceTab::new(req);
        if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
            new_tab.title = fname.to_string();
        }
        new_tab.file_path = Some(path.to_path_buf());

        if self.tabs.len() == 1
            && self.tabs[0].file_path.is_none()
            && !self.tabs[0].is_dirty
            && self.tabs[0].title == "Untitled Request"
        {
            self.tabs[0] = new_tab;
            self.active_tab_index = 0;
        } else {
            self.tabs.push(new_tab);
            self.active_tab_index = self.tabs.len() - 1;
        }

        Ok(())
    }

    /// Synchronously reconciles workspace store with disk changes and prunes deleted open tabs.
    pub fn reconcile_workspace_sync(&mut self) -> anyhow::Result<()> {
        if let Some(root) = self.workspace_store.workspace_root.clone() {
            let existing = aura_sync::workspace_meta::WorkspaceMetadata::load_from_disk(&root).ok();
            let metadata = aura_sync::workspace_meta::WorkspaceMetadata::reconcile_with_disk(&root, existing)?;
            let _ = metadata.save_to_disk(&root);
            let tree = build_tree_from_metadata(&root, &metadata.order)?;
            self.workspace_store.apply_external_sync(metadata, tree);

            let mut ids_to_close = Vec::new();
            for tab in &self.tabs {
                if let Some(ref fp) = tab.file_path {
                    if !fp.exists() {
                        ids_to_close.push(tab.id.clone());
                    }
                }
            }
            for id in ids_to_close {
                self.close_tab(&id);
            }
        }
        Ok(())
    }

    /// Reconciles workspace store with disk changes and prunes deleted open tabs.
    pub async fn reconcile_workspace_with_disk(&mut self) -> anyhow::Result<()> {
        self.reconcile_workspace_sync()
    }

    /// Discover and load collections from workspace directory
    pub fn load_workspace_collections(&mut self, dir: &Path) -> anyhow::Result<usize> {
        let mut loaded = 0;
        if !dir.exists() {
            return Ok(0);
        }
        let mut paths_to_check = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    paths_to_check.push(p);
                } else if p.is_dir() {
                    if let Ok(sub_entries) = std::fs::read_dir(&p) {
                        for sub in sub_entries.flatten() {
                            let sub_p = sub.path();
                            if sub_p.is_file() {
                                paths_to_check.push(sub_p);
                            }
                        }
                    }
                }
            }
        }

        for p in paths_to_check {
            let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if fname.ends_with(".collection.json")
                || fname.ends_with(".collection.toml")
                || fname.ends_with(".aura.json")
                || fname.ends_with(".aura.toml")
            {
                if let Ok(content) = std::fs::read_to_string(&p) {
                    if fname.ends_with(".json") {
                        if let Ok(col) = serde_json::from_str::<Collection>(&content) {
                            if !self.collections.iter().any(|c| c.id == col.id) {
                                self.collections.push(col);
                                loaded += 1;
                            }
                        }
                    } else if fname.ends_with(".toml") {
                        if let Ok(col) = toml::from_str::<Collection>(&content) {
                            if !self.collections.iter().any(|c| c.id == col.id) {
                                self.collections.push(col);
                                loaded += 1;
                            }
                        }
                    }
                }
            }
        }
        Ok(loaded)
    }

    /// Open request by collection and request IDs
    pub fn open_collection_request(&mut self, collection_id: &str, request_id: &str) -> bool {
        for col in &self.collections {
            if col.id == collection_id {
                if let Some(req) = col.requests.iter().find(|r| r.id == request_id) {
                    self.open_request(req.clone());
                    return true;
                }
                fn find_in_folders<'a>(
                    folders: &'a [Folder],
                    req_id: &str,
                ) -> Option<&'a AuraRequest> {
                    for f in folders {
                        if let Some(req) = f.requests.iter().find(|r| r.id == req_id) {
                            return Some(req);
                        }
                        if let Some(req) = find_in_folders(&f.subfolders, req_id) {
                            return Some(req);
                        }
                    }
                    None
                }
                if let Some(req) = find_in_folders(&col.folders, request_id) {
                    self.open_request(req.clone());
                    return true;
                }
            }
        }
        false
    }

    /// Open or focus a tab for a request
    pub fn open_request(&mut self, req: AuraRequest) {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == req.id) {
            self.select_tab(pos);
        } else {
            for t in &mut self.tabs {
                t.is_active = false;
            }
            let new_tab = WorkspaceTab::new(req);
            self.tabs.push(new_tab);
            self.active_tab_index = self.tabs.len() - 1;
        }
    }

    /// Persistent `+` action: Append new untitled request
    pub fn new_request(&mut self) {
        let req = AuraRequest::new(
            "Untitled Request",
            HttpMethod::GET,
            "https://postman-echo.com/get",
        );
        self.open_request(req);
    }

    /// Switch active tab by index
    pub fn select_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            for t in &mut self.tabs {
                t.is_active = false;
            }
            self.tabs[index].is_active = true;
            self.active_tab_index = index;
        }
    }

    /// Navigate to the next tab in circular order
    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            let next_idx = (self.active_tab_index + 1) % self.tabs.len();
            self.select_tab(next_idx);
        }
    }

    /// Navigate to the previous tab in circular order
    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            let prev_idx = if self.active_tab_index == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab_index - 1
            };
            self.select_tab(prev_idx);
        }
    }

    /// Close tab by ID
    pub fn close_tab(&mut self, id: &str) {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            self.tabs.remove(pos);
            if self.tabs.is_empty() {
                self.new_request();
            } else if self.active_tab_index >= self.tabs.len() {
                self.active_tab_index = self.tabs.len() - 1;
                self.tabs[self.active_tab_index].is_active = true;
            } else {
                self.tabs[self.active_tab_index].is_active = true;
            }
        }
    }

    /// Close the active tab
    pub fn close_active_tab(&mut self) {
        if !self.tabs.is_empty() && self.active_tab_index < self.tabs.len() {
            let id = self.tabs[self.active_tab_index].id.clone();
            self.close_tab(&id);
        }
    }

    /// Mark active tab as dirty or clean
    pub fn mark_active_tab_dirty(&mut self, is_dirty: bool) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.mark_dirty(is_dirty);
        }
    }

    /// Mark specific tab by ID as dirty or clean
    pub fn mark_tab_dirty(&mut self, id: &str, is_dirty: bool) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.mark_dirty(is_dirty);
        }
    }

    /// Save the active request and clear dirty indicator
    pub fn save_active_tab(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.mark_dirty(false);
            self.status_message = Some(format!("Saved: {}", tab.title));
        }
    }

    /// Adjust pane divider percentage (clamped between 0.1 and 0.9)
    pub fn set_split_ratio(&mut self, ratio: f32) {
        self.split_ratio = ratio.clamp(0.1, 0.9);
    }

    /// Start dragging divider
    pub fn start_dragging_divider(&mut self) {
        self.is_dragging_divider = true;
    }

    /// Stop dragging divider
    pub fn stop_dragging_divider(&mut self) {
        self.is_dragging_divider = false;
    }

    /// Adjust pane divider by delta ratio (clamped between 0.15 and 0.85)
    pub fn drag_divider(&mut self, delta_ratio: f32) {
        self.split_ratio = (self.split_ratio + delta_ratio).clamp(0.15, 0.85);
    }

    /// Reset pane divider to 50/50 split
    pub fn reset_divider(&mut self) {
        self.split_ratio = 0.50;
    }

    /// Toggle layout between Horizontal and Vertical
    pub fn toggle_orientation(&mut self) {
        self.orientation = match self.orientation {
            LayoutOrientation::Horizontal => LayoutOrientation::Vertical,
            LayoutOrientation::Vertical => LayoutOrientation::Horizontal,
        };
    }

    /// Open command palette modal
    pub fn open_command_palette(&mut self) {
        self.is_command_palette_open = true;
        self.selected_palette_index = 0;
    }

    /// Close command palette modal
    pub fn close_command_palette(&mut self) {
        self.is_command_palette_open = false;
        self.command_palette_search.clear();
        self.selected_palette_index = 0;
    }

    /// Toggle command palette modal
    pub fn toggle_command_palette(&mut self) {
        if self.is_command_palette_open {
            self.close_command_palette();
        } else {
            self.open_command_palette();
        }
    }

    /// Toggle log viewer modal/view
    pub fn toggle_log_viewer(&mut self) {
        self.is_log_viewer_open = !self.is_log_viewer_open;
        if self.is_log_viewer_open {
            tracing::info!(target: "aura_ui", "System log viewer opened");
        }
    }

    /// Set the minimum log level filter for the log viewer
    pub fn set_log_filter_level(&mut self, level: Option<aura_engine::LogLevel>) {
        self.log_filter_level = level;
    }

    /// Set the subsystem filter for the log viewer
    pub fn set_log_filter_subsystem(&mut self, subsystem: Option<String>) {
        self.log_filter_subsystem = subsystem;
    }

    /// Set the search query for the log viewer
    pub fn set_log_search(&mut self, query: impl Into<String>) {
        self.log_search = query.into();
    }

    /// Retrieve filtered log entries from the global broadcaster
    pub fn get_filtered_logs(&self) -> Vec<aura_engine::LogEntry> {
        aura_engine::LogBroadcaster::global().filter_entries(
            self.log_filter_level,
            self.log_filter_subsystem.as_deref(),
            if self.log_search.is_empty() {
                None
            } else {
                Some(&self.log_search)
            },
        )
    }

    /// Clear all logs in the ring buffer
    pub fn clear_logs(&mut self) {
        aura_engine::LogBroadcaster::global().clear();
        tracing::info!(target: "aura_ui", "System logs cleared from ring buffer");
    }

    /// Set active keybinding preset
    pub fn set_keybinding_preset(&mut self, preset: KeybindingPreset) {
        self.keybinding_preset = preset;
    }

    /// Filter items for command palette
    pub fn filtered_palette_items(&self) -> Vec<CommandPaletteItem> {
        let q = self.command_palette_search.to_lowercase();
        self.tabs
            .iter()
            .filter(|t| {
                q.is_empty()
                    || t.title.to_lowercase().contains(&q)
                    || t.request.url.to_lowercase().contains(&q)
            })
            .map(|t| CommandPaletteItem {
                id: t.id.clone(),
                title: t.title.clone(),
                subtitle: t.request.url.clone(),
                method: Some(t.method),
            })
            .collect()
    }

    /// Move down in command palette list
    pub fn palette_select_next(&mut self) {
        let items = self.filtered_palette_items();
        if !items.is_empty() {
            self.selected_palette_index = (self.selected_palette_index + 1) % items.len();
        }
    }

    /// Move up in command palette list
    pub fn palette_select_prev(&mut self) {
        let items = self.filtered_palette_items();
        if !items.is_empty() {
            self.selected_palette_index = if self.selected_palette_index == 0 {
                items.len() - 1
            } else {
                self.selected_palette_index - 1
            };
        }
    }

    /// Execute selected item in command palette
    pub fn execute_selected_palette_item(&mut self) -> Option<String> {
        let items = self.filtered_palette_items();
        if let Some(item) = items.get(self.selected_palette_index) {
            let id = item.id.clone();
            if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
                self.select_tab(pos);
            }
            self.close_command_palette();
            Some(id)
        } else {
            None
        }
    }

    /// Handle keyboard action directly
    pub fn handle_key_action(&mut self, action: Action) -> bool {
        match action {
            Action::SendRequest => {
                self.status_message = Some("Dispatching request...".to_string());
                true
            }
            Action::OpenCommandPalette => {
                self.open_command_palette();
                true
            }
            Action::CloseCommandPalette => {
                if self.is_command_palette_open {
                    self.close_command_palette();
                    true
                } else {
                    false
                }
            }
            Action::ToggleLogViewer => {
                self.toggle_log_viewer();
                true
            }
            Action::NewRequest => {
                self.new_request();
                true
            }
            Action::CloseActiveTab => {
                self.close_active_tab();
                true
            }
            Action::ToggleLayout => {
                self.toggle_orientation();
                true
            }
            Action::ResetDivider => {
                self.reset_divider();
                true
            }
            Action::SaveRequest => {
                self.save_active_tab();
                true
            }
            Action::NextTab => {
                self.next_tab();
                true
            }
            Action::PrevTab => {
                self.prev_tab();
                true
            }
        }
    }
}

fn build_tree_from_metadata(
    dir: &Path,
    order: &[aura_sync::workspace_meta::WorkspaceOrderItem],
) -> anyhow::Result<Vec<crate::stores::workspace_store::WorkspaceTreeNode>> {
    use aura_sync::workspace_meta::WorkspaceOrderItem;
    use crate::stores::workspace_store::WorkspaceTreeNode;

    let mut tree = Vec::new();
    for item in order {
        match item {
            WorkspaceOrderItem::Folder { name, order } => {
                let folder_path = dir.join(name);
                let children = build_tree_from_metadata(&folder_path, order)?;
                tree.push(WorkspaceTreeNode::Folder {
                    name: name.clone(),
                    path: folder_path,
                    is_expanded: true,
                    children,
                });
            }
            WorkspaceOrderItem::Request { name } => {
                let req_path = dir.join(name);
                let method = if let Ok(content) = std::fs::read_to_string(&req_path) {
                    if let Ok(r) = aura_parser::parse_request(&content) {
                        r.method
                    } else if let Ok(req) = aura_sync::toml_to_request(&content, None) {
                        req.method
                    } else {
                        HttpMethod::GET
                    }
                } else {
                    HttpMethod::GET
                };
                tree.push(WorkspaceTreeNode::Request {
                    name: name.clone(),
                    path: req_path,
                    method,
                });
            }
        }
    }
    Ok(tree)
}
