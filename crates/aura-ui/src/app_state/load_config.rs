//! Load-runner configuration builder and per-request flow-control setters on `AuraAppState`.

use super::{AuraAppState, LoadThinkMode};

impl AuraAppState {
    pub fn build_load_runner_config(&self) -> aura_engine::RunnerConfig {
        use aura_core::model::ThinkTimeConfig;
        use aura_engine::{LoadProfileMode, LoadStage, RunnerConfig, SynchronizingTimerConfig};

        let total_iterations = self
            .load_iterations_str
            .trim()
            .parse::<usize>()
            .unwrap_or(10)
            .max(1);
        let concurrency = self
            .load_concurrency_str
            .trim()
            .parse::<usize>()
            .unwrap_or(2)
            .max(1);
        let delay_ms = self.load_delay_ms_str.trim().parse::<u64>().unwrap_or(0);
        let target_rps = if self.load_target_rps_enabled {
            self.load_target_rps_str
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|v| *v > 0.0)
        } else {
            None
        };
        let think_time = match self.load_think_mode {
            LoadThinkMode::None => ThinkTimeConfig::None,
            LoadThinkMode::Uniform => ThinkTimeConfig::Uniform {
                min_ms: self.load_think_a_str.trim().parse::<u64>().unwrap_or(0),
                max_ms: self.load_think_b_str.trim().parse::<u64>().unwrap_or(0),
            },
            LoadThinkMode::Gaussian => ThinkTimeConfig::Gaussian {
                mean_ms: self.load_think_a_str.trim().parse::<f64>().unwrap_or(0.0).max(0.0),
                stddev_ms: self.load_think_b_str.trim().parse::<f64>().unwrap_or(0.0).max(0.0),
            },
        };
        let sync_barrier = if self.load_barrier_enabled {
            Some(SynchronizingTimerConfig {
                group_size: self
                    .load_barrier_size_str
                    .trim()
                    .parse::<usize>()
                    .unwrap_or(2)
                    .max(2),
                timeout_ms: self
                    .load_barrier_timeout_str
                    .trim()
                    .parse::<u64>()
                    .unwrap_or(5000)
                    .max(50),
            })
        } else {
            None
        };
        let weighted_routing = self.load_weighted_routing;
        let data_feeder = if self.load_data_enabled && !self.load_data_file_path.trim().is_empty() {
            let resolved_path = match &self.workspace_store.workspace_root {
                Some(root) => aura_sync::resolve_workspace_path(root, &self.load_data_file_path)
                    .display()
                    .to_string(),
                None => self.load_data_file_path.trim().to_string(),
            };
            Some(aura_engine::DataFeederConfig {
                enabled: true,
                file_path: resolved_path,
                sharing_mode: self.load_data_sharing,
                eof_behavior: self.load_data_eof,
                ..Default::default()
            })
        } else {
            None
        };
        let sample_assertions = aura_engine::PerformanceAssertionConfig {
            max_duration_ms: self.load_max_duration_ms_str.trim().parse::<u64>().ok().filter(|&v| v > 0),
            min_size_bytes: self.load_min_size_bytes_str.trim().parse::<usize>().ok().filter(|&v| v > 0),
            max_size_bytes: self.load_max_size_bytes_str.trim().parse::<usize>().ok().filter(|&v| v > 0),
        };
        let max_p95_ms = self.load_sla_p95_str.trim().parse::<u64>().ok().filter(|&v| v > 0);
        let max_p99_ms = self.load_sla_p99_str.trim().parse::<u64>().ok().filter(|&v| v > 0);
        let max_error_rate_percent = self.load_sla_err_str.trim().parse::<f64>().ok().filter(|&v| v >= 0.0);
        let sla_thresholds =
            if max_p95_ms.is_some() || max_p99_ms.is_some() || max_error_rate_percent.is_some() {
                Some(aura_engine::SlaThresholds {
                    max_p95_ms,
                    max_p99_ms,
                    max_error_rate_percent,
                })
            } else {
                None
            };

        match self.load_profile_mode {
            LoadProfileMode::FixedIterations => RunnerConfig {
                total_iterations,
                concurrency,
                delay_ms,
                ramp_up_secs: self
                    .load_ramp_up_str
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(0.0)
                    .max(0.0),
                ramp_down_secs: 0.0,
                duration_secs: None,
                stages: Vec::new(),
                target_rps,
                think_time,
                sync_barrier,
                weighted_routing,
                data_feeder,
                sample_assertions,
                sla_thresholds,
            },
            LoadProfileMode::Duration => RunnerConfig {
                total_iterations,
                concurrency,
                delay_ms,
                ramp_up_secs: 0.0,
                ramp_down_secs: 0.0,
                duration_secs: Some(
                    self.load_duration_str
                        .trim()
                        .parse::<f64>()
                        .unwrap_or(15.0)
                        .max(0.5),
                ),
                stages: Vec::new(),
                target_rps,
                think_time,
                sync_barrier,
                weighted_routing,
                data_feeder,
                sample_assertions,
                sla_thresholds,
            },
            LoadProfileMode::RampUpDown => RunnerConfig {
                total_iterations,
                concurrency,
                delay_ms,
                ramp_up_secs: self
                    .load_ramp_up_str
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(5.0)
                    .max(0.0),
                ramp_down_secs: self
                    .load_ramp_down_str
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(5.0)
                    .max(0.0),
                duration_secs: Some(
                    self.load_duration_str
                        .trim()
                        .parse::<f64>()
                        .unwrap_or(20.0)
                        .max(1.0),
                ),
                stages: Vec::new(),
                target_rps,
                think_time,
                sync_barrier,
                weighted_routing,
                data_feeder,
                sample_assertions,
                sla_thresholds,
            },
            LoadProfileMode::SteppedStages => {
                let stages = self
                    .load_stages
                    .iter()
                    .map(|(vus, secs)| {
                        LoadStage::new(
                            vus.trim().parse::<usize>().unwrap_or(1).max(1),
                            secs.trim().parse::<f64>().unwrap_or(5.0).max(0.1),
                        )
                    })
                    .collect();
                RunnerConfig {
                    total_iterations,
                    concurrency,
                    delay_ms,
                    ramp_up_secs: 0.0,
                    ramp_down_secs: 0.0,
                    duration_secs: None,
                    stages,
                    target_rps,
                    think_time,
                    sync_barrier,
                    weighted_routing,
                    data_feeder,
                    sample_assertions,
                    sla_thresholds,
                }
            }
        }
    }

    pub fn toggle_active_flow_once_per_run(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let flow = tab
                .request
                .flow
                .get_or_insert_with(aura_core::model::FlowControl::default);
            flow.once_per_run = !flow.once_per_run;
            tab.mark_dirty(true);
            tab.sync_spec_from_request();
        }
    }

    pub fn set_active_flow_weight_str(&mut self, text: &str) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let val = text.trim().parse::<u8>().ok().map(|v| v.clamp(1, 100));
            let flow = tab
                .request
                .flow
                .get_or_insert_with(aura_core::model::FlowControl::default);
            flow.weight_percent = val;
            tab.mark_dirty(true);
            tab.sync_spec_from_request();
        }
    }

    pub fn set_active_flow_loop_count_str(&mut self, text: &str) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let val = text.trim().parse::<u32>().ok().filter(|&v| v > 1);
            let flow = tab
                .request
                .flow
                .get_or_insert_with(aura_core::model::FlowControl::default);
            flow.loop_count = val;
            tab.mark_dirty(true);
            tab.sync_spec_from_request();
        }
    }

    pub fn set_active_flow_condition_if(&mut self, text: String) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let flow = tab
                .request
                .flow
                .get_or_insert_with(aura_core::model::FlowControl::default);
            flow.condition_if = if text.trim().is_empty() {
                None
            } else {
                Some(text)
            };
            tab.mark_dirty(true);
            tab.sync_spec_from_request();
        }
    }

    pub fn set_active_flow_loop_while(&mut self, text: String) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let flow = tab
                .request
                .flow
                .get_or_insert_with(aura_core::model::FlowControl::default);
            flow.loop_while = if text.trim().is_empty() {
                None
            } else {
                Some(text)
            };
            tab.mark_dirty(true);
            tab.sync_spec_from_request();
        }
    }
}
