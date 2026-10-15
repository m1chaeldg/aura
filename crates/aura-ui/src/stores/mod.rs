//! Domain stores conforming to AGENTS.md Layer A.

pub mod layout_store;
pub mod session_store;
pub mod workspace_store;

pub use layout_store::{EditorTab, LayoutOrientation, LayoutStore, ResponseBodyMode, ResponseTab};
pub use session_store::{SessionStore, WorkspaceTab};
pub use workspace_store::{
    ClipboardPayload, ContextMenuState, DragPlacement, SidebarDragState, WorkspaceStore,
    WorkspaceTreeNode,
};

