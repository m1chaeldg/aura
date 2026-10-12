//! Aura: High-performance REST API Client in Rust with built-in workspace management,
//! sandboxed offline scripting, and GPUI visual interface.
//!
//! Multi-crate architecture facade re-exporting domain crates:
//! - aura-core: Domain data models, templating, security, interceptors
//! - aura-engine: Network execution, collection runner, mock server, logging
//! - aura-scripting: Sandboxed offline Deno V8 JavaScript runtime
//! - aura-sync: Two-way filesystem synchronization and format importers
//! - aura-ui: GPUI presentation components, Domain Stores, and Ergonomics

pub use aura_core::design;
pub use aura_core::interceptor;
pub use aura_core::model;
pub use aura_core::security;
pub use aura_core::templating;

pub use aura_parser as parser;
pub use aura_scripting as scripting;

pub mod engine {
    pub use aura_engine::engine::*;
}
pub mod runner {
    pub use aura_engine::runner::*;
}
pub mod mock {
    pub use aura_engine::mock::*;
}
pub mod logging {
    pub use aura_engine::logging::*;
}

pub mod sync {
    pub use aura_sync::sync::*;
}
pub mod importers {
    pub use aura_sync::importers::*;
}

pub mod ui {
    pub use aura_ui::*;
}

pub use aura_core::design::*;
pub use aura_core::model::*;
pub use aura_engine::engine::execute_request;
pub use aura_engine::runner::run_collection;
pub use aura_ui::AuraWorkspaceView;
