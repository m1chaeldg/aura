//! Format Service: Offloads JSON/XML pretty-printing and syntax parsing to background threads.

use anyhow::Result;

#[derive(Debug, Clone, Default)]
pub struct FormatService;

impl FormatService {
    pub fn new() -> Self {
        Self
    }

    /// Format JSON string pretty
    pub fn format_json(raw: &str) -> Result<String> {
        let val: serde_json::Value = serde_json::from_str(raw)?;
        Ok(serde_json::to_string_pretty(&val)?)
    }
}
