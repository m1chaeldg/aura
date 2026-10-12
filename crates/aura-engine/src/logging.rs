//! Asynchronous structured logging subsystem for Aura.
//! Zero telemetry. Writes rolling logs to local disk and broadcasts
//! real-time events to the UI viewer via `LogBroadcaster`.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

static GLOBAL_BROADCASTER: OnceLock<LogBroadcaster> = OnceLock::new();

/// Returns the standard default local application logs directory.
pub fn default_log_dir() -> PathBuf {
    dirs::data_local_dir()
        .map(|d| d.join("aura").join("logs"))
        .unwrap_or_else(|| PathBuf::from("logs"))
}

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
    /// Retrieve the singleton global log broadcaster instance.
    pub fn global() -> &'static LogBroadcaster {
        GLOBAL_BROADCASTER.get_or_init(|| Self::new(1000, default_log_dir()))
    }

    /// Retrieve or initialize the singleton global broadcaster with a custom directory.
    pub fn global_with_dir(log_dir: impl AsRef<Path>) -> &'static LogBroadcaster {
        GLOBAL_BROADCASTER.get_or_init(|| Self::new(1000, log_dir))
    }

    pub fn new(max_entries: usize, log_dir: impl AsRef<Path>) -> Self {
        let dir = log_dir.as_ref().to_path_buf();
        let _ = std::fs::create_dir_all(&dir);

        Self {
            max_entries,
            buffer: Arc::new(RwLock::new(VecDeque::with_capacity(max_entries))),
            log_dir: dir,
        }
    }

    /// Push an entry directly into the in-memory ring buffer without blocking file I/O.
    pub fn push_entry(&self, entry: LogEntry) {
        let mut buf = self.buffer.write();
        if buf.len() >= self.max_entries {
            buf.pop_front();
        }
        buf.push_back(entry);
    }

    /// Log a message and append to aura.log file synchronously.
    pub fn log(&self, level: LogLevel, subsystem: impl Into<String>, message: impl Into<String>) {
        let entry = LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            level,
            subsystem: subsystem.into(),
            message: message.into(),
        };

        // Append to in-memory ring buffer
        self.push_entry(entry.clone());

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

    pub fn log_dir(&self) -> &Path {
        &self.log_dir
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

/// Visitor to extract formatted messages and structured fields from `tracing::Event`.
struct EventMessageVisitor {
    message: String,
    fields: Vec<(String, String)>,
}

impl EventMessageVisitor {
    fn new() -> Self {
        Self {
            message: String::new(),
            fields: Vec::new(),
        }
    }
}

impl Visit for EventMessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
            // Strip surrounding quotes if present from Debug formatting
            if self.message.starts_with('"') && self.message.ends_with('"') && self.message.len() >= 2 {
                self.message = self.message[1..self.message.len() - 1].to_string();
            }
        } else {
            self.fields.push((field.name().to_string(), format!("{value:?}")));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields.push((field.name().to_string(), value.to_string()));
        }
    }
}

/// Custom `tracing_subscriber::Layer` capturing structured events into `LogBroadcaster`.
pub struct BroadcasterLayer {
    broadcaster: LogBroadcaster,
}

impl BroadcasterLayer {
    pub fn new(broadcaster: LogBroadcaster) -> Self {
        Self { broadcaster }
    }
}

impl<S> Layer<S> for BroadcasterLayer
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let level = match *metadata.level() {
            Level::TRACE => LogLevel::TRACE,
            Level::DEBUG => LogLevel::DEBUG,
            Level::INFO => LogLevel::INFO,
            Level::WARN => LogLevel::WARN,
            Level::ERROR => LogLevel::ERROR,
        };

        let mut visitor = EventMessageVisitor::new();
        event.record(&mut visitor);

        let final_message = if visitor.message.is_empty() {
            if visitor.fields.is_empty() {
                metadata.name().to_string()
            } else {
                visitor
                    .fields
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        } else if visitor.fields.is_empty() {
            visitor.message
        } else {
            let extra = visitor
                .fields
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(" ");
            format!("{} ({})", visitor.message, extra)
        };

        let subsystem = metadata.target().to_string();
        self.broadcaster.push_entry(LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            level,
            subsystem,
            message: final_message,
        });
    }
}

/// Guard holding the background non-blocking worker for rolling file log flushing.
pub struct LoggingGuard {
    _guard: Option<tracing_appender::non_blocking::WorkerGuard>,
}

/// Initialize the asynchronous structured system logging subsystem.
///
/// Features:
/// 1. Rolling file appender with non-blocking async background thread (`tracing-appender`).
/// 2. Zero telemetry, strictly offline local disk persistence.
/// 3. Real-time broadcasting to `LogBroadcaster` in-memory ring buffer for UI display.
/// 4. Console standard output layer with environment filter.
pub fn init_system_logging(custom_log_dir: Option<PathBuf>) -> anyhow::Result<LoggingGuard> {
    let log_dir = custom_log_dir.unwrap_or_else(default_log_dir);
    let _ = std::fs::create_dir_all(&log_dir);

    let broadcaster = LogBroadcaster::global_with_dir(&log_dir);

    // Non-blocking rolling daily file appender
    let file_appender = tracing_appender::rolling::daily(&log_dir, "aura.log");
    let (non_blocking_writer, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,aura=debug,aura_engine=debug,aura_ui=debug,aura_parser=debug,aura_sync=debug"));

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(false);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking_writer)
        .with_ansi(false)
        .with_target(true);

    let broadcast_layer = BroadcasterLayer::new(broadcaster.clone());

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer)
        .with(broadcast_layer);

    // Attempt to initialize global subscriber (ignores error if already initialized in unit tests)
    let _ = subscriber.try_init();

    Ok(LoggingGuard {
        _guard: Some(guard),
    })
}
