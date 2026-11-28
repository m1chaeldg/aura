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
pub mod runner_reporting;
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
pub use aura_core::assertions::PerformanceAssertionConfig;
pub use aura_core::data_feeder::{DataEofBehavior, DataFeederConfig, DataSharingMode};
pub use aura_core::model::ThinkTimeConfig;
pub use runner::{
    run_collection, run_collection_with_cancel, run_collection_with_options, LoadProfileMode,
    LoadStage, ResolvedSchedule, RunnerConfig, RunnerExecutionRecord, RunnerReport, SlaThresholds,
    SlaViolation, SynchronizingTimerConfig,
};
pub use runner_reporting::{
    compute_aggregate_report, compute_time_series, export_samples_json, export_samples_jtl_csv,
    generate_html_report, EndpointAggregateRow, TimeSeriesBucket,
};
pub use sse::{execute_sse_request, SseStreamParser};
pub use vaults::{
    clear_secret_cache, delete_keepass_profile_password, delete_vault_profile_token,
    fetch_aws_secrets_manager, fetch_hashicorp_vault_secret, fetch_keepass_secret,
    get_keepass_profile_password, get_vault_profile_token, keychain_delete, keychain_get,
    keychain_set, resolve_inline_secrets_in_request, resolve_secret_reference,
    resolve_template_secrets, secret_cache_len, set_keepass_profile_password,
    set_vault_profile_token, test_aws_credentials, test_hashicorp_vault_connection,
    test_keepass_connection, AwsVaultProfile, HashiCorpVaultProfile, KeePassProfile, VaultConfig,
};
pub use websocket::{connect_websocket, WsCommand, WsEvent};
