//! Session Domain Store: Tracks open document tabs, active selection, and dirty states.

use aura_core::model::{AuraRequest, AuraResponse, HttpMethod};
use serde::{Deserialize, Serialize};

use std::path::{Path, PathBuf};

/// Represents an open tab in the multi-tab workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceTab {
    pub id: String,
    pub title: String,
    pub method: HttpMethod,
    pub is_dirty: bool,
    pub is_active: bool,
    pub request: AuraRequest,
    pub response: Option<AuraResponse>,
    pub file_path: Option<PathBuf>,
}

impl WorkspaceTab {
    pub fn new(request: AuraRequest) -> Self {
        Self {
            id: request.id.clone(),
            title: request.name.clone(),
            method: request.method,
            is_dirty: false,
            is_active: true,
            request,
            response: None,
            file_path: None,
        }
    }

    pub fn badge_color(&self) -> &'static str {
        self.method.badge_color()
    }

    pub fn mark_dirty(&mut self, is_dirty: bool) {
        self.is_dirty = is_dirty;
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionStore {
    pub tabs: Vec<WorkspaceTab>,
    pub active_tab_index: usize,
}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open a new Untitled request tab
    pub fn new_request(&mut self) {
        let req = AuraRequest::new(
            "Untitled Request",
            HttpMethod::GET,
            "https://postman-echo.com/get",
        );
        self.open_request(req);
    }

    /// Open a request into a tab
    pub fn open_request(&mut self, request: AuraRequest) {
        for tab in &mut self.tabs {
            tab.is_active = false;
        }

        let new_tab = WorkspaceTab::new(request);
        self.tabs.push(new_tab);
        self.active_tab_index = self.tabs.len() - 1;
    }

    /// Open a request backed by a file path, activating existing tab if already open.
    pub fn open_request_file(&mut self, request: AuraRequest, path: PathBuf) {
        if let Some(pos) = self.tabs.iter().position(|t| t.file_path.as_ref() == Some(&path)) {
            self.select_tab(pos);
            return;
        }

        for tab in &mut self.tabs {
            tab.is_active = false;
        }

        let mut new_tab = WorkspaceTab::new(request);
        if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
            new_tab.title = file_name.to_string();
        }
        new_tab.file_path = Some(path);
        self.tabs.push(new_tab);
        self.active_tab_index = self.tabs.len() - 1;
    }

    /// Updates tab titles and paths when a file or directory is renamed.
    pub fn handle_item_renamed(&mut self, old_path: &Path, new_path: &Path, new_name: &str) {
        for tab in &mut self.tabs {
            if let Some(ref fp) = tab.file_path.clone() {
                if fp == old_path {
                    tab.file_path = Some(new_path.to_path_buf());
                    tab.title = new_name.to_string();
                } else if let Ok(suffix) = fp.strip_prefix(old_path) {
                    tab.file_path = Some(new_path.join(suffix));
                }
            }
        }
    }

    /// Closes tabs corresponding to a deleted file or folder.
    pub fn handle_item_deleted(&mut self, deleted_path: &Path) {
        let matching_ids: Vec<String> = self
            .tabs
            .iter()
            .filter(|t| {
                if let Some(ref fp) = t.file_path {
                    fp == deleted_path || fp.starts_with(deleted_path)
                } else {
                    false
                }
            })
            .map(|t| t.id.clone())
            .collect();

        for id in matching_ids {
            self.close_tab(&id);
        }
    }

    /// Select active tab by index
    pub fn select_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            for tab in &mut self.tabs {
                tab.is_active = false;
            }
            self.tabs[index].is_active = true;
            self.active_tab_index = index;
        }
    }

    /// Close tab by id with safe fallback
    pub fn close_tab(&mut self, tab_id: &str) {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == tab_id) {
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
        if !self.tabs.is_empty() {
            let id = self.tabs[self.active_tab_index].id.clone();
            self.close_tab(&id);
        }
    }

    /// Mark a specific tab dirty
    pub fn mark_tab_dirty(&mut self, tab_id: &str, is_dirty: bool) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.mark_dirty(is_dirty);
        }
    }

    /// Mark active tab dirty
    pub fn mark_active_tab_dirty(&mut self, is_dirty: bool) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.mark_dirty(is_dirty);
        }
    }

    /// Save active tab and clear dirty state
    pub fn save_active_tab(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.mark_dirty(false);
        }
    }

    /// Navigate to next tab
    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            let next = (self.active_tab_index + 1) % self.tabs.len();
            self.select_tab(next);
        }
    }

    /// Navigate to previous tab
    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            let prev = if self.active_tab_index == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab_index - 1
            };
            self.select_tab(prev);
        }
    }

    pub fn active_tab(&self) -> Option<&WorkspaceTab> {
        self.tabs.get(self.active_tab_index)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut WorkspaceTab> {
        self.tabs.get_mut(self.active_tab_index)
    }
}
