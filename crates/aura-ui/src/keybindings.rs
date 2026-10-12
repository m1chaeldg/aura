//! Power User Ergonomics: Keyboard Shortcuts & Presets (VS Code, IntelliJ IDEA, Sublime Text).
//!
//! Provides customizable keymap profiles allowing developers to navigate the workspace,
//! execute requests, and control layout using their existing muscle memory.

use serde::{Deserialize, Serialize};

/// High-level actions that can be triggered via keyboard shortcuts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    /// Send the active request (default: `Cmd/Ctrl + Enter`)
    SendRequest,
    /// Open the global command palette (default: `Cmd/Ctrl + P`)
    OpenCommandPalette,
    /// Close the command palette modal (default: `Escape`)
    CloseCommandPalette,
    /// Toggle the App/System Log Viewer (default: `Cmd/Ctrl + Shift + L`)
    ToggleLogViewer,
    /// Append a new untitled request tab (default: `Cmd/Ctrl + N` / `Cmd/Ctrl + T`)
    NewRequest,
    /// Close the active workspace tab (default: `Cmd/Ctrl + W`)
    CloseActiveTab,
    /// Toggle layout between Horizontal and Vertical (default: `Cmd/Ctrl + \`)
    ToggleLayout,
    /// Reset the pane divider to 50/50 split (default: `Cmd/Ctrl + R`)
    ResetDivider,
    /// Save the active request and clear dirty indicator (default: `Cmd/Ctrl + S`)
    SaveRequest,
    /// Switch to the next tab (default: `Ctrl + Tab`)
    NextTab,
    /// Switch to the previous tab (default: `Ctrl + Shift + Tab`)
    PrevTab,
}

impl Action {
    /// User-friendly title for the action
    pub fn title(&self) -> &'static str {
        match self {
            Self::SendRequest => "Send Request",
            Self::OpenCommandPalette => "Open Command Palette",
            Self::CloseCommandPalette => "Close Command Palette",
            Self::ToggleLogViewer => "Toggle System Log Viewer",
            Self::NewRequest => "New Request Tab",
            Self::CloseActiveTab => "Close Active Tab",
            Self::ToggleLayout => "Toggle Layout Orientation",
            Self::ResetDivider => "Reset Divider (50/50)",
            Self::SaveRequest => "Save Request",
            Self::NextTab => "Next Tab",
            Self::PrevTab => "Previous Tab",
        }
    }
}

/// Supported keymap presets for popular IDEs and code editors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum KeybindingPreset {
    #[default]
    VsCode,
    IntelliJ,
    SublimeText,
}

impl KeybindingPreset {
    /// Display name of the keymap preset
    pub fn name(&self) -> &'static str {
        match self {
            Self::VsCode => "Visual Studio Code",
            Self::IntelliJ => "IntelliJ IDEA",
            Self::SublimeText => "Sublime Text",
        }
    }

    /// Primary keyboard shortcut string for an action under this preset
    pub fn primary_shortcut(&self, action: Action) -> &'static str {
        match (self, action) {
            // VS Code keymap
            (Self::VsCode, Action::SendRequest) => "Ctrl+Enter",
            (Self::VsCode, Action::OpenCommandPalette) => "Ctrl+P",
            (Self::VsCode, Action::CloseCommandPalette) => "Escape",
            (Self::VsCode, Action::ToggleLogViewer) => "Ctrl+Shift+L",
            (Self::VsCode, Action::NewRequest) => "Ctrl+N",
            (Self::VsCode, Action::CloseActiveTab) => "Ctrl+W",
            (Self::VsCode, Action::ToggleLayout) => "Ctrl+\\",
            (Self::VsCode, Action::ResetDivider) => "Ctrl+R",
            (Self::VsCode, Action::SaveRequest) => "Ctrl+S",
            (Self::VsCode, Action::NextTab) => "Ctrl+Tab",
            (Self::VsCode, Action::PrevTab) => "Ctrl+Shift+Tab",

            // IntelliJ IDEA keymap
            (Self::IntelliJ, Action::SendRequest) => "Ctrl+Enter",
            (Self::IntelliJ, Action::OpenCommandPalette) => "Shift+Shift",
            (Self::IntelliJ, Action::CloseCommandPalette) => "Escape",
            (Self::IntelliJ, Action::ToggleLogViewer) => "Ctrl+Shift+L",
            (Self::IntelliJ, Action::NewRequest) => "Ctrl+Alt+Insert",
            (Self::IntelliJ, Action::CloseActiveTab) => "Ctrl+F4",
            (Self::IntelliJ, Action::ToggleLayout) => "Ctrl+\\",
            (Self::IntelliJ, Action::ResetDivider) => "Ctrl+R",
            (Self::IntelliJ, Action::SaveRequest) => "Ctrl+S",
            (Self::IntelliJ, Action::NextTab) => "Alt+Right",
            (Self::IntelliJ, Action::PrevTab) => "Alt+Left",

            // Sublime Text keymap
            (Self::SublimeText, Action::SendRequest) => "Ctrl+Enter",
            (Self::SublimeText, Action::OpenCommandPalette) => "Ctrl+Shift+P",
            (Self::SublimeText, Action::CloseCommandPalette) => "Escape",
            (Self::SublimeText, Action::ToggleLogViewer) => "Ctrl+Shift+L",
            (Self::SublimeText, Action::NewRequest) => "Ctrl+N",
            (Self::SublimeText, Action::CloseActiveTab) => "Ctrl+W",
            (Self::SublimeText, Action::ToggleLayout) => "Alt+Shift+2",
            (Self::SublimeText, Action::ResetDivider) => "Ctrl+R",
            (Self::SublimeText, Action::SaveRequest) => "Ctrl+S",
            (Self::SublimeText, Action::NextTab) => "Ctrl+PageDown",
            (Self::SublimeText, Action::PrevTab) => "Ctrl+PageUp",
        }
    }

    /// Resolve an incoming key event into an action based on active preset rules
    pub fn resolve(&self, key: &str, ctrl_or_cmd: bool, shift: bool, alt: bool) -> Option<Action> {
        let k = key.to_lowercase();

        // Escape always closes command palette regardless of preset
        if k == "escape" {
            return Some(Action::CloseCommandPalette);
        }

        // Shared universal shortcuts: Send Request (Ctrl+Enter), Save Request (Ctrl+S)
        if ctrl_or_cmd && !shift && !alt {
            match k.as_str() {
                "enter" => return Some(Action::SendRequest),
                "s" => return Some(Action::SaveRequest),
                "r" => return Some(Action::ResetDivider),
                "\\" => return Some(Action::ToggleLayout),
                _ => {}
            }
        }

        // Preset-specific mappings
        match self {
            Self::VsCode => {
                if ctrl_or_cmd && !shift && !alt {
                    match k.as_str() {
                        "p" => Some(Action::OpenCommandPalette),
                        "n" | "t" => Some(Action::NewRequest),
                        "w" => Some(Action::CloseActiveTab),
                        "tab" => Some(Action::NextTab),
                        _ => None,
                    }
                } else if ctrl_or_cmd && shift && !alt {
                    match k.as_str() {
                        "l" => Some(Action::ToggleLogViewer),
                        "p" => Some(Action::OpenCommandPalette),
                        "tab" => Some(Action::PrevTab),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            Self::IntelliJ => {
                if ctrl_or_cmd && shift && !alt {
                    match k.as_str() {
                        "a" => Some(Action::OpenCommandPalette),
                        "l" => Some(Action::ToggleLogViewer),
                        _ => None,
                    }
                } else if ctrl_or_cmd && !shift && !alt {
                    match k.as_str() {
                        "p" => Some(Action::OpenCommandPalette),
                        "n" => Some(Action::NewRequest),
                        "f4" | "w" => Some(Action::CloseActiveTab),
                        _ => None,
                    }
                } else if ctrl_or_cmd && alt && !shift && k == "insert" {
                    Some(Action::NewRequest)
                } else if alt && !ctrl_or_cmd && !shift {
                    match k.as_str() {
                        "right" => Some(Action::NextTab),
                        "left" => Some(Action::PrevTab),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            Self::SublimeText => {
                if ctrl_or_cmd && shift && !alt {
                    match k.as_str() {
                        "p" => Some(Action::OpenCommandPalette),
                        "l" => Some(Action::ToggleLogViewer),
                        _ => None,
                    }
                } else if ctrl_or_cmd && !shift && !alt {
                    match k.as_str() {
                        "n" => Some(Action::NewRequest),
                        "w" => Some(Action::CloseActiveTab),
                        "pagedown" => Some(Action::NextTab),
                        "pageup" => Some(Action::PrevTab),
                        _ => None,
                    }
                } else if alt && shift && !ctrl_or_cmd && k == "2" {
                    Some(Action::ToggleLayout)
                } else {
                    None
                }
            }
        }
    }

    /// List all mapped actions with their human-readable shortcut string
    pub fn all_shortcuts(&self) -> Vec<(Action, &'static str)> {
        vec![
            (
                Action::SendRequest,
                self.primary_shortcut(Action::SendRequest),
            ),
            (
                Action::OpenCommandPalette,
                self.primary_shortcut(Action::OpenCommandPalette),
            ),
            (
                Action::ToggleLogViewer,
                self.primary_shortcut(Action::ToggleLogViewer),
            ),
            (
                Action::NewRequest,
                self.primary_shortcut(Action::NewRequest),
            ),
            (
                Action::CloseActiveTab,
                self.primary_shortcut(Action::CloseActiveTab),
            ),
            (
                Action::ToggleLayout,
                self.primary_shortcut(Action::ToggleLayout),
            ),
            (
                Action::ResetDivider,
                self.primary_shortcut(Action::ResetDivider),
            ),
            (
                Action::SaveRequest,
                self.primary_shortcut(Action::SaveRequest),
            ),
            (Action::NextTab, self.primary_shortcut(Action::NextTab)),
            (Action::PrevTab, self.primary_shortcut(Action::PrevTab)),
        ]
    }
}
