//! Fast native performance bounds evaluated per sample (zero V8 overhead).

use crate::model::{AuraResponse, TestAssertion};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerformanceAssertionConfig {
    #[serde(default)]
    pub max_duration_ms: Option<u64>,
    #[serde(default)]
    pub min_size_bytes: Option<usize>,
    #[serde(default)]
    pub max_size_bytes: Option<usize>,
}

impl PerformanceAssertionConfig {
    pub fn is_empty(&self) -> bool {
        self.max_duration_ms.is_none()
            && self.min_size_bytes.is_none()
            && self.max_size_bytes.is_none()
    }
}

/// Evaluates Duration (`max_duration_ms`) and Size (`min_size_bytes..=max_size_bytes`) bounds cleanly.
pub fn evaluate_performance_assertions(
    cfg: &PerformanceAssertionConfig,
    resp: &AuraResponse,
) -> Vec<TestAssertion> {
    let mut out = Vec::new();
    if let Some(max_ms) = cfg.max_duration_ms {
        let passed = resp.duration_ms <= max_ms;
        out.push(TestAssertion {
            name: format!("Duration <= {max_ms} ms"),
            passed,
            message: (!passed).then(|| {
                format!(
                    "Response duration {} ms exceeded max {} ms",
                    resp.duration_ms, max_ms
                )
            }),
            duration_ms: 0,
        });
    }
    if let Some(min_b) = cfg.min_size_bytes {
        let passed = resp.size_bytes >= min_b;
        out.push(TestAssertion {
            name: format!("Size >= {min_b} bytes"),
            passed,
            message: (!passed).then(|| {
                format!(
                    "Response size {} bytes below minimum {} bytes",
                    resp.size_bytes, min_b
                )
            }),
            duration_ms: 0,
        });
    }
    if let Some(max_b) = cfg.max_size_bytes {
        let passed = resp.size_bytes <= max_b;
        out.push(TestAssertion {
            name: format!("Size <= {max_b} bytes"),
            passed,
            message: (!passed).then(|| {
                format!(
                    "Response size {} bytes exceeded maximum {} bytes",
                    resp.size_bytes, max_b
                )
            }),
            duration_ms: 0,
        });
    }
    out
}
