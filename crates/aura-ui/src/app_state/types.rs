//! Shared UI state enums, dialog states, and drag-and-drop payloads for `AuraAppState`.

use aura_core::model::{AuraRequest, HttpMethod};
use gpui::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Sub-tabs within the Scripts panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ScriptSubTab {
    #[default]
    PreRequest,
    PostResponse,
}

/// Top-level tab inside the Load Tester modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadModalTab {
    #[default]
    SetupAndScope,
    RequestsAndFlow,
    ResultsAndCharts,
}

impl LoadModalTab {
    pub const ALL: [Self; 3] = [
        Self::SetupAndScope,
        Self::RequestsAndFlow,
        Self::ResultsAndCharts,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::SetupAndScope => "Setup & Scope",
            Self::RequestsAndFlow => "Requests & Flow",
            Self::ResultsAndCharts => "Results & Charts",
        }
    }
}

/// A discovered workspace request entry shown in the Load Tester's Requests & Flow tab.
#[derive(Debug, Clone)]
pub struct LoadRequestEntry {
    pub relative_path: String,
    pub abs_path: PathBuf,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    pub included: bool,
    pub is_dirty_tab: bool,
    pub request: AuraRequest,
}

/// UI tab inside Load Tester results section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadResultsViewTab {
    #[default]
    TimeSeriesCharts,
    AggregateTable,
    RawStream,
}

impl LoadResultsViewTab {
    pub const ALL: [Self; 3] = [
        Self::TimeSeriesCharts,
        Self::AggregateTable,
        Self::RawStream,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::TimeSeriesCharts => "Live Charts",
            Self::AggregateTable => "Aggregate Report",
            Self::RawStream => "Raw Samples",
        }
    }
}

/// Time-series metric plotted in the Live Charts tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadChartMetric {
    #[default]
    Rps,
    ActiveVus,
    LatencyPercentiles,
    ErrorRate,
}

impl LoadChartMetric {
    pub const ALL: [Self; 4] = [
        Self::Rps,
        Self::ActiveVus,
        Self::LatencyPercentiles,
        Self::ErrorRate,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Rps => "RPS",
            Self::ActiveVus => "Active VUs",
            Self::LatencyPercentiles => "Latency (p50/p95/p99)",
            Self::ErrorRate => "Error Rate %",
        }
    }
}

/// UI selector for JMeter-style jittered Think-Time mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadThinkMode {
    #[default]
    None,
    Uniform,
    Gaussian,
}

impl LoadThinkMode {
    pub const ALL: [Self; 3] = [Self::None, Self::Uniform, Self::Gaussian];

    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "No Think-Time",
            Self::Uniform => "Uniform (min..max ms)",
            Self::Gaussian => "Gaussian (mean ± std)",
        }
    }
}

/// Active input focus target for modal-free inline editing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActiveInputTarget {
    Url,
    ParamKey(usize),
    ParamValue(usize),
    HeaderKey(usize),
    HeaderValue(usize),
    VariableKey(usize),
    VariableValue(usize),
    SettingTimeout,
    SettingMaxRedirects,
    FormParamKey(usize),
    FormParamValue(usize),
    AuthBearerToken,
    AuthBasicUsername,
    AuthBasicPassword,
    AuthApiKeyName,
    AuthApiKeyValue,
    AuthAwsField(usize),
    AuthOAuth2Field(usize),
    AuthSchemeField(usize),
    SettingCertField(usize),
    SettingProxyField(usize),
    BodyText,
    PreRequestScript,
    PostResponseScript,
    TestScript,
    AssertExpr(usize),
    AssertExpected(usize),
    ExtractorVar(usize),
    ExtractorExpr(usize),
    SpecText,
}

/// Drag and drop payload for sidebar items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DragItemPayload {
    pub path: PathBuf,
    pub is_folder: bool,
}

impl gpui::Render for DragItemPayload {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex};
        let name = self
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Item");
        let icon_name = if self.is_folder {
            gpui_kit::assets::IconName::Folder
        } else {
            gpui_kit::assets::IconName::FileText
        };
        h_flex()
            .gap_1p5()
            .px_2()
            .py_1()
            .bg(cx.theme().popover)
            .border_1()
            .border_color(cx.theme().primary)
            .rounded(cx.theme().radius)
            .text_xs()
            .text_color(cx.theme().popover_foreground)
            .shadow_md()
            .child(crate::theme::icon(icon_name).xsmall())
            .child(name.to_string())
    }
}

/// Destination item for the Move To quick-pick dialog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveToDestination {
    pub display_name: String,
    pub path: PathBuf,
    pub is_root: bool,
}

/// State for active Move To quick-pick dialog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveToDialogState {
    pub source_path: PathBuf,
    pub search_query: String,
    pub selected_index: usize,
    pub destinations: Vec<MoveToDestination>,
}

impl MoveToDialogState {
    pub fn new(source_path: PathBuf, root: PathBuf, folders: Vec<PathBuf>) -> Self {
        let mut destinations = Vec::new();
        let root_name = root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Workspace Root");
        destinations.push(MoveToDestination {
            display_name: format!("/ (Root: {})", root_name),
            path: root.clone(),
            is_root: true,
        });
        for folder in folders {
            if folder == source_path || folder.starts_with(&source_path) {
                continue;
            }
            let rel = folder.strip_prefix(&root).unwrap_or(&folder);
            destinations.push(MoveToDestination {
                display_name: format!("/{}", rel.to_string_lossy().replace('\\', "/")),
                path: folder,
                is_root: false,
            });
        }
        Self {
            source_path,
            search_query: String::new(),
            selected_index: 0,
            destinations,
        }
    }

    pub fn filtered_destinations(&self) -> Vec<&MoveToDestination> {
        if self.search_query.trim().is_empty() {
            return self.destinations.iter().collect();
        }
        let q = self.search_query.to_lowercase();
        self.destinations
            .iter()
            .filter(|d| d.display_name.to_lowercase().contains(&q))
            .collect()
    }

    pub fn select_next(&mut self) {
        let total = self.filtered_destinations().len();
        if total > 0 {
            self.selected_index = (self.selected_index + 1) % total;
        }
    }

    pub fn select_prev(&mut self) {
        let total = self.filtered_destinations().len();
        if total > 0 {
            if self.selected_index == 0 {
                self.selected_index = total - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    pub fn selected_destination(&self) -> Option<&MoveToDestination> {
        let filtered = self.filtered_destinations();
        if filtered.is_empty() {
            None
        } else {
            let idx = self.selected_index.min(filtered.len() - 1);
            Some(filtered[idx])
        }
    }
}

/// State for active inline renaming in the sidebar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineRenameState {
    pub path: PathBuf,
    pub current_value: String,
}

/// State for active rename modal dialog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameDialogState {
    pub path: PathBuf,
    pub original_name: String,
    pub current_name: String,
    pub is_folder: bool,
}

/// State for active request editor modal dialogs (headers, params, body, auth).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorDialogState {
    AddHeader {
        key: String,
        value: String,
        is_value_focused: bool,
    },
    EditHeader {
        index: usize,
        key: String,
        value: String,
        is_value_focused: bool,
    },
    AddParam {
        key: String,
        value: String,
        is_value_focused: bool,
    },
    EditParam {
        index: usize,
        key: String,
        value: String,
        is_value_focused: bool,
    },
    EditBody {
        content: String,
    },
    EditBearerAuth {
        token: String,
    },
    EditBasicAuth {
        username: String,
        password: String,
        is_password_focused: bool,
    },
    EditApiKeyAuth {
        key: String,
        value: String,
        in_header: bool,
        is_value_focused: bool,
    },
    EditAwsSigV4 {
        access_key: String,
        secret_key: String,
        region: String,
        service: String,
        session_token: String,
        active_field: usize,
    },
    EditOAuth2 {
        grant_type: String,
        access_token: String,
        token_url: String,
        client_id: String,
        client_secret: String,
        active_field: usize,
    },
    AddFormParam {
        key: String,
        value: String,
        is_value_focused: bool,
    },
    AddMultipartField {
        name: String,
        value: String,
        file_path: String,
        is_file: bool,
        active_field: usize,
    },
}

/// Editing mode for the Environment Profile Editor modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnvironmentEditorMode {
    #[default]
    Ui,
    Code,
}

/// State for the modal Environment Profile Editor.
#[derive(Debug, Clone)]
pub struct EnvironmentEditorState {
    pub original_path: Option<PathBuf>,
    pub profile: aura_core::model::EnvironmentProfile,
    pub mode: EnvironmentEditorMode,
    pub code_buffer: String,
    pub code_error: Option<String>,
}

/// Top-bar dropdown menu categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopMenuKind {
    File,
    Workspace,
    View,
    Tools,
    Ai,
    Help,
}

/// Command palette entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPaletteItem {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub method: Option<HttpMethod>,
}
