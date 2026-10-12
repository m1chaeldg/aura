use aura_engine::logging::{LogBroadcaster, LogEntry, LogLevel};
use aura_ui::app_state::AuraAppState;

#[test]
fn test_app_state_log_viewer_toggle() {
    let mut state = AuraAppState::new();
    assert!(!state.is_log_viewer_open);

    state.toggle_log_viewer();
    assert!(state.is_log_viewer_open);

    state.toggle_log_viewer();
    assert!(!state.is_log_viewer_open);
}

#[test]
fn test_app_state_log_filters_and_clear() {
    let mut state = AuraAppState::new();
    let broadcaster = LogBroadcaster::global();
    broadcaster.clear();

    broadcaster.push_entry(LogEntry {
        id: "1".to_string(),
        timestamp: chrono::Utc::now(),
        level: LogLevel::INFO,
        subsystem: "aura_engine".to_string(),
        message: "Request executed".to_string(),
    });

    broadcaster.push_entry(LogEntry {
        id: "2".to_string(),
        timestamp: chrono::Utc::now(),
        level: LogLevel::WARN,
        subsystem: "aura_ui".to_string(),
        message: "Memory high".to_string(),
    });

    broadcaster.push_entry(LogEntry {
        id: "3".to_string(),
        timestamp: chrono::Utc::now(),
        level: LogLevel::ERROR,
        subsystem: "aura_sync".to_string(),
        message: "Failed to write file".to_string(),
    });

    // Default filters: returns all
    let all = state.get_filtered_logs();
    assert_eq!(all.len(), 3);

    // Filter by level
    state.set_log_filter_level(Some(LogLevel::WARN));
    let warn_or_above = state.get_filtered_logs();
    assert_eq!(warn_or_above.len(), 2);

    // Filter by subsystem
    state.set_log_filter_level(None);
    state.set_log_filter_subsystem(Some("aura_engine".to_string()));
    let engine_only = state.get_filtered_logs();
    assert_eq!(engine_only.len(), 1);
    assert_eq!(engine_only[0].subsystem, "aura_engine");

    // Filter by search query
    state.set_log_filter_subsystem(None);
    state.set_log_search("Memory");
    let search_res = state.get_filtered_logs();
    assert_eq!(search_res.len(), 1);
    assert_eq!(search_res[0].message, "Memory high");

    // Clear logs
    state.set_log_search("");
    state.clear_logs();
    assert_eq!(state.get_filtered_logs().len(), 0);
}
