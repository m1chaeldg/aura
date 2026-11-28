//! Domain data models for Aura API Client.

pub mod auth;
pub mod file_format;
pub mod flow;
pub mod http;
pub mod protocol;
pub mod request;
pub mod response;
pub mod workspace;

pub use auth::{AuthConfig, ClientCertificateConfig, ProxyConfig, ProxyProtocol};
pub use file_format::{AuraFileRequest, ScriptHooks, Settings};
pub use flow::{FlowControl, ThinkTimeConfig, pick_weighted_index};
pub use http::{BodyType, HeaderItem, HttpMethod, MultipartField, MultipartPart, ParamItem};
pub use protocol::{GrpcConfig, RequestProtocol, SseEvent, StreamMessage, WsMessageTemplate};
pub use request::{AuraRequest, RequestSettings, ScriptConfig};
pub use response::{
    AssertionResult, AuraResponse, ConsoleMessage, ExecutionReport, RequestTimingBreakdown,
    ScriptError, TestAssertion,
};
pub use workspace::{
    Collection, CollectionMeta, Environment, EnvironmentProfile, Folder, VariableItem,
    VariableValue, Workspace, WorkspaceManifest,
};
