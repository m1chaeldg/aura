use aura::logging::{init_system_logging, LogBroadcaster, LogLevel};

#[test]
fn test_logging_broadcaster_and_file_append() {
    let temp_dir = std::env::temp_dir().join(format!("aura_test_log_{}", uuid::Uuid::new_v4()));
    let broadcaster = LogBroadcaster::new(50, &temp_dir);

    broadcaster.log(LogLevel::INFO, "Engine", "Engine initialized");
    broadcaster.log(LogLevel::WARN, "Deno Runtime", "Memory threshold watch");
    broadcaster.log(LogLevel::ERROR, "Network", "Connection timed out");

    let entries = broadcaster.get_entries();
    assert_eq!(entries.len(), 3);

    let filtered = broadcaster.filter_entries(Some(LogLevel::WARN), None, None);
    assert_eq!(filtered.len(), 2);

    let filtered_sub = broadcaster.filter_entries(None, Some("Engine"), None);
    assert_eq!(filtered_sub.len(), 1);
    assert_eq!(filtered_sub[0].message, "Engine initialized");

    let log_file = temp_dir.join("aura.log");
    assert!(log_file.exists());
    let content = std::fs::read_to_string(log_file).unwrap();
    assert!(content.contains("[INFO] [Engine] Engine initialized"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_system_logging_initialization_and_tracing_capture() {
    let temp_dir = std::env::temp_dir().join(format!("aura_test_system_log_{}", uuid::Uuid::new_v4()));
    let _guard = init_system_logging(Some(temp_dir.clone())).expect("initialize system logging");

    tracing::info!(target: "aura_engine", "System engine started up");
    tracing::warn!(target: "aura_ui", "Layout split adjusted");
    tracing::error!(target: "aura_sync", "Disk sync retry required");

    let broadcaster = LogBroadcaster::global();
    let entries = broadcaster.get_entries();

    // Verify events were captured into the in-memory ring buffer
    assert!(entries.iter().any(|e| e.message.contains("System engine started up") && e.level == LogLevel::INFO));
    assert!(entries.iter().any(|e| e.message.contains("Layout split adjusted") && e.level == LogLevel::WARN));
    assert!(entries.iter().any(|e| e.message.contains("Disk sync retry required") && e.level == LogLevel::ERROR));

    // Test filtering by level and subsystem
    let error_entries = broadcaster.filter_entries(Some(LogLevel::ERROR), None, None);
    assert!(error_entries.iter().all(|e| e.level == LogLevel::ERROR));

    let ui_entries = broadcaster.filter_entries(None, Some("aura_ui"), None);
    assert!(ui_entries.iter().any(|e| e.subsystem == "aura_ui"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}
