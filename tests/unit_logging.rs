use aura::logging::{LogBroadcaster, LogLevel};

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
