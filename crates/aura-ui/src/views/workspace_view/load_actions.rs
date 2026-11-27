//! Async Load Testing & Collection Runner controller with live streaming metrics
//! and workspace integration (ordered discovery, dirty-tab overrides, saved profiles).

use super::AuraWorkspaceView;
use crate::app_state::{LoadModalTab, LoadRequestEntry};
use aura_engine::runner::run_collection_with_cancel;
use aura_sync::{
    collect_ordered_request_paths, delete_load_profile, list_load_profiles, save_load_profile,
    to_workspace_relative_string, LoadTargetScope,
};
use gpui::*;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

impl AuraWorkspaceView {
    pub(super) fn open_load_tester_modal(&mut self, cx: &mut Context<Self>) {
        self.state.is_load_tester_modal_open = true;
        self.state.load_scope_dropdown_open = false;
        self.state.load_env_dropdown_open = false;
        self.state.load_profile_dropdown_open = false;
        cx.notify();
        self.spawn_refresh_load_workspace_data(cx);
    }

    pub(super) fn spawn_refresh_load_workspace_data(&mut self, cx: &mut Context<Self>) {
        let Some(root) = self.state.workspace_store.workspace_root.clone() else {
            self.state.load_discovered_requests.clear();
            self.state.load_saved_profiles.clear();
            cx.notify();
            return;
        };

        let scope_folder: Option<PathBuf> = match &self.state.load_target_scope {
            LoadTargetScope::Folder { relative_path } => Some(PathBuf::from(relative_path)),
            _ => None,
        };

        // Prefer unsaved in-memory edits from open dirty tabs over disk files.
        let dirty_tabs: HashMap<PathBuf, aura_core::model::AuraRequest> = self
            .state
            .tabs
            .iter()
            .filter(|t| t.is_dirty)
            .filter_map(|t| t.file_path.as_ref().map(|p| (p.clone(), t.request.clone())))
            .collect();

        let bg_task = cx.background_executor().spawn(async move {
            let paths = collect_ordered_request_paths(&root, scope_folder.as_deref())
                .unwrap_or_default();
            let mut entries = Vec::with_capacity(paths.len());
            for abs_path in paths {
                let (req_opt, is_dirty_tab) = if let Some(dirty_req) = dirty_tabs.get(&abs_path) {
                    (Some(dirty_req.clone()), true)
                } else if let Ok(content) = std::fs::read_to_string(&abs_path) {
                    let parsed = aura_parser::parse_request(&content)
                        .ok()
                        .or_else(|| aura_sync::toml_to_request(&content, None).ok());
                    (parsed, false)
                } else {
                    (None, false)
                };

                if let Some(req) = req_opt {
                    let rel = to_workspace_relative_string(&root, &abs_path);
                    entries.push(LoadRequestEntry {
                        relative_path: rel,
                        abs_path,
                        name: req.name.clone(),
                        method: req.method,
                        url: req.url.clone(),
                        included: true,
                        is_dirty_tab,
                        request: req,
                    });
                }
            }

            let profiles = list_load_profiles(&root).unwrap_or_default();
            (entries, profiles)
        });

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let (entries, profiles) = bg_task.await;
                let _ = this.update(&mut cx, |this, cx| {
                    this.state.load_discovered_requests = entries;
                    this.state.apply_overrides_to_discovered_requests();
                    this.state.load_saved_profiles = profiles;
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub(super) fn spawn_save_load_profile(&mut self, cx: &mut Context<Self>) {
        let Some(root) = self.state.workspace_store.workspace_root.clone() else {
            self.state.load_error = Some("Open a workspace to save load test profiles.".to_string());
            cx.notify();
            return;
        };

        let profile = self.state.to_workspace_load_profile();
        let name = profile.name.clone();

        let bg_task = cx.background_executor().spawn(async move {
            save_load_profile(&root, &profile)?;
            list_load_profiles(&root)
        });

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let res = bg_task.await;
                let _ = this.update(&mut cx, |this, cx| {
                    match res {
                        Ok(profiles) => {
                            this.state.load_saved_profiles = profiles;
                            this.state.load_error = None;
                            this.state.status_message = Some(format!("Saved load profile '{name}'"));
                        }
                        Err(e) => {
                            this.state.load_error = Some(format!("Failed to save profile: {e:#}"));
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub(super) fn spawn_delete_load_profile(&mut self, name: String, cx: &mut Context<Self>) {
        let Some(root) = self.state.workspace_store.workspace_root.clone() else {
            return;
        };
        let name_clone = name.clone();

        let bg_task = cx.background_executor().spawn(async move {
            delete_load_profile(&root, &name_clone)?;
            list_load_profiles(&root)
        });

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                if let Ok(profiles) = bg_task.await {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.state.load_saved_profiles = profiles;
                        this.state.status_message = Some(format!("Deleted load profile '{name}'"));
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    pub(super) fn spawn_start_load_test(&mut self, cx: &mut Context<Self>) {
        if self.state.load_is_running {
            return;
        }

        let requests = self.state.effective_load_requests();
        if requests.is_empty() {
            self.state.load_error = Some(match &self.state.load_target_scope {
                LoadTargetScope::ActiveRequest => "No active request tab open to benchmark.".to_string(),
                LoadTargetScope::Workspace => "No included requests found in the active workspace.".to_string(),
                LoadTargetScope::Folder { relative_path } => {
                    format!("No included requests found in folder '{relative_path}'.")
                }
            });
            cx.notify();
            return;
        }

        let config = self.state.build_load_runner_config();
        let context = self.state.template_context_for_load();
        let options = aura_engine::ExecutionOptions {
            domain_certs: self.state.domain_client_certs.clone(),
            vault_config: self.state.vault_config.clone(),
            global_proxy: Some(self.state.global_proxy.clone()),
            cookie_jar: self.state.cookie_jar.clone(),
            interceptor: Some(self.state.interceptor_manager.clone()),
        };

        let cancel = CancellationToken::new();
        self.state.load_cancel = Some(cancel.clone());
        self.state.load_is_running = true;
        self.state.load_error = None;
        self.state.load_report = None;
        self.state.load_live_records.clear();
        self.state.load_modal_tab = LoadModalTab::ResultsAndCharts;
        self.state.status_message = Some(format!(
            "Load test started ({} profile, {} request template(s))...",
            self.state.load_profile_mode.label(),
            requests.len()
        ));
        cx.notify();

        let (log_tx, mut log_rx) =
            tokio::sync::mpsc::unbounded_channel::<aura_engine::RunnerExecutionRecord>();

        // Stream live records incrementally into UI state
        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                while let Some(rec) = log_rx.recv().await {
                    let updated = this.update(&mut cx, |this, cx| {
                        this.state.load_live_records.push(rec);
                        if this.state.load_live_records.len() > 2000 {
                            let excess = this.state.load_live_records.len() - 2000;
                            this.state.load_live_records.drain(0..excess);
                        }
                        cx.notify();
                    });
                    if updated.is_err() {
                        break;
                    }
                }
            }
        })
        .detach();

        // Execute multi-VU runner on dedicated Tokio runtime off the GPUI thread
        let bg_task = cx.background_executor().spawn(async move {
            run_on_tokio_runtime(requests, context, options, config, Some(log_tx), cancel).await
        });

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let res = bg_task.await;
                let _ = this.update(&mut cx, |this, cx| {
                    this.state.load_is_running = false;
                    this.state.load_cancel = None;
                    match res {
                        Ok(report) => {
                            this.state.status_message = Some(format!(
                                "Load test complete: {} reqs, {:.1} RPS, p95 {} ms ({:.1}% err)",
                                report.total_requests,
                                report.requests_per_second,
                                report.p95_latency_ms,
                                report.error_rate_percent
                            ));
                            this.state.load_report = Some(report);
                        }
                        Err(e) => {
                            let msg = format!("{e:#}");
                            this.state.load_error = Some(msg.clone());
                            this.state.status_message = Some(format!("Load test failed: {msg}"));
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub(super) fn stop_load_test(&mut self, cx: &mut Context<Self>) {
        if let Some(cancel) = self.state.load_cancel.take() {
            cancel.cancel();
            self.state.status_message = Some("Stopping load test gracefully...".to_string());
            cx.notify();
        }
    }

    pub(super) fn spawn_pick_load_data_file(&mut self, cx: &mut Context<Self>) {
        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let Some(path) = crate::services::WorkspaceService::pick_data_file().await else {
                    return;
                };
                let _ = this.update(&mut cx, |this, cx| {
                    let stored = match &this.state.workspace_store.workspace_root {
                        Some(root) if path.starts_with(root) => {
                            to_workspace_relative_string(root, &path)
                        }
                        _ => path.display().to_string(),
                    };
                    this.state.load_data_file_path = stored;
                    this.state.load_data_enabled = true;
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub(super) fn spawn_export_load_report(&mut self, fmt: LoadExportFormat, cx: &mut Context<Self>) {
        let report_opt = self.state.load_report.clone();
        let records = report_opt
            .as_ref()
            .map(|r| r.execution_log.clone())
            .unwrap_or_else(|| self.state.load_live_records.clone());
        if records.is_empty() && report_opt.is_none() {
            self.state.status_message = Some("Run a load test before exporting a report.".into());
            cx.notify();
            return;
        }

        let report = report_opt.unwrap_or_else(|| {
            let total_dur = records.iter().map(|r| r.elapsed_ms).max().unwrap_or(1);
            aura_engine::RunnerReport {
                total_requests: records.len(),
                total_duration_ms: total_dur,
                execution_log: records.clone(),
                ..Default::default()
            }
        });

        let (default_name, ext) = match fmt {
            LoadExportFormat::CsvJtl => ("aura-load-samples.csv", "csv"),
            LoadExportFormat::Json => ("aura-load-report.json", "json"),
            LoadExportFormat::Html => ("aura-load-report.html", "html"),
        };

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let Some(path) =
                    crate::services::WorkspaceService::pick_save_report_file(default_name, ext)
                        .await
                else {
                    return;
                };
                let content_res: anyhow::Result<String> = match fmt {
                    LoadExportFormat::CsvJtl => {
                        Ok(aura_engine::export_samples_jtl_csv(&report.execution_log))
                    }
                    LoadExportFormat::Json => aura_engine::export_samples_json(&report),
                    LoadExportFormat::Html => {
                        Ok(aura_engine::generate_html_report("Aura Load Test Report", &report))
                    }
                };
                let write_res = match content_res {
                    Ok(body) => tokio::fs::write(&path, body).await.map_err(anyhow::Error::from),
                    Err(e) => Err(e),
                };
                let _ = this.update(&mut cx, |this, cx| {
                    this.state.status_message = Some(match write_res {
                        Ok(()) => format!("Exported report to {}", path.display()),
                        Err(e) => format!("Failed to export report: {e:#}"),
                    });
                    cx.notify();
                });
            }
        })
        .detach();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadExportFormat {
    CsvJtl,
    Json,
    Html,
}

async fn run_on_tokio_runtime(
    requests: Vec<aura_core::model::AuraRequest>,
    context: aura_core::templating::TemplateContext,
    options: aura_engine::ExecutionOptions,
    config: aura_engine::RunnerConfig,
    log_tx: Option<tokio::sync::mpsc::UnboundedSender<aura_engine::RunnerExecutionRecord>>,
    cancel: CancellationToken,
) -> anyhow::Result<aura_engine::RunnerReport> {
    if tokio::runtime::Handle::try_current().is_ok() {
        return Ok(run_collection_with_cancel(
            requests,
            context,
            options,
            config,
            log_tx,
            cancel,
        )
        .await);
    }

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .build()?;
    Ok(rt.block_on(run_collection_with_cancel(
        requests,
        context,
        options,
        config,
        log_tx,
        cancel,
    )))
}
