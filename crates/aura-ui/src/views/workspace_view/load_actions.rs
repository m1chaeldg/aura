//! Async Load Testing & Collection Runner controller with live streaming metrics.

use super::AuraWorkspaceView;
use aura_engine::runner::run_collection_with_cancel;
use gpui::*;
use tokio_util::sync::CancellationToken;

impl AuraWorkspaceView {
    pub(super) fn spawn_start_load_test(&mut self, cx: &mut Context<Self>) {
        if self.state.load_is_running {
            return;
        }

        let requests: Vec<aura_core::model::AuraRequest> = if self.state.load_use_collection {
            let mut all: Vec<aura_core::model::AuraRequest> = self
                .state
                .collections
                .iter()
                .flat_map(|c| c.requests.iter().cloned())
                .collect();
            if all.is_empty() {
                if let Some(tab) = self.state.active_tab() {
                    all.push(tab.request.clone());
                }
            }
            all
        } else {
            self.state
                .active_tab()
                .map(|t| vec![t.request.clone()])
                .unwrap_or_default()
        };

        if requests.is_empty() {
            self.state.load_error = Some("No request open to benchmark.".to_string());
            cx.notify();
            return;
        }

        let config = self.state.build_load_runner_config();
        let context = self.state.active_template_context();
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
                    this.state.load_data_file_path = path.display().to_string();
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
