//! Layout Domain Store: Tracks split ratios, orientation, tabs, and modal visibility.

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutStore {
    pub split_ratio: f32, // Clamped between 0.1 and 0.9 (default 0.5)
    pub orientation: LayoutOrientation,
    pub active_editor_tab: EditorTab,
    pub active_response_tab: ResponseTab,
    pub is_code_mode: bool,
    pub is_command_palette_open: bool,
    pub is_log_viewer_open: bool,
    pub command_palette_search: String,
    pub selected_palette_index: usize,
    pub is_dragging_divider: bool,
    pub status_message: Option<String>,
}

impl Default for LayoutStore {
    fn default() -> Self {
        Self {
            split_ratio: 0.5,
            orientation: LayoutOrientation::Horizontal,
            active_editor_tab: EditorTab::Params,
            active_response_tab: ResponseTab::Json,
            is_code_mode: false,
            is_command_palette_open: false,
            is_log_viewer_open: false,
            command_palette_search: String::new(),
            selected_palette_index: 0,
            is_dragging_divider: false,
            status_message: None,
        }
    }
}

impl LayoutStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset pane divider to 50/50 split
    pub fn reset_divider(&mut self) {
        self.split_ratio = 0.5;
    }

    /// Set split ratio clamped between 0.1 and 0.9
    pub fn set_split_ratio(&mut self, ratio: f32) {
        self.split_ratio = ratio.clamp(0.1, 0.9);
    }

    /// Toggle layout between Horizontal and Vertical
    pub fn toggle_orientation(&mut self) {
        self.orientation = match self.orientation {
            LayoutOrientation::Horizontal => LayoutOrientation::Vertical,
            LayoutOrientation::Vertical => LayoutOrientation::Horizontal,
        };
    }

    pub fn toggle_command_palette(&mut self) {
        self.is_command_palette_open = !self.is_command_palette_open;
        if self.is_command_palette_open {
            self.command_palette_search.clear();
            self.selected_palette_index = 0;
        }
    }

    pub fn toggle_log_viewer(&mut self) {
        self.is_log_viewer_open = !self.is_log_viewer_open;
    }

    pub fn toggle_code_mode(&mut self) {
        self.is_code_mode = !self.is_code_mode;
    }

    pub fn set_status_message(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
    }

    pub fn clear_status_message(&mut self) {
        self.status_message = None;
    }
}
