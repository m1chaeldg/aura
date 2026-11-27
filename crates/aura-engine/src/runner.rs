//! High-concurrency Collection Runner, JMeter-style Thread Groups & Load Profiles,
//! Timers (Uniform/Gaussian Think-Time, Constant Throughput Timer, Synchronizing Rendezvous Barrier),
//! Logic Controllers (Once-Only, Weighted/Throughput Distribution, Loop, If/While Conditions),
//! Request Chaining (`setNextRequest`), Cookie/Variable propagation, & HDR Latency Profiler.

use crate::engine::{execute_request_with_options, ExecutionOptions};
use aura_core::assertions::{
    evaluate_flow_condition, evaluate_performance_assertions, PerformanceAssertionConfig,
};
use aura_core::cookies::CookieJar;
use aura_core::data_feeder::{parse_data_content, DataFeeder, DataFeederConfig};
use aura_core::model::{pick_weighted_index, AuraRequest, AuraResponse, ThinkTimeConfig};
use aura_core::templating::TemplateContext;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;

/// Single stage in a Stepped / Spike load test schedule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoadStage {
    pub duration_secs: f64,
    pub target_vus: usize,
}

impl LoadStage {
    pub fn new(target_vus: usize, duration_secs: f64) -> Self {
        Self {
            duration_secs,
            target_vus,
        }
    }
}

/// Synchronizing Timer (JMeter Rendezvous Barrier) pausing `group_size` VUs
/// before releasing them together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SynchronizingTimerConfig {
    pub group_size: usize,
    pub timeout_ms: u64,
}

impl Default for SynchronizingTimerConfig {
    fn default() -> Self {
        Self {
            group_size: 0,
            timeout_ms: 5000,
        }
    }
}

/// High-level preset selector for UI and scripted configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LoadProfileMode {
    #[default]
    FixedIterations,
    Duration,
    RampUpDown,
    SteppedStages,
}

impl LoadProfileMode {
    pub const ALL: [Self; 4] = [
        Self::FixedIterations,
        Self::Duration,
        Self::RampUpDown,
        Self::SteppedStages,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::FixedIterations => "Fixed Iterations",
            Self::Duration => "Duration (Soak)",
            Self::RampUpDown => "Ramp Up / Down",
            Self::SteppedStages => "Spike / Stepped",
        }
    }
}

/// Global SLA threshold gates evaluated across the full runner report (for CI/CD).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct SlaThresholds {
    #[serde(default)]
    pub max_p95_ms: Option<u64>,
    #[serde(default)]
    pub max_p99_ms: Option<u64>,
    #[serde(default)]
    pub max_error_rate_percent: Option<f64>,
}

/// Specific SLA gate failure reported in `RunnerReport::sla_violations`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlaViolation {
    pub metric: String,
    pub actual: String,
    pub threshold: String,
}

/// Configuration for collection runner & JMeter-style load execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerConfig {
    pub total_iterations: usize,
    pub concurrency: usize,
    pub delay_ms: u64,
    /// Optional gradual ramp-up period (seconds) to scale VUs from `0` to `concurrency`.
    #[serde(default)]
    pub ramp_up_secs: f64,
    /// Optional gradual ramp-down period (seconds) at the tail end of a duration run.
    #[serde(default)]
    pub ramp_down_secs: f64,
    /// Optional time-boxed soak/stress duration (seconds). Overrides `total_iterations` when set.
    #[serde(default)]
    pub duration_secs: Option<f64>,
    /// Multi-stage spike/stepped load schedule. Overrides `duration_secs` and `concurrency` when non-empty.
    #[serde(default)]
    pub stages: Vec<LoadStage>,
    /// Target throughput shaping cap in requests per second (JMeter Constant Throughput Timer).
    #[serde(default)]
    pub target_rps: Option<f64>,
    /// Jittered think-time distribution (Uniform or Gaussian) between requests.
    #[serde(default)]
    pub think_time: ThinkTimeConfig,
    /// Optional Synchronizing Timer (Rendezvous Barrier) releasing N VUs simultaneously.
    #[serde(default)]
    pub sync_barrier: Option<SynchronizingTimerConfig>,
    /// Enable Throughput / Weighted Controller selection across non-once-only requests.
    #[serde(default)]
    pub weighted_routing: bool,
    /// Optional CSV / JSON Data Feeder parameterization config.
    #[serde(default)]
    pub data_feeder: Option<DataFeederConfig>,
    /// Global per-sample Duration & Size assertions applied across all samples.
    #[serde(default)]
    pub sample_assertions: PerformanceAssertionConfig,
    /// Optional Global SLA threshold gates (`p95`, `p99`, `error_rate`).
    #[serde(default)]
    pub sla_thresholds: Option<SlaThresholds>,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            total_iterations: 10,
            concurrency: 2,
            delay_ms: 0,
            ramp_up_secs: 0.0,
            ramp_down_secs: 0.0,
            duration_secs: None,
            stages: Vec::new(),
            target_rps: None,
            think_time: ThinkTimeConfig::None,
            sync_barrier: None,
            weighted_routing: false,
            data_feeder: None,
            sample_assertions: PerformanceAssertionConfig::default(),
            sla_thresholds: None,
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
    #[serde(default)]
    pub active_vus: usize,
    #[serde(default)]
    pub elapsed_ms: u64,
    #[serde(default)]
    pub size_bytes: usize,
}

/// Statistical HDR Latency Profiler & summary metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(default)]
    pub peak_vus: usize,
    #[serde(default)]
    pub target_rps: Option<f64>,
    #[serde(default = "default_sla_passed")]
    pub sla_passed: bool,
    #[serde(default)]
    pub sla_violations: Vec<SlaViolation>,
    pub execution_log: Vec<RunnerExecutionRecord>,
}

fn default_sla_passed() -> bool {
    true
}

impl Default for RunnerReport {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            error_rate_percent: 0.0,
            requests_per_second: 0.0,
            min_latency_ms: 0,
            mean_latency_ms: 0.0,
            p50_latency_ms: 0,
            p90_latency_ms: 0,
            p95_latency_ms: 0,
            p99_latency_ms: 0,
            max_latency_ms: 0,
            total_duration_ms: 0,
            peak_vus: 0,
            target_rps: None,
            sla_passed: true,
            sla_violations: Vec::new(),
            execution_log: Vec::new(),
        }
    }
}

impl RunnerReport {
    pub fn evaluate_sla(&self, sla: &SlaThresholds) -> Vec<SlaViolation> {
        let mut violations = Vec::new();
        if let Some(max_p95) = sla.max_p95_ms {
            if self.p95_latency_ms > max_p95 {
                violations.push(SlaViolation {
                    metric: "p95 Latency".to_string(),
                    actual: format!("{} ms", self.p95_latency_ms),
                    threshold: format!("<= {max_p95} ms"),
                });
            }
        }
        if let Some(max_p99) = sla.max_p99_ms {
            if self.p99_latency_ms > max_p99 {
                violations.push(SlaViolation {
                    metric: "p99 Latency".to_string(),
                    actual: format!("{} ms", self.p99_latency_ms),
                    threshold: format!("<= {max_p99} ms"),
                });
            }
        }
        if let Some(max_err) = sla.max_error_rate_percent {
            if self.error_rate_percent > max_err {
                violations.push(SlaViolation {
                    metric: "Error Rate".to_string(),
                    actual: format!("{:.2}%", self.error_rate_percent),
                    threshold: format!("<= {max_err:.2}%"),
                });
            }
        }
        violations
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedSchedule {
    pub max_vus: usize,
    pub is_time_boxed: bool,
    pub total_duration: Option<Duration>,
    stages: Vec<LoadStage>,
    ramp_up_secs: f64,
    ramp_down_secs: f64,
}

impl ResolvedSchedule {
    pub fn from_config(cfg: &RunnerConfig) -> Self {
        if !cfg.stages.is_empty() {
            let max_vus = cfg
                .stages
                .iter()
                .map(|s| s.target_vus)
                .max()
                .unwrap_or(1)
                .max(1);
            let total_secs: f64 = cfg
                .stages
                .iter()
                .map(|s| s.duration_secs.max(0.01))
                .sum();
            return Self {
                max_vus,
                is_time_boxed: true,
                total_duration: Some(Duration::from_secs_f64(total_secs.max(0.01))),
                stages: cfg.stages.clone(),
                ramp_up_secs: 0.0,
                ramp_down_secs: 0.0,
            };
        }

        let max_vus = cfg.concurrency.max(1);
        if let Some(dur) = cfg.duration_secs {
            let total_secs = dur.max(0.01);
            let raw_up = cfg.ramp_up_secs.max(0.0);
            let raw_down = cfg.ramp_down_secs.max(0.0);
            let (ramp_up_secs, ramp_down_secs) = if raw_up + raw_down > total_secs {
                let scale = total_secs / (raw_up + raw_down);
                (raw_up * scale, raw_down * scale)
            } else {
                (raw_up, raw_down)
            };
            return Self {
                max_vus,
                is_time_boxed: true,
                total_duration: Some(Duration::from_secs_f64(total_secs)),
                stages: Vec::new(),
                ramp_up_secs,
                ramp_down_secs,
            };
        }

        Self {
            max_vus,
            is_time_boxed: false,
            total_duration: None,
            stages: Vec::new(),
            ramp_up_secs: cfg.ramp_up_secs.max(0.0),
            ramp_down_secs: 0.0,
        }
    }

    /// Returns target allowed VU count at `elapsed`, or `None` once time-boxed schedule has finished.
    pub fn active_vus_at(&self, elapsed: Duration) -> Option<usize> {
        let t = elapsed.as_secs_f64();
        if let Some(total) = self.total_duration {
            let total_secs = total.as_secs_f64();
            if t >= total_secs {
                return None;
            }
            if !self.stages.is_empty() {
                let mut acc = 0.0;
                for stage in &self.stages {
                    acc += stage.duration_secs.max(0.01);
                    if t < acc {
                        return Some(stage.target_vus);
                    }
                }
                return Some(self.stages.last().map(|s| s.target_vus).unwrap_or(0));
            }

            let ramp_up = self.ramp_up_secs;
            let ramp_down = self.ramp_down_secs;
            let hold_end = (total_secs - ramp_down).max(ramp_up);

            if ramp_up > 0.0 && t < ramp_up {
                let frac = (t / ramp_up).clamp(0.0, 1.0);
                let active = ((self.max_vus as f64) * frac).ceil() as usize;
                return Some(active.clamp(1, self.max_vus));
            }
            if ramp_down > 0.0 && t >= hold_end {
                let rem = ((total_secs - t) / ramp_down).clamp(0.0, 1.0);
                let active = ((self.max_vus as f64) * rem).ceil() as usize;
                return Some(active.min(self.max_vus));
            }
            return Some(self.max_vus);
        }

        if self.ramp_up_secs > 0.0 && t < self.ramp_up_secs {
            let frac = (t / self.ramp_up_secs).clamp(0.0, 1.0);
            let active = ((self.max_vus as f64) * frac).ceil() as usize;
            Some(active.clamp(1, self.max_vus))
        } else {
            Some(self.max_vus)
        }
    }
}

/// Token-slot throughput pacer enforcing JMeter Constant Throughput Timer semantics.
struct ThroughputLimiter {
    next_slot: tokio::sync::Mutex<Instant>,
    interval: Duration,
}

impl ThroughputLimiter {
    fn new(rps: f64, start: Instant) -> Self {
        let clamped_rps = rps.max(0.05);
        Self {
            next_slot: tokio::sync::Mutex::new(start),
            interval: Duration::from_secs_f64(1.0 / clamped_rps),
        }
    }

    async fn acquire(&self, deadline: Option<Instant>, cancel: &CancellationToken) -> bool {
        let wait_until = {
            let mut guard = self.next_slot.lock().await;
            let now = Instant::now();
            let slot = if *guard <= now { now } else { *guard };
            *guard = slot + self.interval;
            slot
        };

        if let Some(dl) = deadline {
            if wait_until >= dl {
                return false;
            }
        }

        let now = Instant::now();
        if wait_until > now {
            let sleep_dur = wait_until - now;
            tokio::select! {
                _ = tokio::time::sleep(sleep_dur) => {}
                _ = cancel.cancelled() => return false,
            }
        }

        if cancel.is_cancelled() {
            return false;
        }
        if let Some(dl) = deadline {
            if Instant::now() >= dl {
                return false;
            }
        }
        true
    }
}

struct BarrierInner {
    waiting: usize,
    generation: u64,
}

/// Deadlock-free generation-based Synchronizing Timer / Rendezvous Barrier.
struct RendezvousBarrier {
    group_size: usize,
    timeout: Duration,
    state: parking_lot::Mutex<BarrierInner>,
    tx: watch::Sender<u64>,
}

impl RendezvousBarrier {
    fn new(cfg: SynchronizingTimerConfig) -> Self {
        let (tx, _) = watch::channel(0u64);
        Self {
            group_size: cfg.group_size.max(2),
            timeout: Duration::from_millis(cfg.timeout_ms.max(50)),
            state: parking_lot::Mutex::new(BarrierInner {
                waiting: 0,
                generation: 0,
            }),
            tx,
        }
    }

    /// Waits until `group_size` workers gather at the barrier or `timeout`/`cancel` fires.
    async fn wait(&self, cancel: &CancellationToken) -> bool {
        let mut rx = self.tx.subscribe();
        let target_gen = {
            let mut st = self.state.lock();
            st.waiting += 1;
            let my_gen = st.generation;
            if st.waiting >= self.group_size {
                st.waiting = 0;
                st.generation = my_gen + 1;
                let _ = self.tx.send(st.generation);
                return true;
            }
            my_gen + 1
        };

        let deadline = tokio::time::Instant::now() + self.timeout;
        loop {
            if *rx.borrow() >= target_gen {
                return true;
            }
            tokio::select! {
                changed = rx.changed() => {
                    if changed.is_err() || *rx.borrow() >= target_gen {
                        return true;
                    }
                }
                _ = tokio::time::sleep_until(deadline) => {
                    let mut st = self.state.lock();
                    if st.generation < target_gen && st.waiting > 0 {
                        st.waiting -= 1;
                    }
                    return true;
                }
                _ = cancel.cancelled() => {
                    let mut st = self.state.lock();
                    if st.generation < target_gen && st.waiting > 0 {
                        st.waiting -= 1;
                    }
                    return false;
                }
            }
        }
    }
}

/// Execute a collection with worker pool, HDR metrics, and Postman/Bruno-compatible
/// `setNextRequest` flow control and variable/cookie chaining across steps in each iteration.
pub async fn run_collection(
    requests: Vec<AuraRequest>,
    context: TemplateContext,
    config: RunnerConfig,
    log_tx: Option<mpsc::UnboundedSender<RunnerExecutionRecord>>,
) -> RunnerReport {
    run_collection_with_options(requests, context, ExecutionOptions::default(), config, log_tx)
        .await
}

pub async fn run_collection_with_options(
    requests: Vec<AuraRequest>,
    context: TemplateContext,
    base_options: ExecutionOptions,
    config: RunnerConfig,
    log_tx: Option<mpsc::UnboundedSender<RunnerExecutionRecord>>,
) -> RunnerReport {
    run_collection_with_cancel(
        requests,
        context,
        base_options,
        config,
        log_tx,
        CancellationToken::new(),
    )
    .await
}

pub async fn run_collection_with_cancel(
    requests: Vec<AuraRequest>,
    context: TemplateContext,
    base_options: ExecutionOptions,
    config: RunnerConfig,
    log_tx: Option<mpsc::UnboundedSender<RunnerExecutionRecord>>,
    cancel: CancellationToken,
) -> RunnerReport {
    let schedule = ResolvedSchedule::from_config(&config);
    if requests.is_empty() || (!schedule.is_time_boxed && config.total_iterations == 0) {
        return RunnerReport::default();
    }

    let max_vus = schedule.max_vus.max(1);
    let start_overall = Instant::now();
    let deadline = schedule.total_duration.map(|d| start_overall + d);

    let limiter = config
        .target_rps
        .filter(|r| *r > 0.0)
        .map(|rps| Arc::new(ThroughputLimiter::new(rps, start_overall)));
    let barrier = config
        .sync_barrier
        .filter(|b| b.group_size >= 2)
        .map(|b| Arc::new(RendezvousBarrier::new(b)));

    let has_any_weight = requests
        .iter()
        .any(|r| r.flow.as_ref().and_then(|f| f.weight_percent).is_some());
    let use_weighted = config.weighted_routing || has_any_weight;

    let shared_requests = Arc::new(requests);
    let feeder = load_feeder_async(config.data_feeder.as_ref()).await;
    let iteration_counter = Arc::new(AtomicUsize::new(0));
    let peak_vus = Arc::new(AtomicUsize::new(0));
    let records = Arc::new(parking_lot::Mutex::new(Vec::new()));

    let mut handles = Vec::with_capacity(max_vus);

    for vu_idx in 0..max_vus {
        let reqs = Arc::clone(&shared_requests);
        let worker_feeder = feeder.clone();
        let counter = Arc::clone(&iteration_counter);
        let peak_ref = Arc::clone(&peak_vus);
        let recs = Arc::clone(&records);
        let tx = log_tx.clone();
        let ctx = context.clone();
        let worker_base_opts = base_options.clone();
        let total = config.total_iterations;
        let delay = config.delay_ms;
        let worker_think = config.think_time;
        let worker_perf = config.sample_assertions;
        let worker_sched = schedule.clone();
        let worker_limiter = limiter.clone();
        let worker_barrier = barrier.clone();
        let worker_cancel = cancel.clone();
        let max_chain_steps = reqs.len().saturating_mul(10).max(50);

        let handle = tokio::spawn(async move {
            // Persisted across iterations for this VU so Once-Only (`once_per_run`) setup shares tokens/cookies
            let mut vu_once_done: HashSet<usize> = HashSet::new();
            let mut vu_once_vars: HashMap<String, String> = HashMap::new();
            let mut vu_once_jar: CookieJar = worker_base_opts.cookie_jar.clone();
            let mut vu_data_cursor: usize = 0;

            loop {
                if worker_cancel.is_cancelled() {
                    break;
                }

                // Wait until schedule activates this VU slot or schedule finishes
                let active_vus_now = loop {
                    if worker_cancel.is_cancelled() {
                        return;
                    }
                    let elapsed = start_overall.elapsed();
                    match worker_sched.active_vus_at(elapsed) {
                        None => return,
                        Some(allowed) => {
                            if vu_idx < allowed {
                                break allowed;
                            }
                            if !worker_sched.is_time_boxed
                                && counter.load(Ordering::SeqCst) >= total
                            {
                                return;
                            }
                            tokio::select! {
                                _ = tokio::time::sleep(Duration::from_millis(20)) => {}
                                _ = worker_cancel.cancelled() => return,
                            }
                        }
                    }
                };

                peak_ref.fetch_max(active_vus_now, Ordering::Relaxed);

                // Pull next parameterized row BEFORE consuming iteration if feeder stops VU on EOF
                let data_row = if let Some(ref f) = worker_feeder {
                    match f.next_row(&mut vu_data_cursor) {
                        Some(row) => Some(row),
                        None => break,
                    }
                } else {
                    None
                };

                let current_iter = counter.fetch_add(1, Ordering::SeqCst);
                if !worker_sched.is_time_boxed && current_iter >= total {
                    break;
                }

                let mut iter_ctx = ctx.clone();
                if let Some(row) = data_row {
                    for (k, v) in row {
                        iter_ctx.request_vars.insert(k, v);
                    }
                }
                for (k, v) in &vu_once_vars {
                    iter_ctx.request_vars.insert(k.clone(), v.clone());
                }
                let mut iter_jar: CookieJar = vu_once_jar.clone();
                let mut last_response: Option<AuraResponse> = None;

                // Determine execution sequence for this iteration:
                // 1) Run any unexecuted `once_per_run` setup requests once per VU thread.
                // 2) If weighted routing is enabled across main requests, pick 1 weighted endpoint;
                //    otherwise walk sequential cursor with `setNextRequest` / `If` / `While` / `Loop`.
                if use_weighted {
                    let mut once_queue = Vec::new();
                    let mut weighted_indices = Vec::new();
                    let mut weighted_weights = Vec::new();

                    for (idx, r) in reqs.iter().enumerate() {
                        let is_once = r.flow.as_ref().map(|f| f.once_per_run).unwrap_or(false);
                        if is_once {
                            if !vu_once_done.contains(&idx) {
                                once_queue.push(idx);
                            }
                        } else {
                            weighted_indices.push(idx);
                            let w = r
                                .flow
                                .as_ref()
                                .and_then(|f| f.weight_percent)
                                .unwrap_or(1)
                                .max(1) as u32;
                            weighted_weights.push(w);
                        }
                    }

                    let mut exec_queue = once_queue;
                    if let Some(chosen) = pick_weighted_index(&weighted_weights) {
                        exec_queue.push(weighted_indices[chosen]);
                    }

                    for idx in exec_queue {
                        if !run_single_controller_step(
                            idx,
                            &reqs[idx],
                            true,
                            current_iter,
                            start_overall,
                            deadline,
                            delay,
                            worker_think,
                            worker_perf,
                            &worker_sched,
                            vu_idx,
                            &peak_ref,
                            &worker_limiter,
                            &worker_barrier,
                            &worker_cancel,
                            &worker_base_opts,
                            &mut iter_ctx,
                            &mut iter_jar,
                            &mut vu_once_done,
                            &mut vu_once_vars,
                            &mut vu_once_jar,
                            &mut last_response,
                            &recs,
                            &tx,
                        )
                        .await
                        {
                            break;
                        }
                    }
                    continue;
                }

                let mut cursor = 0usize;
                let mut steps = 0usize;
                while cursor < reqs.len() && steps < max_chain_steps {
                    if worker_cancel.is_cancelled() {
                        break;
                    }
                    let is_once = reqs[cursor]
                        .flow
                        .as_ref()
                        .map(|f| f.once_per_run)
                        .unwrap_or(false);
                    if is_once && vu_once_done.contains(&cursor) {
                        cursor += 1;
                        continue;
                    }

                    steps += 1;
                    let keep_going = run_single_controller_step(
                        cursor,
                        &reqs[cursor],
                        false,
                        current_iter,
                        start_overall,
                        deadline,
                        delay,
                        worker_think,
                        worker_perf,
                        &worker_sched,
                        vu_idx,
                        &peak_ref,
                        &worker_limiter,
                        &worker_barrier,
                        &worker_cancel,
                        &worker_base_opts,
                        &mut iter_ctx,
                        &mut iter_jar,
                        &mut vu_once_done,
                        &mut vu_once_vars,
                        &mut vu_once_jar,
                        &mut last_response,
                        &recs,
                        &tx,
                    )
                    .await;

                    if !keep_going {
                        break;
                    }

                    let (stop_chain, next_target) = last_response
                        .as_ref()
                        .map(|r| (r.stop_chain, r.next_request.clone()))
                        .unwrap_or((false, None));

                    if stop_chain {
                        break;
                    }

                    if let Some(target_name) = next_target {
                        let target_trim = target_name.trim();
                        if let Some(next_idx) = reqs.iter().position(|r| {
                            r.id == target_trim || r.name.eq_ignore_ascii_case(target_trim)
                        }) {
                            cursor = next_idx;
                        } else {
                            tracing::warn!(
                                target: "aura_engine::runner",
                                requested = %target_trim,
                                "setNextRequest target not found in collection; stopping chain"
                            );
                            break;
                        }
                    } else {
                        cursor += 1;
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
    let records_list = records.lock().clone();
    compute_report(
        records_list,
        total_duration,
        peak_vus.load(Ordering::Relaxed),
        config.target_rps,
        config.sla_thresholds,
    )
}

async fn load_feeder_async(cfg_opt: Option<&DataFeederConfig>) -> Option<Arc<DataFeeder>> {
    let cfg = cfg_opt?;
    if !cfg.enabled {
        return None;
    }
    if !cfg.inline_rows.is_empty() {
        return Some(Arc::new(DataFeeder::from_rows(
            cfg.inline_rows.clone(),
            cfg.sharing_mode,
            cfg.eof_behavior,
        )));
    }
    let path_str = cfg.file_path.trim();
    if path_str.is_empty() {
        return None;
    }
    match tokio::fs::read_to_string(path_str).await {
        Ok(raw) => match parse_data_content(&raw, Path::new(path_str), cfg.delimiter) {
            Ok(rows) => Some(Arc::new(DataFeeder::from_rows(
                rows,
                cfg.sharing_mode,
                cfg.eof_behavior,
            ))),
            Err(e) => {
                tracing::error!(target: "aura_engine::runner", error = %e, "Failed to parse data feeder");
                None
            }
        },
        Err(e) => {
            tracing::error!(target: "aura_engine::runner", path = %path_str, error = %e, "Failed to read data feeder file");
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_single_controller_step(
    req_idx: usize,
    template_req: &AuraRequest,
    _is_weighted: bool,
    current_iter: usize,
    start_overall: Instant,
    deadline: Option<Instant>,
    delay_ms: u64,
    runner_think: ThinkTimeConfig,
    runner_perf: PerformanceAssertionConfig,
    schedule: &ResolvedSchedule,
    vu_idx: usize,
    peak_ref: &Arc<AtomicUsize>,
    limiter: &Option<Arc<ThroughputLimiter>>,
    barrier: &Option<Arc<RendezvousBarrier>>,
    cancel: &CancellationToken,
    base_opts: &ExecutionOptions,
    iter_ctx: &mut TemplateContext,
    iter_jar: &mut CookieJar,
    vu_once_done: &mut HashSet<usize>,
    vu_once_vars: &mut HashMap<String, String>,
    vu_once_jar: &mut CookieJar,
    last_response: &mut Option<AuraResponse>,
    recs: &Arc<parking_lot::Mutex<Vec<RunnerExecutionRecord>>>,
    tx: &Option<mpsc::UnboundedSender<RunnerExecutionRecord>>,
) -> bool {
    let flow_opt = template_req.flow.as_ref();

    // Conditional `If` controller check before executing
    if let Some(cond_expr) = flow_opt.and_then(|f| f.condition_if.as_deref()) {
        if !cond_expr.trim().is_empty() {
            let merged_vars = merge_context_vars(iter_ctx);
            if !evaluate_flow_condition(cond_expr, last_response.as_ref(), &merged_vars) {
                return true;
            }
        }
    }

    let loop_count = flow_opt.and_then(|f| f.loop_count).unwrap_or(1).max(1);
    let while_expr = flow_opt.and_then(|f| f.loop_while.as_deref());
    let max_while = flow_opt.map(|f| f.max_loops.max(1)).unwrap_or(10);
    let is_once = flow_opt.map(|f| f.once_per_run).unwrap_or(false);

    for _loop_rep in 0..loop_count {
        let mut while_iters = 0u32;
        loop {
            if cancel.is_cancelled() {
                return false;
            }
            let current_active = match schedule.active_vus_at(start_overall.elapsed()) {
                None => return false,
                Some(allowed) => {
                    if vu_idx >= allowed {
                        return false;
                    }
                    peak_ref.fetch_max(allowed, Ordering::Relaxed);
                    allowed
                }
            };

            if let Some(ref b) = barrier {
                if !b.wait(cancel).await {
                    return false;
                }
            }

            if let Some(ref lim) = limiter {
                if !lim.acquire(deadline, cancel).await {
                    return false;
                }
            }

            let mut req_clone = template_req.clone();
            let mut opts = base_opts.clone();
            opts.cookie_jar = iter_jar.clone();

            let res: Result<AuraResponse, anyhow::Error> =
                execute_request_with_options(&mut req_clone, iter_ctx, &opts).await;

            let (status, latency, size_bytes, passed, err_msg, resp_opt) = match res {
                Ok(mut resp) => {
                    if !runner_perf.is_empty() {
                        let mut sample_perf = evaluate_performance_assertions(&runner_perf, &resp);
                        resp.assertions.append(&mut sample_perf);
                    }
                    let failed_msg = resp
                        .assertions
                        .iter()
                        .find(|a| !a.passed)
                        .and_then(|a| a.message.clone().or_else(|| Some(a.name.clone())));
                    let pass = resp.status < 400 && resp.assertions.iter().all(|a| a.passed);

                    for (k, v) in &resp.exported_variables {
                        iter_ctx.request_vars.insert(k.clone(), v.clone());
                        if is_once {
                            vu_once_vars.insert(k.clone(), v.clone());
                        }
                    }
                    let now = Utc::now();
                    for c in &resp.captured_cookies {
                        iter_jar.set_cookie(c.clone(), now);
                        if is_once {
                            vu_once_jar.set_cookie(c.clone(), now);
                        }
                    }

                    (
                        resp.status,
                        resp.duration_ms,
                        resp.size_bytes,
                        pass,
                        failed_msg,
                        Some(resp),
                    )
                }
                Err(e) => (0, 0, 0, false, Some(e.to_string()), None),
            };

            if is_once {
                vu_once_done.insert(req_idx);
            }

            if let Some(r) = resp_opt {
                *last_response = Some(r);
            }

            let record = RunnerExecutionRecord {
                iteration: current_iter + 1,
                request_name: req_clone.name.clone(),
                method: req_clone.method.as_str().to_string(),
                status,
                latency_ms: latency,
                passed,
                error: err_msg,
                active_vus: current_active,
                elapsed_ms: start_overall.elapsed().as_millis() as u64,
                size_bytes,
            };

            recs.lock().push(record.clone());
            if let Some(ref sender) = tx {
                let _ = sender.send(record);
            }

            // Think-Time Pacing: combine fixed delay_ms, runner ThinkTimeConfig (Uniform/Gaussian),
            // and per-request `[flow]` pacing delay.
            let mut total_wait = Duration::from_millis(delay_ms);
            if let Some(d) = runner_think.sample_duration() {
                total_wait += d;
            }
            if let Some(d) = flow_opt.and_then(|f| f.calculate_pacing_delay()) {
                total_wait += d;
            }
            if total_wait > Duration::ZERO {
                if let Some(dl) = deadline {
                    let now = Instant::now();
                    if now >= dl {
                        return false;
                    }
                    let rem = dl - now;
                    total_wait = total_wait.min(rem);
                }
                if total_wait > Duration::ZERO {
                    tokio::select! {
                        _ = tokio::time::sleep(total_wait) => {}
                        _ = cancel.cancelled() => return false,
                    }
                }
            }

            while_iters += 1;
            let should_continue_while = if let Some(w_expr) = while_expr {
                if !w_expr.trim().is_empty() && while_iters < max_while {
                    let merged_vars = merge_context_vars(iter_ctx);
                    evaluate_flow_condition(w_expr, last_response.as_ref(), &merged_vars)
                } else {
                    false
                }
            } else {
                false
            };

            if !should_continue_while {
                break;
            }
        }
    }

    true
}

fn merge_context_vars(ctx: &TemplateContext) -> HashMap<String, String> {
    let mut merged = ctx.collection_vars.clone();
    merged.extend(ctx.environment_vars.clone());
    merged.extend(ctx.request_vars.clone());
    merged
}

fn compute_report(
    records: Vec<RunnerExecutionRecord>,
    total_duration: Duration,
    peak_vus: usize,
    target_rps: Option<f64>,
    sla_thresholds: Option<SlaThresholds>,
) -> RunnerReport {
    let total_requests = records.len();
    if total_requests == 0 {
        return RunnerReport {
            peak_vus,
            target_rps,
            ..Default::default()
        };
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

    let mut report = RunnerReport {
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
        peak_vus: peak_vus.max(1),
        target_rps,
        sla_passed: true,
        sla_violations: Vec::new(),
        execution_log: records,
    };

    if let Some(sla) = sla_thresholds {
        let violations = report.evaluate_sla(&sla);
        report.sla_passed = violations.is_empty();
        report.sla_violations = violations;
    }

    report
}
