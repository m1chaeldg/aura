//! Core HTTP engine, collection runner, mock server, and logging broadcaster for Aura.

pub mod engine;
pub mod grpc;
pub mod logging;
pub mod mock;
pub mod runner;
pub mod sse;
pub mod tls;
pub mod websocket;

pub use engine::{execute_request, fetch_graphql_introspection};
pub use grpc::{execute_grpc_call, reflect_grpc_services};
pub use logging::{
    default_log_dir, init_system_logging, BroadcasterLayer, LogBroadcaster, LogEntry, LogLevel,
    LoggingGuard,
};
pub use mock::MockServer;
pub use runner::{run_collection, RunnerConfig, RunnerExecutionRecord, RunnerReport};
pub use sse::{execute_sse_request, SseStreamParser};
pub use websocket::{connect_websocket, WsCommand, WsEvent};
