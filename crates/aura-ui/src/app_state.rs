//! Main application state for UI coordinating Domain Stores and Services.

use crate::keybindings::{Action, KeybindingPreset};
use crate::stores::{EditorTab, LayoutOrientation, ResponseTab, WorkspaceTab};
use aura_core::model::{AuraRequest, Collection, Folder, HttpMethod};
use serde::{Deserialize, Serialize};
use std::path::Path;

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
    pub command_palette_search: String,
    pub selected_palette_index: usize,
    pub collections: Vec<Collection>,
    pub is_dragging_divider: bool,
    pub keybinding_preset: KeybindingPreset,
    pub status_message: Option<String>,
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
            command_palette_search: String::new(),
            selected_palette_index: 0,
            collections: Vec::new(),
            is_dragging_divider: false,
            keybinding_preset: KeybindingPreset::VsCode,
            status_message: None,
        }
    }
}

impl AuraAppState {
    pub fn new() -> Self {
        Self::default()
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
