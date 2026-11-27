use super::{AuraAppState, CommandPaletteItem, TopMenuKind};
use crate::keybindings::{Action, KeybindingPreset};
use crate::stores::LayoutOrientation;
use aura_core::model::HttpMethod;
use std::path::Path;

impl AuraAppState {
    /// Adjust pane divider percentage (clamped between 0.1 and 0.9)
    pub fn set_split_ratio(&mut self, ratio: f32) {
        self.split_ratio = ratio.clamp(0.1, 0.9);
    }

    /// Start dragging divider
    pub fn start_dragging_divider(&mut self) {
        self.is_dragging_divider = true;
    }

    /// Stop dragging divider
    pub fn stop_dragging_divider(&mut self) {
        self.is_dragging_divider = false;
    }

    /// Adjust pane divider by delta ratio (clamped between 0.15 and 0.85)
    pub fn drag_divider(&mut self, delta_ratio: f32) {
        self.split_ratio = (self.split_ratio + delta_ratio).clamp(0.15, 0.85);
    }

    /// Reset pane divider to 50/50 split
    pub fn reset_divider(&mut self) {
        self.split_ratio = 0.50;
    }

    /// Toggle layout between Horizontal and Vertical
    pub fn toggle_orientation(&mut self) {
        self.orientation = match self.orientation {
            LayoutOrientation::Horizontal => LayoutOrientation::Vertical,
            LayoutOrientation::Vertical => LayoutOrientation::Horizontal,
        };
    }

    /// Open command palette modal
    pub fn open_command_palette(&mut self) {
        self.is_command_palette_open = true;
        self.selected_palette_index = 0;
    }

    /// Close command palette modal
    pub fn close_command_palette(&mut self) {
        self.is_command_palette_open = false;
        self.command_palette_search.clear();
        self.selected_palette_index = 0;
    }

    /// Toggle command palette modal
    pub fn toggle_command_palette(&mut self) {
        if self.is_command_palette_open {
            self.close_command_palette();
        } else {
            self.open_command_palette();
        }
    }

    /// Toggle log viewer drawer
    pub fn toggle_log_viewer(&mut self) {
        self.is_log_viewer_open = !self.is_log_viewer_open;
        if self.is_log_viewer_open {
            tracing::info!(target: "aura_ui", "System log viewer opened");
        }
    }

    /// Toggle sidebar collapsed state
    pub fn toggle_sidebar(&mut self) {
        self.is_sidebar_collapsed = !self.is_sidebar_collapsed;
    }

    /// Set the minimum log level filter for the log viewer
    pub fn set_log_filter_level(&mut self, level: Option<aura_engine::LogLevel>) {
        self.log_filter_level = level;
    }

    /// Set the subsystem filter for the log viewer
    pub fn set_log_filter_subsystem(&mut self, subsystem: Option<String>) {
        self.log_filter_subsystem = subsystem;
    }

    /// Set the search query for the log viewer
    pub fn set_log_search(&mut self, query: impl Into<String>) {
        self.log_search = query.into();
    }

    /// Retrieve filtered log entries from the global broadcaster
    pub fn get_filtered_logs(&self) -> Vec<aura_engine::LogEntry> {
        aura_engine::LogBroadcaster::global().filter_entries(
            self.log_filter_level,
            self.log_filter_subsystem.as_deref(),
            if self.log_search.is_empty() {
                None
            } else {
                Some(&self.log_search)
            },
        )
    }

    /// Set whether the log viewer renders as a copyable textbox instead of a table
    pub fn set_log_viewer_text_mode(&mut self, text_mode: bool) {
        self.is_log_viewer_text_mode = text_mode;
    }

    /// Format filtered log entries as plain text for textbox display or clipboard copy
    pub fn format_filtered_logs_text(&self) -> String {
        self.get_filtered_logs()
            .iter()
            .rev()
            .take(200)
            .map(|entry| {
                format!(
                    "[{}] [{:<5}] [{}] {}",
                    entry.timestamp.format("%H:%M:%S%.3f"),
                    entry.level.as_str(),
                    entry.subsystem,
                    entry.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Clear all logs in the ring buffer
    pub fn clear_logs(&mut self) {
        aura_engine::LogBroadcaster::global().clear();
        tracing::info!(target: "aura_ui", "System logs cleared from ring buffer");
    }

    /// Set active keybinding preset
    pub fn set_keybinding_preset(&mut self, preset: KeybindingPreset) {
        self.keybinding_preset = preset;
    }

    /// Toggle top-bar dropdown menu
    pub fn toggle_top_menu(&mut self, menu: TopMenuKind) {
        if self.open_top_menu == Some(menu) {
            self.open_top_menu = None;
        } else {
            self.open_top_menu = Some(menu);
            self.is_env_dropdown_open = false;
        }
    }

    /// Close any open top-bar dropdown menu
    pub fn close_top_menu(&mut self) {
        self.open_top_menu = None;
    }

    /// Filter items for command palette (open tabs + application commands)
    pub fn filtered_palette_items(&self) -> Vec<CommandPaletteItem> {
        let q = self.command_palette_search.to_lowercase();
        let mut items: Vec<CommandPaletteItem> = self
            .tabs
            .iter()
            .filter(|t| {
                q.is_empty()
                    || t.title.to_lowercase().contains(&q)
                    || t.request.url.to_lowercase().contains(&q)
            })
            .map(|t| CommandPaletteItem {
                id: t.id.clone(),
                title: t.title.clone(),
                subtitle: t.request.url.clone(),
                method: Some(t.method),
            })
            .collect();

        const APP_COMMANDS: &[(&str, &str, &str)] = &[
            ("cmd:new_request", "File: New Request", "Create a new request tab (Ctrl+N)"),
            ("cmd:save_request", "File: Save Request", "Save active request to disk (Ctrl+S)"),
            ("cmd:close_tab", "File: Close Active Tab", "Close current request tab (Ctrl+W)"),
            ("cmd:export_code", "File: Export Request Code", "Generate cURL, fetch, Python, Rust, or Go"),
            ("cmd:open_workspace", "Workspace: Open Folder...", "Open a workspace directory"),
            ("cmd:new_environment", "Workspace: New Environment...", "Create a new environment profile"),
            ("cmd:import_wsdl", "Workspace: Import WSDL...", "Import SOAP WSDL into collection"),
            ("cmd:git_sync", "Workspace: Git Status & Sync", "Open built-in Git diff, commit & push/pull"),
            ("cmd:toggle_sidebar", "View: Toggle Sidebar", "Show or hide workspace tree"),
            ("cmd:toggle_layout", "View: Toggle Split Orientation", "Switch Side-by-Side / Stacked"),
            ("cmd:reset_divider", "View: Reset Split 50/50", "Reset pane divider to 50%"),
            ("cmd:toggle_logs", "View: Toggle Diagnostic Logs", "Open or close system log viewer"),
            ("cmd:cookies", "Tools: Cookie Jar Manager", "Inspect, edit, and clear cookies"),
            ("cmd:proxy", "Tools: Global Proxy Settings", "Configure HTTP / HTTPS / SOCKS5 proxy"),
            ("cmd:capture_proxy", "Tools: Capture Proxy & Interceptor", "Local MITM capture proxy & rules"),
            ("cmd:security_vault", "Tools: Security Vaults & mTLS", "Configure HashiCorp Vault, AWS SM & certs"),
            ("cmd:load_tester", "Tools: Load Tester", "Run JMeter-style load & SLA tests"),
            ("cmd:settings", "Settings: Preferences & Auto-Update", "Configure auto-save and GitHub release checks"),
            ("cmd:help", "Help: Documentation & Shortcuts", "View Aura guide, variables, and shortcuts"),
            ("cmd:check_updates", "Help: Check for Updates", "Check GitHub Releases for new Aura version"),
            ("cmd:about", "Help: About Aura & Version", "Show version, license, and build info"),
        ];

        for (id, title, subtitle) in APP_COMMANDS {
            if q.is_empty()
                || title.to_lowercase().contains(&q)
                || subtitle.to_lowercase().contains(&q)
            {
                items.push(CommandPaletteItem {
                    id: (*id).to_string(),
                    title: (*title).to_string(),
                    subtitle: (*subtitle).to_string(),
                    method: None,
                });
            }
        }

        items
    }

    /// Move down in command palette list
    pub fn palette_select_next(&mut self) {
        let items = self.filtered_palette_items();
        if !items.is_empty() {
            self.selected_palette_index = (self.selected_palette_index + 1) % items.len();
        }
    }

    /// Move up in command palette list
    pub fn palette_select_prev(&mut self) {
        let items = self.filtered_palette_items();
        if !items.is_empty() {
            self.selected_palette_index = if self.selected_palette_index == 0 {
                items.len() - 1
            } else {
                self.selected_palette_index - 1
            };
        }
    }

    /// Execute selected item in command palette
    pub fn execute_selected_palette_item(&mut self) -> Option<String> {
        let items = self.filtered_palette_items();
        if let Some(item) = items.get(self.selected_palette_index) {
            let id = item.id.clone();
            if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
                self.select_tab(pos);
            }
            self.close_command_palette();
            Some(id)
        } else {
            None
        }
    }

    /// Handle keyboard action directly
    pub fn handle_key_action(&mut self, action: Action) -> bool {
        match action {
            Action::SendRequest => {
                self.status_message = Some("Dispatching request...".to_string());
                true
            }
            Action::OpenCommandPalette => {
                self.open_command_palette();
                true
            }
            Action::CloseCommandPalette => {
                if self.is_command_palette_open {
                    self.close_command_palette();
                    true
                } else {
                    false
                }
            }
            Action::ToggleLogViewer => {
                self.toggle_log_viewer();
                true
            }
            Action::NewRequest => {
                self.new_request();
                true
            }
            Action::CloseActiveTab => {
                self.close_active_tab();
                true
            }
            Action::ToggleLayout => {
                self.toggle_orientation();
                true
            }
            Action::ResetDivider => {
                self.reset_divider();
                true
            }
            Action::SaveRequest => {
                self.save_active_tab();
                true
            }
            Action::NextTab => {
                self.next_tab();
                true
            }
            Action::PrevTab => {
                self.prev_tab();
                true
            }
            Action::TreeUp => {
                self.workspace_store.select_prev_node();
                true
            }
            Action::TreeDown => {
                self.workspace_store.select_next_node();
                true
            }
            Action::TreeExpand => {
                self.workspace_store.expand_selected();
                true
            }
            Action::TreeCollapse => {
                self.workspace_store.collapse_selected();
                true
            }
            Action::TreeOpen => {
                if let Some(selected) = self.workspace_store.selected_path.clone() {
                    if selected.is_file() {
                        let _ = self.open_file_in_tab(&selected);
                        return true;
                    }
                }
                false
            }
            Action::DuplicateItem => {
                if let Some(selected) = self.workspace_store.selected_path.clone() {
                    if let Some(root) = self.workspace_store.workspace_root.clone() {
                        let srv = self.workspace_service.clone();
                        let rt_handle = tokio::runtime::Handle::try_current();
                        if let Ok(handle) = rt_handle {
                            handle.spawn(async move {
                                let _ = srv.duplicate_item(&root, &selected).await;
                            });
                        }
                    }
                    return true;
                }
                false
            }
            Action::CopyItem => {
                self.workspace_store.copy_selected();
                true
            }
            Action::PasteItem => {
                if let Some(payload) = self.workspace_store.clipboard_item.clone() {
                    if let Some(root) = self.workspace_store.workspace_root.clone() {
                        let target = self
                            .workspace_store
                            .selected_path
                            .clone()
                            .unwrap_or_else(|| root.clone());
                        let target_dir = if target.is_dir() {
                            target
                        } else {
                            target.parent().unwrap_or(&target).to_path_buf()
                        };
                        let srv = self.workspace_service.clone();
                        let rt_handle = tokio::runtime::Handle::try_current();
                        if let Ok(handle) = rt_handle {
                            handle.spawn(async move {
                                let _ = srv.paste_item(&root, &payload, &target_dir).await;
                            });
                        }
                    }
                    return true;
                }
                false
            }
            Action::RenameItem => {
                if let Some(selected) = self.workspace_store.selected_path.clone() {
                    let name = selected
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();
                    let is_folder = selected.is_dir();
                    self.request_rename(selected.clone(), is_folder);
                    self.start_inline_rename(selected, name);
                    true
                } else {
                    false
                }
            }
            Action::DeleteItem => {
                if let Some(selected) = self.workspace_store.selected_path.clone() {
                    self.request_delete(selected);
                    true
                } else {
                    false
                }
            }
        }
    }
}

pub(super) fn build_tree_from_metadata(
    dir: &Path,
    order: &[aura_sync::workspace_meta::WorkspaceOrderItem],
) -> anyhow::Result<Vec<crate::stores::workspace_store::WorkspaceTreeNode>> {
    use crate::stores::workspace_store::WorkspaceTreeNode;
    use aura_sync::workspace_meta::WorkspaceOrderItem;

    let mut tree = Vec::new();
    for item in order {
        match item {
            WorkspaceOrderItem::Folder { name, order } => {
                if aura_sync::workspace_meta::is_ignored_workspace_dir(name) {
                    continue;
                }
                let folder_path = dir.join(name);
                let children = build_tree_from_metadata(&folder_path, order)?;
                tree.push(WorkspaceTreeNode::Folder {
                    name: name.clone(),
                    path: folder_path,
                    is_expanded: true,
                    children,
                });
            }
            WorkspaceOrderItem::Request { name } => {
                let req_path = dir.join(name);
                let method = if let Ok(content) = std::fs::read_to_string(&req_path) {
                    if let Ok(r) = aura_parser::parse_request(&content) {
                        r.method
                    } else if let Ok(req) = aura_sync::toml_to_request(&content, None) {
                        req.method
                    } else {
                        HttpMethod::GET
                    }
                } else {
                    HttpMethod::GET
                };
                tree.push(WorkspaceTreeNode::Request {
                    name: name.clone(),
                    path: req_path,
                    method,
                });
            }
        }
    }
    Ok(tree)
}
