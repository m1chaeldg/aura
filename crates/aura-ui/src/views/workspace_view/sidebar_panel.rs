use super::AuraWorkspaceView;
use crate::input_registry::InputKey;
use crate::views::{SidebarEnvAction, SidebarView};
use aura_core::model::HttpMethod;
use gpui::*;
use gpui_kit::component::resizable::{resizable_panel, ResizablePanel};

impl AuraWorkspaceView {
    pub(super) fn render_sidebar_panel(&self, cx: &mut Context<Self>) -> ResizablePanel {
        let panel = if self.state.is_sidebar_collapsed {
            resizable_panel()
                .size(px(48.))
                .size_range(px(48.)..px(48.))
        } else {
            resizable_panel()
                .size(px(250.))
                .size_range(px(180.)..px(450.))
        };
        panel.child(SidebarView::render(
                &self.state,
                &self.sidebar_tree_state,
                self.sidebar_tree_meta.clone(),
                self.inputs.get_input(&InputKey::InlineRename),
                cx,
                |this, col_id, req_id_or_path, _window, cx| {
                    if col_id.is_empty() {
                        let path = std::path::PathBuf::from(req_id_or_path);
                        this.state.workspace_store.select_node(path.clone());
                        let _ = this.state.open_file_in_tab(&path);
                    } else {
                        this.state.open_collection_request(col_id, req_id_or_path);
                    }
                    cx.notify();
                },
                |this, _window, cx| {
                    this.spawn_create_new_request(cx);
                },
                |this, folder_path, _window, cx| {
                    this.state
                        .workspace_store
                        .select_node(folder_path.to_path_buf());
                    this.state.toggle_folder(folder_path);
                    cx.notify();
                },
                |this, target_path, is_folder, pos, _window, cx| {
                    this.state
                        .workspace_store
                        .select_node(target_path.to_path_buf());
                    this.state.workspace_store.open_context_menu(
                        target_path.to_path_buf(),
                        is_folder,
                        pos,
                    );
                    cx.notify();
                },
                |this, folder_path, _window, cx| {
                    Self::spawn_create_request_in_folder(this, folder_path, cx);
                },
                |this, item_path, _window, cx| {
                    Self::spawn_duplicate_item(this, item_path, cx);
                },
                |this, dragging_path, target_parent, placement, _window, cx| {
                    this.handle_drop_item(
                        dragging_path.to_path_buf(),
                        target_parent.to_path_buf(),
                        placement,
                        cx,
                    );
                },
                |this, env_action, _window, cx| {
                    match env_action {
                        SidebarEnvAction::New => this.state.open_new_environment_editor(),
                        SidebarEnvAction::Paste => {
                            let _ = this.state.paste_environment();
                        }
                        SidebarEnvAction::Edit(id) => this.state.open_environment_editor(&id),
                        SidebarEnvAction::Activate(id) => {
                            if this.state.active_environment_id.as_deref() == Some(id.as_str()) {
                                this.state.set_active_environment(None);
                            } else {
                                this.state.set_active_environment(Some(id));
                            }
                        }
                        SidebarEnvAction::Duplicate(id) => {
                            let _ = this.state.duplicate_environment(&id);
                        }
                        SidebarEnvAction::Copy(id) => this.state.copy_environment(&id),
                        SidebarEnvAction::Delete(id) => {
                            let _ = this.state.delete_environment(&id);
                        }
                    }
                    cx.notify();
                },
            ))
    }

    pub(super) fn spawn_create_new_request(&mut self, cx: &mut Context<Self>) {
        let Some(root) = self.state.workspace_store.workspace_root.clone() else {
            self.state.new_request();
            cx.notify();
            return;
        };

        let target_dir = match self.state.workspace_store.selected_path.as_ref() {
            Some(p) if p.starts_with(&root) && p.is_dir() => p.clone(),
            Some(p) if p.starts_with(&root) => p.parent().unwrap_or(&root).to_path_buf(),
            _ => root,
        };

        Self::spawn_create_request_in_folder(self, &target_dir, cx);
    }

    fn spawn_create_request_in_folder(
        this: &mut Self,
        folder_path: &std::path::Path,
        cx: &mut Context<Self>,
    ) {
        let Some(root) = this.state.workspace_store.workspace_root.clone() else {
            return;
        };
        let srv = this.state.workspace_service.clone();
        let folder = folder_path.to_path_buf();
        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                if let Ok(p) = srv
                    .create_request(&root, &folder, "Untitled Request", HttpMethod::GET)
                    .await
                {
                    let _ = this.update(&mut cx, |this, cx| {
                        let _ = this.state.open_file_in_tab(&p);
                        let _ = this.state.reconcile_workspace_sync();
                        this.state.workspace_store.select_node(p);
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    fn spawn_duplicate_item(
        this: &mut Self,
        item_path: &std::path::Path,
        cx: &mut Context<Self>,
    ) {
        let Some(root) = this.state.workspace_store.workspace_root.clone() else {
            return;
        };
        let srv = this.state.workspace_service.clone();
        let item = item_path.to_path_buf();
        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                if let Ok(p) = srv.duplicate_item(&root, &item).await {
                    let _ = this.update(&mut cx, |this, cx| {
                        if p.is_file() {
                            let _ = this.state.open_file_in_tab(&p);
                        }
                        let _ = this.state.reconcile_workspace_sync();
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }
}
