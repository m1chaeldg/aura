use aura_ui::preferences::AuraPreferences;
use std::fs;
use std::path::{Path, PathBuf};

struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!("aura_test_pref_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&p).expect("create temp dir");
        Self(p)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn tempdir() -> Result<TempDir, std::io::Error> {
    Ok(TempDir::new())
}

#[test]
fn test_preferences_save_and_load_cycle() {
    let tmp = tempdir().expect("tempdir");
    let pref_file = tmp.path().join("preferences.json");

    // Missing file returns default
    let default_prefs = AuraPreferences::load_from_file(&pref_file);
    assert_eq!(default_prefs.last_workspace_path, None);

    // Save custom path
    let mut prefs = AuraPreferences::default();
    prefs.last_workspace_path = Some(PathBuf::from("/custom/workspace/path"));
    prefs.save_to_file(&pref_file).expect("save");

    // Load back
    let loaded = AuraPreferences::load_from_file(&pref_file);
    assert_eq!(loaded.last_workspace_path, Some(PathBuf::from("/custom/workspace/path")));
}

#[test]
fn test_corrupted_preferences_recovers_to_default() {
    let tmp = tempdir().expect("tempdir");
    let pref_file = tmp.path().join("preferences.json");
    fs::write(&pref_file, "{ invalid json content").unwrap();

    let loaded = AuraPreferences::load_from_file(&pref_file);
    assert_eq!(loaded.last_workspace_path, None);
}
