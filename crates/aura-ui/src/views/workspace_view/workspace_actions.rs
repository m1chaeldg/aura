use super::AuraWorkspaceView;
use crate::views::SidebarMenuAction;
use aura_core::model::HttpMethod;
use gpui::*;

impl AuraWorkspaceView {
    /// Confirms pending item deletion off the main thread and syncs workspace.
    pub fn confirm_delete_action(&mut self, cx: &mut Context<Self>) {
        let mut state_clone = self.state.clone();
        self.state.pending_delete_path = None;
        cx.notify();

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                if let Err(e) = state_clone.confirm_delete().await {
                    tracing::error!(target: "aura_ui::workspace", "Failed to confirm delete: {e}");
                }
                let _ = this.update(&mut cx, |this, cx| {
                    if let Err(e) = this.state.reconcile_workspace_sync() {
                        tracing::error!(target: "aura_ui::workspace", "Failed to reconcile workspace: {e}");
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Confirms pending item renaming off the main thread and syncs workspace.
    pub fn confirm_rename_action(&mut self, cx: &mut Context<Self>) {
        let mut state_clone = self.state.clone();
        self.state.rename_dialog = None;
        cx.notify();

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                match state_clone.confirm_rename().await {
                    Ok(Some(new_path)) => {
                        let _ = this.update(&mut cx, |this, cx| {
                            if let Err(e) = this.state.reconcile_workspace_sync() {
                                tracing::error!(target: "aura_ui::workspace", "Failed to reconcile workspace: {e}");
                            }
                            this.state.workspace_store.select_node(new_path);
                            cx.notify();
                        });
                    }
                    Ok(None) => {}
                    Err(e) => {
                        tracing::error!(target: "aura_ui::workspace", "Failed to confirm rename: {e}");
                    }
                }
            }
        })
        .detach();
    }

    /// Confirms pending Move To dialog action and relocates item.
    pub fn confirm_move_to_action(&mut self, cx: &mut Context<Self>) {
        if let Some(dialog) = self.state.move_to_dialog.take() {
            if let Some(dest) = dialog.selected_destination() {
                let source = dialog.source_path.clone();
                let target_parent = dest.path.clone();
                self.handle_drop_item(
                    source,
                    target_parent,
                    crate::stores::workspace_store::DragPlacement::Inside,
                    cx,
                );
            }
        }
    }

    /// Handles actions dispatched from the sidebar context menu.
    pub fn handle_sidebar_menu_action(
        &mut self,
        action: SidebarMenuAction,
        cx: &mut Context<Self>,
    ) {
        let menu = match self.state.workspace_store.active_context_menu.take() {
            Some(m) => m,
            None => return,
        };
        let target = menu.target_path;
        let is_folder = menu.is_folder;

        match action {
            SidebarMenuAction::NewRequest => {
                let parent = if is_folder {
                    target
                } else {
                    target.parent().unwrap_or(&target).to_path_buf()
                };
                let root = self.state.workspace_store.workspace_root.clone();
                let srv = self.state.workspace_service.clone();
                cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                    let mut cx = cx.clone();
                    async move {
                        if let Some(r) = root {
                            if let Ok(p) = srv
                                .create_request(&r, &parent, "New Request", HttpMethod::GET)
                                .await
                            {
                                let _ = this.update(&mut cx, |this, cx| {
                                    let _ = this.state.open_file_in_tab(&p);
                                    let _ = this.state.reconcile_workspace_sync();
                                    cx.notify();
                                });
                            }
                        }
                    }
                })
                .detach();
            }
            SidebarMenuAction::NewFolder => {
                let parent = if is_folder {
                    target
                } else {
                    target.parent().unwrap_or(&target).to_path_buf()
                };
                let root = self.state.workspace_store.workspace_root.clone();
                let srv = self.state.workspace_service.clone();
                cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                    let mut cx = cx.clone();
                    async move {
                        if let Some(r) = root {
                            let _ = srv.create_folder(&r, &parent, "New Folder").await;
                            let _ = this.update(&mut cx, |this, cx| {
                                let _ = this.state.reconcile_workspace_sync();
                                cx.notify();
                            });
                        }
                    }
                })
                .detach();
            }
            SidebarMenuAction::Duplicate => {
                let root = self.state.workspace_store.workspace_root.clone();
                let srv = self.state.workspace_service.clone();
                cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                    let mut cx = cx.clone();
                    async move {
                        if let Some(r) = root {
                            if let Ok(p) = srv.duplicate_item(&r, &target).await {
                                let _ = this.update(&mut cx, |this, cx| {
                                    if p.is_file() {
                                        let _ = this.state.open_file_in_tab(&p);
                                    }
                                    let _ = this.state.reconcile_workspace_sync();
                                    cx.notify();
                                });
                            }
                        }
                    }
                })
                .detach();
            }
            SidebarMenuAction::Copy => {
                self.state.workspace_store.select_node(target);
                self.state.workspace_store.copy_selected();
                cx.notify();
            }
            SidebarMenuAction::Paste => {
                if let Some(payload) = self.state.workspace_store.clipboard_item.clone() {
                    let root = self.state.workspace_store.workspace_root.clone();
                    let srv = self.state.workspace_service.clone();
                    let target_parent = if is_folder {
                        target
                    } else {
                        target.parent().unwrap_or(&target).to_path_buf()
                    };
                    cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                        let mut cx = cx.clone();
                        async move {
                            if let Some(r) = root {
                                let _ = srv.paste_item(&r, &payload, &target_parent).await;
                                let _ = this.update(&mut cx, |this, cx| {
                                    let _ = this.state.reconcile_workspace_sync();
                                    cx.notify();
                                });
                            }
                        }
                    })
                    .detach();
                }
            }
            SidebarMenuAction::Rename => {
                let name = target
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                self.state.request_rename(target.clone(), is_folder);
                self.state.start_inline_rename(target, name);
                cx.notify();
            }

            SidebarMenuAction::MoveTo => {
                self.state.open_move_to_dialog(target);
                cx.notify();
            }

            SidebarMenuAction::Delete => {
                self.state.request_delete(target);
                cx.notify();
            }
        }
    }

    pub fn handle_drop_item(
        &mut self,
        dragging: std::path::PathBuf,
        target: std::path::PathBuf,
        placement: crate::stores::workspace_store::DragPlacement,
        cx: &mut Context<Self>,
    ) {
        if let Some(root) = self.state.workspace_store.workspace_root.clone() {
            let srv = self.state.workspace_service.clone();
            let target_parent = match placement {
                crate::stores::workspace_store::DragPlacement::Inside => target,
                _ => target.parent().unwrap_or(&root).to_path_buf(),
            };
            self.state.workspace_store.set_drag_state(None);
            cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    if let Ok(()) = srv.reorder_item(&root, &dragging, &target_parent, 0).await {
                        let _ = this.update(&mut cx, |this, cx| {
                            let _ = this.state.reconcile_workspace_sync();
                            cx.notify();
                        });
                    }
                }
            })
            .detach();
        }
    }

}
