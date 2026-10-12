//! Modular GPUI presentation components adhering to AGENTS.md Layer C.

pub mod command_palette_view;
pub mod request_editor_view;
pub mod request_header_view;
pub mod response_pane_view;
pub mod sidebar_view;
pub mod split_resizer_view;
pub mod tab_bar_view;
pub mod workspace_view;

pub use command_palette_view::CommandPaletteView;
pub use request_editor_view::RequestEditorView;
pub use request_header_view::RequestHeaderView;
pub use response_pane_view::ResponsePaneView;
pub use sidebar_view::SidebarView;
pub use split_resizer_view::SplitResizerView;
pub use tab_bar_view::TabBarView;
pub use workspace_view::AuraWorkspaceView;
