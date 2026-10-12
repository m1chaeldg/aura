//! High-concurrency Collection Runner & Statistical HDR Latency Profiler.

use crate::engine::execute_request;
use aura_core::model::{AuraRequest, AuraResponse};
use aura_core::templating::TemplateContext;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Configuration for collection runner execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerConfig {
    pub total_iterations: usize,
    pub concurrency: usize,
    pub delay_ms: u64,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            total_iterations: 10,
            concurrency: 2,
            delay_ms: 0,
        }
    }
}

/// A single execution record in the live runner feed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerExecutionRecord {
    pub iteration: usize,
    pub request_name: String,
    pub method: String,
    pub status: u16,
    pub latency_ms: u64,
    pub passed: bool,
    pub error: Option<String>,
}

/// Statistical HDR Latency Profiler & summary metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunnerReport {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub error_rate_percent: f64,
    pub requests_per_second: f64,
    pub min_latency_ms: u64,
    pub mean_latency_ms: f64,
    pub p50_latency_ms: u64,
    pub p90_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
    pub max_latency_ms: u64,
    pub total_duration_ms: u64,
    pub execution_log: Vec<RunnerExecutionRecord>,
}

/// Execute a collection or list of requests with worker pool and HDR metrics
pub async fn run_collection(
    requests: Vec<AuraRequest>,
    context: TemplateContext,
    config: RunnerConfig,
    log_tx: Option<mpsc::UnboundedSender<RunnerExecutionRecord>>,
) -> RunnerReport {
    if requests.is_empty() || config.total_iterations == 0 {
        return RunnerReport::default();
    }

    let concurrency = config.concurrency.max(1);
    let start_overall = Instant::now();

    let shared_requests = Arc::new(requests);
    let iteration_counter = Arc::new(AtomicUsize::new(0));
    let records = Arc::new(parking_lot::Mutex::new(Vec::new()));

    let mut handles = Vec::new();

    for _ in 0..concurrency {
        let reqs = Arc::clone(&shared_requests);
        let counter = Arc::clone(&iteration_counter);
        let recs = Arc::clone(&records);
        let tx = log_tx.clone();
        let ctx = context.clone();
        let total = config.total_iterations;
        let delay = config.delay_ms;

        let handle = tokio::spawn(async move {
            loop {
                let current_iter = counter.fetch_add(1, Ordering::SeqCst);
                if current_iter >= total {
                    break;
                }

                for req in reqs.iter() {
                    let res: Result<AuraResponse, anyhow::Error> = execute_request(req, &ctx).await;
                    let (status, latency, passed, err_msg) = match res {
                        Ok(resp) => {
                            let pass =
                                resp.status < 400 && resp.assertions.iter().all(|a| a.passed);
                            (resp.status, resp.duration_ms, pass, None)
                        }
                        Err(e) => (0, 0, false, Some(e.to_string())),
                    };

                    let record = RunnerExecutionRecord {
                        iteration: current_iter + 1,
                        request_name: req.name.clone(),
                        method: req.method.as_str().to_string(),
                        status,
                        latency_ms: latency,
                        passed,
                        error: err_msg,
                    };

                    recs.lock().push(record.clone());
                    if let Some(ref sender) = tx {
                        let _ = sender.send(record);
                    }

                    if delay > 0 {
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                    }
                }
            }
        });
        handles.push(handle);
    }

    for h in handles {
        let _ = h.await;
    }

    let total_duration = start_overall.elapsed();
    let _total_duration_ms = total_duration.as_millis() as u64;

    let records_list = records.lock().clone();
    compute_report(records_list, total_duration)
}

fn compute_report(records: Vec<RunnerExecutionRecord>, total_duration: Duration) -> RunnerReport {
    let total_requests = records.len();
    if total_requests == 0 {
        return RunnerReport::default();
    }

    let mut latencies: Vec<u64> = records.iter().map(|r| r.latency_ms).collect();
    latencies.sort_unstable();

    let successful = records.iter().filter(|r| r.passed).count();
    let failed = total_requests - successful;
    let error_rate = (failed as f64 / total_requests as f64) * 100.0;

    let total_secs = total_duration.as_secs_f64().max(0.001);
    let rps = total_requests as f64 / total_secs;

    let min = *latencies.first().unwrap_or(&0);
    let max = *latencies.last().unwrap_or(&0);
    let sum: u64 = latencies.iter().sum();
    let mean = sum as f64 / total_requests as f64;

    let percentile = |pct: f64| -> u64 {
        if latencies.is_empty() {
            return 0;
        }
        let rank = (pct / 100.0 * (latencies.len() - 1) as f64).round() as usize;
        latencies[rank.min(latencies.len() - 1)]
    };

    RunnerReport {
        total_requests,
        successful_requests: successful,
        failed_requests: failed,
        error_rate_percent: error_rate,
        requests_per_second: rps,
        min_latency_ms: min,
        mean_latency_ms: mean,
        p50_latency_ms: percentile(50.0),
        p90_latency_ms: percentile(90.0),
        p95_latency_ms: percentile(95.0),
        p99_latency_ms: percentile(99.0),
        max_latency_ms: max,
        total_duration_ms: total_duration.as_millis() as u64,
        execution_log: records,
    }
}
