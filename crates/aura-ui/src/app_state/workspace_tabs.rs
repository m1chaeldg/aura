use super::layout_and_palette::build_tree_from_metadata;
use super::AuraAppState;
use crate::stores::WorkspaceTab;
use aura_core::model::{AuraRequest, Collection, Folder, HttpMethod};
use std::path::Path;

impl AuraAppState {
    /// Toggles folder expanded/collapsed state in the workspace store.
    pub fn toggle_folder(&mut self, path: &Path) {
        self.workspace_store.toggle_folder_expanded(path);
    }

    /// Opens a workspace directory, saving it to preferences and initializing the store tree.
    pub fn open_workspace_sync(&mut self, root: &Path) -> anyhow::Result<()> {
        let mut prefs = crate::preferences::AuraPreferences::load();
        prefs.last_workspace_path = Some(root.to_path_buf());
        let _ = prefs.save();

        let existing = aura_sync::workspace_meta::WorkspaceMetadata::load_from_disk(root).ok();
        let metadata =
            aura_sync::workspace_meta::WorkspaceMetadata::reconcile_with_disk(root, existing)?;
        let _ = metadata.save_to_disk(root);
        let tree = build_tree_from_metadata(root, &metadata.order)?;

        self.workspace_store
            .set_workspace(root.to_path_buf(), metadata, tree);
        self.reload_environments_from_workspace();
        Ok(())
    }

    /// Asynchronously opens a workspace directory.
    pub async fn open_workspace_directory(&mut self, root: &Path) -> anyhow::Result<()> {
        self.open_workspace_sync(root)
    }

    /// Opens a file in a tab, parsing request or replacing an unmodified untitled tab.
    pub fn open_file_in_tab(&mut self, path: &Path) -> anyhow::Result<()> {
        if let Some(pos) = self
            .tabs
            .iter()
            .position(|t| t.file_path.as_deref() == Some(path))
        {
            self.select_tab(pos);
            return Ok(());
        }

        let content = std::fs::read_to_string(path).unwrap_or_default();
        let file_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled");
        let req = if content.trim().is_empty() {
            AuraRequest::new(file_stem, HttpMethod::GET, "https://postman-echo.com/get")
        } else if let Ok(r) = aura_parser::parse_request(&content) {
            r
        } else {
            match aura_sync::toml_to_request(&content, None) {
                Ok(r) => r,
                Err(_) => {
                    AuraRequest::new(file_stem, HttpMethod::GET, "https://postman-echo.com/get")
                }
            }
        };

        for tab in &mut self.tabs {
            tab.is_active = false;
        }

        let mut new_tab = WorkspaceTab::new(req);
        if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
            new_tab.title = fname.to_string();
        }
        new_tab.file_path = Some(path.to_path_buf());

        if self.tabs.len() == 1
            && self.tabs[0].file_path.is_none()
            && !self.tabs[0].is_dirty
            && self.tabs[0].title == "Untitled Request"
        {
            self.tabs[0] = new_tab;
            self.active_tab_index = 0;
        } else {
            self.tabs.push(new_tab);
            self.active_tab_index = self.tabs.len() - 1;
        }

        Ok(())
    }

    /// Synchronously reconciles workspace store with disk changes and prunes deleted open tabs.
    pub fn reconcile_workspace_sync(&mut self) -> anyhow::Result<()> {
        if let Some(root) = self.workspace_store.workspace_root.clone() {
            let existing = aura_sync::workspace_meta::WorkspaceMetadata::load_from_disk(&root).ok();
            let metadata =
                aura_sync::workspace_meta::WorkspaceMetadata::reconcile_with_disk(&root, existing)?;
            let _ = metadata.save_to_disk(&root);
            let tree = build_tree_from_metadata(&root, &metadata.order)?;
            self.workspace_store.apply_external_sync(metadata, tree);

            let mut ids_to_close = Vec::new();
            for tab in &self.tabs {
                if let Some(ref fp) = tab.file_path {
                    if !fp.exists() {
                        ids_to_close.push(tab.id.clone());
                    }
                }
            }
            for id in ids_to_close {
                self.close_tab(&id);
            }
            self.reload_environments_from_workspace();
        }
        Ok(())
    }

    /// Reconciles workspace store with disk changes and prunes deleted open tabs.
    pub async fn reconcile_workspace_with_disk(&mut self) -> anyhow::Result<()> {
        self.reconcile_workspace_sync()
    }

    /// Discover and load collections from workspace directory
    pub fn load_workspace_collections(&mut self, dir: &Path) -> anyhow::Result<usize> {
        let mut loaded = 0;
        if !dir.exists() {
            return Ok(0);
        }
        let mut paths_to_check = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    paths_to_check.push(p);
                } else if p.is_dir() {
                    let dir_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if aura_sync::workspace_meta::is_ignored_workspace_dir(dir_name) {
                        continue;
                    }
                    if let Ok(sub_entries) = std::fs::read_dir(&p) {
                        for sub in sub_entries.flatten() {
                            let sub_p = sub.path();
                            if sub_p.is_file() {
                                paths_to_check.push(sub_p);
                            }
                        }
                    }
                }
            }
        }

        for p in paths_to_check {
            let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if fname.ends_with(".collection.json")
                || fname.ends_with(".collection.toml")
                || fname.ends_with(".aura.json")
                || fname.ends_with(".aura.toml")
            {
                if let Ok(content) = std::fs::read_to_string(&p) {
                    if fname.ends_with(".json") {
                        if let Ok(col) = serde_json::from_str::<Collection>(&content) {
                            if !self.collections.iter().any(|c| c.id == col.id) {
                                self.collections.push(col);
                                loaded += 1;
                            }
                        }
                    } else if fname.ends_with(".toml") {
                        if let Ok(col) = toml::from_str::<Collection>(&content) {
                            if !self.collections.iter().any(|c| c.id == col.id) {
                                self.collections.push(col);
                                loaded += 1;
                            }
                        }
                    }
                }
            }
        }
        Ok(loaded)
    }

    /// Open request by collection and request IDs
    pub fn open_collection_request(&mut self, collection_id: &str, request_id: &str) -> bool {
        for col in &self.collections {
            if col.id == collection_id {
                if let Some(req) = col.requests.iter().find(|r| r.id == request_id) {
                    self.open_request(req.clone());
                    return true;
                }
                fn find_in_folders<'a>(
                    folders: &'a [Folder],
                    req_id: &str,
                ) -> Option<&'a AuraRequest> {
                    for f in folders {
                        if let Some(req) = f.requests.iter().find(|r| r.id == req_id) {
                            return Some(req);
                        }
                        if let Some(req) = find_in_folders(&f.subfolders, req_id) {
                            return Some(req);
                        }
                    }
                    None
                }
                if let Some(req) = find_in_folders(&col.folders, request_id) {
                    self.open_request(req.clone());
                    return true;
                }
            }
        }
        false
    }

    /// Open or focus a tab for a request
    pub fn open_request(&mut self, req: AuraRequest) {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == req.id) {
            self.select_tab(pos);
        } else {
            for t in &mut self.tabs {
                t.is_active = false;
            }
            let new_tab = WorkspaceTab::new(req);
            self.tabs.push(new_tab);
            self.active_tab_index = self.tabs.len() - 1;
        }
    }

    /// Persistent `+` action: Append new untitled request
    pub fn new_request(&mut self) {
        let req = AuraRequest::new(
            "Untitled Request",
            HttpMethod::GET,
            "https://postman-echo.com/get",
        );
        self.open_request(req);
    }

    /// Switch active tab by index
    pub fn select_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            for t in &mut self.tabs {
                t.is_active = false;
            }
            self.tabs[index].is_active = true;
            self.active_tab_index = index;
            self.clear_setting_drafts();
        }
    }

    /// Navigate to the next tab in circular order
    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            let next_idx = (self.active_tab_index + 1) % self.tabs.len();
            self.select_tab(next_idx);
        }
    }

    /// Navigate to the previous tab in circular order
    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            let prev_idx = if self.active_tab_index == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab_index - 1
            };
            self.select_tab(prev_idx);
        }
    }

    /// Close tab by ID
    pub fn close_tab(&mut self, id: &str) {
        self.execution_service.cancel();
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            self.tabs.remove(pos);
            if self.tabs.is_empty() {
                self.active_tab_index = 0;
            } else if self.active_tab_index >= self.tabs.len() {
                self.active_tab_index = self.tabs.len() - 1;
                self.tabs[self.active_tab_index].is_active = true;
            } else {
                self.tabs[self.active_tab_index].is_active = true;
            }
        }
    }

    /// Close the active tab
    pub fn close_active_tab(&mut self) {
        if !self.tabs.is_empty() && self.active_tab_index < self.tabs.len() {
            let id = self.tabs[self.active_tab_index].id.clone();
            self.close_tab(&id);
        }
    }

    /// Mark active tab as dirty or clean
    pub fn mark_active_tab_dirty(&mut self, is_dirty: bool) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.mark_dirty(is_dirty);
        }
    }

    /// Mark specific tab by ID as dirty or clean
    pub fn mark_tab_dirty(&mut self, id: &str, is_dirty: bool) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.mark_dirty(is_dirty);
        }
    }

    /// Checks if the active tab is currently marked dirty
    pub fn is_active_tab_dirty(&self) -> bool {
        self.tabs
            .get(self.active_tab_index)
            .map(|t| t.is_dirty)
            .unwrap_or(false)
    }

    /// Checks if a specific tab by ID is currently marked dirty
    pub fn is_tab_dirty(&self, id: &str) -> bool {
        self.tabs
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.is_dirty)
            .unwrap_or(false)
    }

    /// Save the active request and clear dirty indicator
    pub fn save_active_tab(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.mark_dirty(false);
            self.status_message = Some(format!("Saved: {}", tab.title));
        }
    }

    /// Automatically saves the active request if dirty, returning whether it was saved
    pub fn auto_save_active_tab(&mut self) -> bool {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if tab.is_dirty {
                tab.mark_dirty(false);
                self.status_message = Some(format!("Auto-saved: {}", tab.title));
                return true;
            }
        }
        false
    }

    /// Automatically saves a specific request tab by ID if dirty, returning whether it was saved
    pub fn auto_save_tab(&mut self, id: &str) -> bool {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            if tab.is_dirty {
                tab.mark_dirty(false);
                self.status_message = Some(format!("Auto-saved: {}", tab.title));
                return true;
            }
        }
        false
    }

    /// Configures whether delayed auto-save is enabled
    pub fn set_auto_save_enabled(&mut self, enabled: bool) {
        self.auto_save_enabled = enabled;
    }

    /// Configures the auto-save debounce delay in milliseconds
    pub fn set_auto_save_delay_ms(&mut self, delay_ms: u64) {
        self.auto_save_delay_ms = delay_ms;
    }
}
