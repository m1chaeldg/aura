//! Load-runner flow control, think-time distributions, and weighted routing.

use serde::{Deserialize, Serialize};

/// JMeter-style think-time distribution for jittered pacing between requests.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum ThinkTimeConfig {
    #[default]
    None,
    /// Uniform random jitter between `min_ms..=max_ms`.
    Uniform { min_ms: u64, max_ms: u64 },
    /// Gaussian (Normal) distributed think-time `mean_ms ± stddev_ms` (clamped >= 0).
    Gaussian { mean_ms: f64, stddev_ms: f64 },
}

impl ThinkTimeConfig {
    pub fn sample_duration(&self) -> Option<std::time::Duration> {
        use rand::Rng;
        let ms: f64 = match *self {
            Self::None => return None,
            Self::Uniform { min_ms, max_ms } => {
                if max_ms == 0 && min_ms == 0 {
                    return None;
                }
                let lo = min_ms.min(max_ms);
                let hi = min_ms.max(max_ms);
                if lo == hi {
                    lo as f64
                } else {
                    rand::thread_rng().gen_range(lo..=hi) as f64
                }
            }
            Self::Gaussian { mean_ms, stddev_ms } => {
                if mean_ms <= 0.0 && stddev_ms <= 0.0 {
                    return None;
                }
                let mut rng = rand::thread_rng();
                // Box-Muller transform for standard normal variate Z ~ N(0, 1)
                let u1: f64 = rng.gen_range(1e-12_f64..=1.0_f64);
                let u2: f64 = rng.gen_range(0.0_f64..1.0_f64);
                let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                (mean_ms + stddev_ms.abs() * z).max(0.0)
            }
        };
        if ms > 0.5 {
            Some(std::time::Duration::from_millis(ms.round() as u64))
        } else {
            None
        }
    }
}

fn default_max_loops() -> u32 {
    10
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowControl {
    pub once_per_run: bool,
    pub retry_until: Option<String>,
    pub retry_interval_ms: u64,
    pub max_retries: u32,
    pub delay_ms: Option<u64>,
    pub jitter_min_ms: Option<u64>,
    pub jitter_max_ms: Option<u64>,
    /// Optional Gaussian timer mean (ms).
    #[serde(default)]
    pub gaussian_mean_ms: Option<u64>,
    /// Optional Gaussian timer standard deviation (ms).
    #[serde(default)]
    pub gaussian_stddev_ms: Option<u64>,
    /// Relative percentage weight for Throughput / Weighted Controller distribution.
    #[serde(default)]
    pub weight_percent: Option<u8>,
    /// Loop Controller repeat count for this request (`>= 1`).
    #[serde(default)]
    pub loop_count: Option<u32>,
    /// Conditional `If` controller expression (evaluated before sending request).
    #[serde(default)]
    pub condition_if: Option<String>,
    /// Conditional `While` controller expression (loops after each response while true).
    #[serde(default)]
    pub loop_while: Option<String>,
    /// Safety cap for `loop_while` iterations (defaults to 10).
    #[serde(default = "default_max_loops")]
    pub max_loops: u32,
    /// Per-request performance bounds (`max_duration_ms`, `min_size_bytes`, `max_size_bytes`).
    #[serde(default)]
    pub max_duration_ms: Option<u64>,
    #[serde(default)]
    pub min_size_bytes: Option<usize>,
    #[serde(default)]
    pub max_size_bytes: Option<usize>,
}

impl Default for FlowControl {
    fn default() -> Self {
        Self {
            once_per_run: false,
            retry_until: None,
            retry_interval_ms: 1000,
            max_retries: 3,
            delay_ms: None,
            jitter_min_ms: None,
            jitter_max_ms: None,
            gaussian_mean_ms: None,
            gaussian_stddev_ms: None,
            weight_percent: None,
            loop_count: None,
            condition_if: None,
            loop_while: None,
            max_loops: default_max_loops(),
            max_duration_ms: None,
            min_size_bytes: None,
            max_size_bytes: None,
        }
    }
}

impl FlowControl {
    pub fn calculate_pacing_delay(&self) -> Option<std::time::Duration> {
        let mut total_ms = self.delay_ms.unwrap_or(0);
        if let (Some(min), Some(max)) = (self.jitter_min_ms, self.jitter_max_ms) {
            if let Some(d) = (ThinkTimeConfig::Uniform {
                min_ms: min,
                max_ms: max,
            })
            .sample_duration()
            {
                total_ms += d.as_millis() as u64;
            }
        }
        if let Some(mean) = self.gaussian_mean_ms {
            let stddev = self.gaussian_stddev_ms.unwrap_or(0);
            if let Some(d) = (ThinkTimeConfig::Gaussian {
                mean_ms: mean as f64,
                stddev_ms: stddev as f64,
            })
            .sample_duration()
            {
                total_ms += d.as_millis() as u64;
            }
        }
        if total_ms > 0 {
            Some(std::time::Duration::from_millis(total_ms))
        } else {
            None
        }
    }
}

/// Samples a slice index proportional to positive weights (for Weighted / Throughput Controller).
pub fn pick_weighted_index(weights: &[u32]) -> Option<usize> {
    let total: u64 = weights.iter().map(|&w| w as u64).sum();
    if total == 0 {
        return None;
    }
    use rand::Rng;
    let mut roll = rand::thread_rng().gen_range(0..total);
    for (idx, &w) in weights.iter().enumerate() {
        let w64 = w as u64;
        if roll < w64 {
            return Some(idx);
        }
        roll -= w64;
    }
    Some(weights.len().saturating_sub(1))
}
