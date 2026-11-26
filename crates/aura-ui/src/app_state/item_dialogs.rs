use super::{
    AuraAppState, InlineRenameState,
    MoveToDialogState, RenameDialogState,
};
use std::path::{Path, PathBuf};

impl AuraAppState {
    pub fn request_delete(&mut self, path: PathBuf) {
        self.pending_delete_path = Some(path);
    }

    pub fn cancel_delete(&mut self) {
        self.pending_delete_path = None;
    }

    pub async fn confirm_delete(&mut self) -> anyhow::Result<()> {
        if let Some(target_path) = self.pending_delete_path.take() {
            if let Some(root) = self.workspace_store.workspace_root.clone() {
                self.workspace_service
                    .delete_item(&root, &target_path)
                    .await?;
                self.workspace_store.apply_delete(&target_path);
            } else if target_path.exists() {
                if target_path.is_dir() {
                    tokio::fs::remove_dir_all(&target_path).await?;
                } else {
                    tokio::fs::remove_file(&target_path).await?;
                }
                self.workspace_store.apply_delete(&target_path);
            }

            // Close matching or descendant open tabs
            let mut ids_to_close = Vec::new();
            for tab in &self.tabs {
                if let Some(ref fp) = tab.file_path {
                    if fp == &target_path || fp.starts_with(&target_path) {
                        ids_to_close.push(tab.id.clone());
                    }
                }
            }
            for id in ids_to_close {
                self.close_tab(&id);
            }
        }
        Ok(())
    }

    pub fn start_inline_rename(&mut self, path: PathBuf, initial_name: impl Into<String>) {
        let name = initial_name.into();
        self.inline_rename = Some(InlineRenameState {
            path: path.clone(),
            current_value: name,
        });
    }

    pub fn cancel_inline_rename(&mut self) {
        self.inline_rename = None;
    }

    /// Requests opening the rename modal dialog for a file or folder.
    pub fn request_rename(&mut self, path: PathBuf, is_folder: bool) {
        let original_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let current_name = if !is_folder && original_name.ends_with(".aura") {
            original_name.trim_end_matches(".aura").to_string()
        } else {
            original_name.clone()
        };
        self.rename_dialog = Some(RenameDialogState {
            path,
            original_name,
            current_name,
            is_folder,
        });
    }

    /// Cancels active rename modal dialog.
    pub fn cancel_rename(&mut self) {
        self.rename_dialog = None;
    }

    /// Updates the text value in active rename dialog.
    pub fn update_rename_text(&mut self, text: String) {
        if let Some(ref mut d) = self.rename_dialog {
            d.current_name = text;
        }
    }

    /// Appends a character to the text in active rename dialog.
    pub fn append_rename_char(&mut self, c: char) {
        if let Some(ref mut d) = self.rename_dialog {
            d.current_name.push(c);
        }
    }

    /// Removes the last character from the text in active rename dialog.
    pub fn backspace_rename(&mut self) {
        if let Some(ref mut d) = self.rename_dialog {
            d.current_name.pop();
        }
    }

    /// Gets current text in active rename dialog for clipboard copy
    pub fn rename_get_text(&self) -> Option<String> {
        self.rename_dialog.as_ref().map(|d| d.current_name.clone())
    }

    /// Pastes text into active rename dialog
    pub fn rename_paste_text(&mut self, text: &str) {
        if let Some(ref mut d) = self.rename_dialog {
            d.current_name.push_str(text);
        }
    }

    /// Opens the Move To dialog for the given source item.
    pub fn open_move_to_dialog(&mut self, source_path: PathBuf) {
        if let Some(root) = self.workspace_store.workspace_root.clone() {
            let folders = self.workspace_store.all_folders();
            self.move_to_dialog = Some(MoveToDialogState::new(source_path, root, folders));
        }
    }

    /// Cancels active Move To dialog.
    pub fn cancel_move_to(&mut self) {
        self.move_to_dialog = None;
    }

    /// Updates the search filter in active Move To dialog.
    pub fn update_move_to_search(&mut self, text: String) {
        if let Some(ref mut d) = self.move_to_dialog {
            d.search_query = text;
            d.selected_index = 0;
        }
    }

    /// Appends a character to the search query in active Move To dialog.
    pub fn append_move_to_search_char(&mut self, c: char) {
        if let Some(ref mut d) = self.move_to_dialog {
            d.search_query.push(c);
            d.selected_index = 0;
        }
    }

    /// Removes the last character from the search query in active Move To dialog.
    pub fn backspace_move_to_search(&mut self) {
        if let Some(ref mut d) = self.move_to_dialog {
            d.search_query.pop();
            d.selected_index = 0;
        }
    }

    /// Gets current search text in active Move To dialog for clipboard copy
    pub fn move_to_get_text(&self) -> Option<String> {
        self.move_to_dialog.as_ref().map(|d| d.search_query.clone())
    }

    /// Pastes text into active Move To dialog search query
    pub fn move_to_paste_text(&mut self, text: &str) {
        if let Some(ref mut d) = self.move_to_dialog {
            d.search_query.push_str(text);
            d.selected_index = 0;
        }
    }

    /// Selects next destination in Move To dialog.
    pub fn move_to_select_next(&mut self) {
        if let Some(ref mut d) = self.move_to_dialog {
            d.select_next();
        }
    }

    /// Selects previous destination in Move To dialog.
    pub fn move_to_select_prev(&mut self) {
        if let Some(ref mut d) = self.move_to_dialog {
            d.select_prev();
        }
    }

    /// Updates tab titles and paths when a file or directory is renamed.
    pub fn handle_item_renamed(&mut self, old_path: &Path, new_path: &Path, new_name: &str) {
        for tab in &mut self.tabs {
            if let Some(ref fp) = tab.file_path {
                if fp == old_path {
                    tab.file_path = Some(new_path.to_path_buf());
                    tab.title = new_name.to_string();
                } else if fp.starts_with(old_path) {
                    if let Ok(rel) = fp.strip_prefix(old_path) {
                        tab.file_path = Some(new_path.join(rel));
                    }
                }
            }
        }
    }

    /// Confirms and commits the rename operation asynchronously to disk and stores.
    pub async fn confirm_rename(&mut self) -> anyhow::Result<Option<PathBuf>> {
        if let Some(rename_state) = self.rename_dialog.take() {
            let trimmed = rename_state.current_name.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            let old_path = rename_state.path;
            if let Some(root) = self.workspace_store.workspace_root.clone() {
                let new_path = self
                    .workspace_service
                    .rename_item(&root, &old_path, trimmed)
                    .await?;
                let final_name = new_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(trimmed);
                self.workspace_store
                    .apply_rename(&old_path, &new_path, final_name);
                self.handle_item_renamed(&old_path, &new_path, final_name);
                return Ok(Some(new_path));
            } else if old_path.exists() {
                let parent = old_path.parent().unwrap_or(&old_path);
                let final_name = if old_path.is_file() && !trimmed.ends_with(".aura") {
                    format!("{}.aura", trimmed)
                } else {
                    trimmed.to_string()
                };
                let new_path = parent.join(&final_name);
                tokio::fs::rename(&old_path, &new_path).await?;
                self.workspace_store
                    .apply_rename(&old_path, &new_path, &final_name);
                self.handle_item_renamed(&old_path, &new_path, &final_name);
                return Ok(Some(new_path));
            }
        }
        Ok(None)
    }

}
