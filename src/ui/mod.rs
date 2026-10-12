//! GPUI Component Views and Application State strictly adhering to DESIGN.md.

pub mod workspace_view;
pub use workspace_view::AuraWorkspaceView;

use crate::model::{AuraRequest, AuraResponse, HttpMethod};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutOrientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorTab {
    Params,
    Headers,
    Variables,
    Body,
    Scripts,
    Tests,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseTab {
    Json,
    Table,
    Headers,
    Tests,
    Raw,
}

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

    /// Color hex code for tab badge
    pub fn badge_color(&self) -> &'static str {
        self.method.badge_color()
    }
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
    pub split_ratio: f32, // 0.0 to 1.0 (default 0.5)
    pub orientation: LayoutOrientation,
    pub active_editor_tab: EditorTab,
    pub active_response_tab: ResponseTab,
    pub is_code_mode: bool, // Dual-mode: Visual vs Code Editor (TOML)
    pub is_command_palette_open: bool,
    pub is_log_viewer_open: bool,
    pub command_palette_search: String,
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
            status_message: None,
        }
    }
}

impl AuraAppState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open or focus a tab for a request
    pub fn open_request(&mut self, req: AuraRequest) {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == req.id) {
            for t in &mut self.tabs {
                t.is_active = false;
            }
            self.tabs[pos].is_active = true;
            self.active_tab_index = pos;
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

    /// Close tab by ID
    pub fn close_tab(&mut self, id: &str) {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            self.tabs.remove(pos);
            if self.tabs.is_empty() {
                self.active_tab_index = 0;
            } else if self.active_tab_index >= self.tabs.len() {
                self.active_tab_index = self.tabs.len() - 1;
                self.tabs[self.active_tab_index].is_active = true;
            }
        }
    }

    /// Reset pane divider to 50/50 split
    pub fn reset_divider(&mut self) {
        self.split_ratio = 0.5;
    }

    /// Toggle layout between Horizontal and Vertical
    pub fn toggle_orientation(&mut self) {
        self.orientation = match self.orientation {
            LayoutOrientation::Horizontal => LayoutOrientation::Vertical,
            LayoutOrientation::Vertical => LayoutOrientation::Horizontal,
        };
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
}
