//! GPUI Presentation Views, Domain Stores, and Ergonomics for Aura.

pub mod app_state;
pub mod editor_support;
pub mod input_registry;
pub mod keybindings;
pub mod preferences;
pub mod services;
pub mod stores;
pub mod views;

pub use app_state::{
    AuraAppState, CommandPaletteItem, DragItemPayload, InlineRenameState, MoveToDestination,
    MoveToDialogState, RenameDialogState,
};
pub use aura_core::design;
pub use keybindings::{Action, KeybindingPreset};
pub use preferences::AuraPreferences;
pub use services::{ExecutionService, FormatService, WorkspaceDiskEvent, WorkspaceService};
pub use stores::{
    EditorTab, LayoutOrientation, LayoutStore, ResponseTab, SessionStore, WorkspaceStore,
    WorkspaceTab, WorkspaceTreeNode,
};
pub use views::AuraWorkspaceView;

/// Initializes GPUI Kit component suite, assets, and dark theme per DESIGN.md.
pub fn init_kit(cx: &mut gpui::App) {
    gpui_kit::init(cx);
    gpui_kit::component::Theme::change(gpui_kit::component::ThemeMode::Dark, None, cx);
    cx.bind_keys([gpui::KeyBinding::new(
        "ctrl-space",
        gpui_kit::component::input::ShowCompletions,
        Some("Input"),
    )]);
}
