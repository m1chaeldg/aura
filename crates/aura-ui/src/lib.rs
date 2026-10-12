//! GPUI Presentation Views, Domain Stores, and Ergonomics for Aura.

pub mod app_state;
pub mod keybindings;
pub mod preferences;
pub mod services;
pub mod stores;
pub mod views;

pub use app_state::{AuraAppState, CommandPaletteItem};
pub use preferences::AuraPreferences;
pub use aura_core::design;
pub use keybindings::{Action, KeybindingPreset};
pub use services::{ExecutionService, FormatService, WorkspaceDiskEvent, WorkspaceService};
pub use stores::{
    EditorTab, LayoutOrientation, LayoutStore, ResponseTab, SessionStore, WorkspaceStore,
    WorkspaceTab, WorkspaceTreeNode,
};
pub use views::AuraWorkspaceView;
