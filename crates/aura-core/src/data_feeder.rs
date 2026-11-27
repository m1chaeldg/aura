//! CSV / JSON Data Feeder (JMeter CSV Data Set Config equivalent).
//! Supports RFC-4180 CSV (including quoted fields & custom delimiters) and JSON array / NDJSON,
//! with AllVUs (global row-once) vs PerVU sharing and Recycle vs StopVuOnEof EOF behavior.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DataSharingMode {
    /// Each row is consumed once globally across all virtual users.
    #[default]
    AllVUs,
    /// Each virtual user starts from row 0 and advances independently.
    PerVU,
}

impl DataSharingMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::AllVUs => "All VUs (Shared)",
            Self::PerVU => "Per VU",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DataEofBehavior {
    /// Wrap around from the last row back to row 0.
    #[default]
    Recycle,
    /// Stop the virtual user / iteration loop when all rows are exhausted.
    StopVuOnEof,
}

impl DataEofBehavior {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Recycle => "Recycle from Start",
            Self::StopVuOnEof => "Stop VU on EOF",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataFeederConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub file_path: String,
    #[serde(default)]
    pub sharing_mode: DataSharingMode,
    #[serde(default)]
    pub eof_behavior: DataEofBehavior,
    #[serde(default = "default_delimiter")]
    pub delimiter: char,
    /// Optional inline pre-parsed rows (useful for tests or programmatic feeds).
    #[serde(default)]
    pub inline_rows: Vec<HashMap<String, String>>,
}

fn default_delimiter() -> char {
    ','
}

impl Default for DataFeederConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            file_path: String::new(),
            sharing_mode: DataSharingMode::AllVUs,
            eof_behavior: DataEofBehavior::Recycle,
            delimiter: ',',
            inline_rows: Vec::new(),
        }
    }
}

/// Thread-safe loaded data feeder shared across virtual user tasks.
#[derive(Debug)]
pub struct DataFeeder {
    rows: Vec<HashMap<String, String>>,
    sharing_mode: DataSharingMode,
    eof_behavior: DataEofBehavior,
    global_cursor: AtomicUsize,
}

impl DataFeeder {
    pub fn from_config(cfg: &DataFeederConfig) -> Result<Self> {
        let rows = if !cfg.inline_rows.is_empty() {
            cfg.inline_rows.clone()
        } else {
            let trimmed = cfg.file_path.trim();
            if trimmed.is_empty() {
                return Err(anyhow!("Data feeder is enabled but file_path is empty"));
            }
            load_data_rows_from_file(Path::new(trimmed), cfg.delimiter)?
        };
        Ok(Self {
            rows,
            sharing_mode: cfg.sharing_mode,
            eof_behavior: cfg.eof_behavior,
            global_cursor: AtomicUsize::new(0),
        })
    }

    pub fn from_rows(
        rows: Vec<HashMap<String, String>>,
        sharing_mode: DataSharingMode,
        eof_behavior: DataEofBehavior,
    ) -> Self {
        Self {
            rows,
            sharing_mode,
            eof_behavior,
            global_cursor: AtomicUsize::new(0),
        }
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Returns the next row for the calling VU, updating `vu_cursor` when in `PerVU` mode.
    /// Returns `None` when EOF is reached and `eof_behavior == DataEofBehavior::StopVuOnEof`.
    pub fn next_row(&self, vu_cursor: &mut usize) -> Option<HashMap<String, String>> {
        if self.rows.is_empty() {
            return None;
        }
        let idx = match self.sharing_mode {
            DataSharingMode::AllVUs => self.global_cursor.fetch_add(1, Ordering::SeqCst),
            DataSharingMode::PerVU => {
                let current = *vu_cursor;
                *vu_cursor = current.saturating_add(1);
                current
            }
        };

        if idx >= self.rows.len() {
            match self.eof_behavior {
                DataEofBehavior::Recycle => Some(self.rows[idx % self.rows.len()].clone()),
                DataEofBehavior::StopVuOnEof => None,
            }
        } else {
            Some(self.rows[idx].clone())
        }
    }
}

pub fn load_data_rows_from_file(path: &Path, delimiter: char) -> Result<Vec<HashMap<String, String>>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read data feeder file: {}", path.display()))?;
    parse_data_content(&content, path, delimiter)
}

pub fn parse_data_content(
    raw: &str,
    path_hint: &Path,
    delimiter: char,
) -> Result<Vec<HashMap<String, String>>> {
    let ext = path_hint
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let trimmed = raw.trim_start_matches('\u{feff}').trim();
    if ext == "json" || ext == "ndjson" || trimmed.starts_with('[') || trimmed.starts_with('{') {
        parse_json_rows(trimmed)
    } else {
        Ok(parse_csv_rows(trimmed, delimiter))
    }
}

pub fn parse_json_rows(raw: &str) -> Result<Vec<HashMap<String, String>>> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(arr) = val.as_array() {
            return Ok(arr.iter().filter_map(json_obj_to_string_map).collect());
        }
        if let Some(map) = json_obj_to_string_map(&val) {
            return Ok(vec![map]);
        }
        return Err(anyhow!("JSON data feeder must contain an array of objects or NDJSON lines"));
    }

    // NDJSON fallback (one JSON object per non-empty line)
    let mut rows = Vec::new();
    for line in raw.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let val: serde_json::Value = serde_json::from_str(t)
            .with_context(|| format!("Invalid JSON object in NDJSON line: {t}"))?;
        if let Some(map) = json_obj_to_string_map(&val) {
            rows.push(map);
        }
    }
    Ok(rows)
}

fn json_obj_to_string_map(val: &serde_json::Value) -> Option<HashMap<String, String>> {
    let obj = val.as_object()?;
    let mut map = HashMap::with_capacity(obj.len());
    for (k, v) in obj {
        let s = match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Null => String::new(),
            other => other.to_string(),
        };
        map.insert(k.clone(), s);
    }
    Some(map)
}

pub fn parse_csv_rows(raw: &str, delimiter: char) -> Vec<HashMap<String, String>> {
    let records = parse_csv_records(raw, delimiter);
    if records.len() < 2 {
        return Vec::new();
    }
    let headers: Vec<String> = records[0]
        .iter()
        .map(|h| h.trim().to_string())
        .collect();

    let mut out = Vec::with_capacity(records.len() - 1);
    for row in records.into_iter().skip(1) {
        if row.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        let mut map = HashMap::with_capacity(headers.len());
        for (i, header) in headers.iter().enumerate() {
            if header.is_empty() {
                continue;
            }
            let val = row.get(i).cloned().unwrap_or_default();
            map.insert(header.clone(), val);
        }
        out.push(map);
    }
    out
}

fn parse_csv_records(raw: &str, delimiter: char) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut current_row = Vec::new();
    let mut current_field = String::new();
    let mut in_quotes = false;
    let mut chars = raw.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    current_field.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                current_field.push(c);
            }
        } else if c == '"' {
            in_quotes = true;
        } else if c == delimiter {
            current_row.push(std::mem::take(&mut current_field));
        } else if c == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            current_row.push(std::mem::take(&mut current_field));
            records.push(std::mem::take(&mut current_row));
        } else if c == '\n' {
            current_row.push(std::mem::take(&mut current_field));
            records.push(std::mem::take(&mut current_row));
        } else {
            current_field.push(c);
        }
    }

    if !current_field.is_empty() || !current_row.is_empty() {
        current_row.push(current_field);
        records.push(current_row);
    }
    records
}
