//! Asynchronous domain services conforming to AGENTS.md Layer B.

pub mod ai_discovery;
pub mod ai_service;
pub mod execution_service;
pub mod format_service;
pub mod response_bi_service;
pub mod response_data_service;
pub mod update_service;
pub mod workspace_service;

pub use ai_discovery::{AiDiscoveryResult, KNOWN_AI_CLIS, discover_all, merge_discovery_into_preferences};
pub use ai_service::{AiChatMessage, AiChatRole, AiContextSnapshot, AiResponse, AiService, AuraAgentAction};

pub use execution_service::ExecutionService;
pub use format_service::{FormatService, ResponseLanguage};
pub use response_bi_service::{
    AggregationOp, BiSummary, ChartSeriesPoint, ChartType, ColumnSummary, ResponseBiService,
};
pub use response_data_service::{ResponseDataService, TableDataset, TreeNode, TreeNodeKind};
pub use update_service::{CURRENT_VERSION, GITHUB_REPO, ReleaseInfo, UpdateCheckOutcome, UpdateService};
pub use workspace_service::{WorkspaceDiskEvent, WorkspaceService};
