//! Core HTTP engine, collection runner, mock server, and logging broadcaster for Aura.

pub mod engine;
pub mod grpc;
pub mod interceptor_proxy;
pub mod logging;
pub mod mock;
pub mod mtls;
pub mod oauth2;
pub mod proxy;
pub mod runner;
pub mod sse;
pub mod tls;
pub mod vaults;
pub mod websocket;

pub use engine::{
    execute_request, execute_request_with_options, fetch_graphql_introspection, ExecutionOptions,
};
pub use grpc::{execute_grpc_call, reflect_grpc_services};
pub use interceptor_proxy::{start_capture_proxy, CaptureProxyHandle};
pub use logging::{
    default_log_dir, init_system_logging, BroadcasterLayer, LogBroadcaster, LogEntry, LogLevel,
    LoggingGuard,
};
pub use mock::MockServer;
pub use mtls::{build_reqwest_identity, resolve_client_cert_for_url};
pub use oauth2::{
    exchange_oauth2_token, maybe_refresh_oauth2_auth, run_oauth2_token_acquisition,
    OAuth2TokenResult,
};
pub use runner::{run_collection, RunnerConfig, RunnerExecutionRecord, RunnerReport};
pub use sse::{execute_sse_request, SseStreamParser};
pub use vaults::{
    fetch_aws_secrets_manager, fetch_hashicorp_vault_secret, keychain_delete, keychain_get,
    keychain_set, resolve_secret_reference, resolve_template_secrets, VaultConfig,
};
pub use websocket::{connect_websocket, WsCommand, WsEvent};
