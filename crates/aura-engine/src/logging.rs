//! Asynchronous structured logging subsystem for Aura.
//! Zero telemetry. Writes rolling logs to local disk and broadcasts
//! real-time events to the UI viewer.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: LogLevel,
    pub subsystem: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogLevel {
    TRACE,
    DEBUG,
    INFO,
    WARN,
    ERROR,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TRACE => "TRACE",
            Self::DEBUG => "DEBUG",
            Self::INFO => "INFO",
            Self::WARN => "WARN",
            Self::ERROR => "ERROR",
        }
    }

    pub fn color_hex(&self) -> &'static str {
        match self {
            Self::TRACE => aura_core::design::colors::LOG_TRACE,
            Self::DEBUG => aura_core::design::colors::LOG_DEBUG,
            Self::INFO => aura_core::design::colors::LOG_INFO,
            Self::WARN => aura_core::design::colors::LOG_WARN,
            Self::ERROR => aura_core::design::colors::LOG_ERROR,
        }
    }
}

/// Ring buffer broadcaster for real-time UI log tailing
#[derive(Debug, Clone)]
pub struct LogBroadcaster {
    max_entries: usize,
    buffer: Arc<RwLock<VecDeque<LogEntry>>>,
    log_dir: PathBuf,
}

impl LogBroadcaster {
    pub fn new(max_entries: usize, log_dir: impl AsRef<Path>) -> Self {
        let dir = log_dir.as_ref().to_path_buf();
        let _ = std::fs::create_dir_all(&dir);

        Self {
            max_entries,
            buffer: Arc::new(RwLock::new(VecDeque::with_capacity(max_entries))),
            log_dir: dir,
        }
    }

    pub fn log(&self, level: LogLevel, subsystem: impl Into<String>, message: impl Into<String>) {
        let entry = LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            level,
            subsystem: subsystem.into(),
            message: message.into(),
        };

        // Append to in-memory ring buffer
        {
            let mut buf = self.buffer.write();
            if buf.len() >= self.max_entries {
                buf.pop_front();
            }
            buf.push_back(entry.clone());
        }

        // Also append line to aura.log file
        let log_file = self.log_dir.join("aura.log");
        let line = format!(
            "{} [{}] [{}] {}\n",
            entry.timestamp.to_rfc3339(),
            entry.level.as_str(),
            entry.subsystem,
            entry.message
        );
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
            .and_then(|mut f| {
                use std::io::Write;
                f.write_all(line.as_bytes())
            });
    }

    pub fn get_entries(&self) -> Vec<LogEntry> {
        self.buffer.read().iter().cloned().collect()
    }

    pub fn filter_entries(
        &self,
        min_level: Option<LogLevel>,
        subsystem: Option<&str>,
        search_query: Option<&str>,
    ) -> Vec<LogEntry> {
        let buf = self.buffer.read();
        buf.iter()
            .filter(|e| {
                if let Some(min_lvl) = min_level {
                    if e.level < min_lvl {
                        return false;
                    }
                }
                if let Some(sub) = subsystem {
                    if sub != "All" && !e.subsystem.eq_ignore_ascii_case(sub) {
                        return false;
                    }
                }
                if let Some(q) = search_query {
                    if !q.is_empty() && !e.message.to_lowercase().contains(&q.to_lowercase()) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    pub fn clear(&self) {
        self.buffer.write().clear();
    }
}
