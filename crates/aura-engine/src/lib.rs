//! Core HTTP engine, collection runner, mock server, and logging broadcaster for Aura.

pub mod engine;
pub mod logging;
pub mod mock;
pub mod runner;

pub use engine::execute_request;
pub use logging::{
    default_log_dir, init_system_logging, BroadcasterLayer, LogBroadcaster, LogEntry, LogLevel,
    LoggingGuard,
};
pub use mock::MockServer;
pub use runner::{run_collection, RunnerConfig, RunnerExecutionRecord, RunnerReport};
