//! Session Domain Store: Tracks open document tabs, active selection, and dirty states.

use aura_core::model::{AuraRequest, AuraResponse, HttpMethod};
use serde::{Deserialize, Serialize};

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
