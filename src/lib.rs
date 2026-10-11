//! Aura: High-performance REST API Client in Rust with built-in workspace management,
//! sandboxed offline scripting, and GPUI visual interface.

pub mod design;
pub mod engine;
pub mod importers;
pub mod interceptor;
pub mod logging;
pub mod mock;
pub mod model;
pub mod runner;
pub mod scripting;
pub mod security;
pub mod sync;
pub mod templating;
pub mod ui;

pub use design::*;
pub use engine::execute_request;
pub use model::*;
pub use runner::run_collection;
