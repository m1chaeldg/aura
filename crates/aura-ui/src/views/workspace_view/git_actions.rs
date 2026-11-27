//! Asynchronous Git operations offloaded to GPUI's background executor.

use super::AuraWorkspaceView;
use gpui::*;
use std::path::PathBuf;

impl AuraWorkspaceView {
    pub(super) fn spawn_git_refresh(&mut self, cx: &mut Context<Self>) {
        let dir = self.state.git_workspace_dir();
        let selected = self.state.git_selected_file.clone();
        self.state.git_is_busy = true;
        self.state.git_error = None;
        cx.notify();

        let bg = cx.background_executor().spawn(async move {
            let snap = aura_sync::git_status(&dir)?;
            let diff = if let Some((ref path, staged)) = selected {
                let still_present = if staged {
                    snap.staged.iter().any(|f| &f.path == path)
                } else {
                    snap.unstaged.iter().any(|f| &f.path == path)
                };
                if still_present {
                    aura_sync::git_diff(&dir, path, staged).unwrap_or_default()
                } else {
                    String::new()
                }
            } else if let Some(first) = snap.unstaged.first().or_else(|| snap.staged.first()) {
                aura_sync::git_diff(&dir, &first.path, first.staged).unwrap_or_default()
            } else {
                String::new()
            };
            Ok::<_, anyhow::Error>((snap, diff))
        });

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let res = bg.await;
                let _ = this.update(&mut cx, |this, cx| {
                    this.state.git_is_busy = false;
                    match res {
                        Ok((snap, diff)) => {
                            let auto_sel = if this.state.git_selected_file.is_none() {
                                snap.unstaged
                                    .first()
                                    .or_else(|| snap.staged.first())
                                    .map(|f| (f.path.clone(), f.staged))
                            } else {
                                this.state.git_selected_file.clone().filter(|(p, staged)| {
                                    if *staged {
                                        snap.staged.iter().any(|f| &f.path == p)
                                    } else {
                                        snap.unstaged.iter().any(|f| &f.path == p)
                                    }
                                })
                            };
                            this.state.git_selected_file = auto_sel;
                            this.state.git_selected_diff = diff;
                            this.state.git_snapshot = Some(snap);
                            this.state.git_error = None;
                        }
                        Err(e) => {
                            this.state.git_error = Some(format!("{e:#}"));
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub(super) fn spawn_git_select_file(&mut self, path: String, staged: bool, cx: &mut Context<Self>) {
        let dir = self.state.git_workspace_dir();
        self.state.git_selected_file = Some((path.clone(), staged));
        self.state.git_error = None;
        cx.notify();

        let p_bg = path.clone();
        let bg = cx
            .background_executor()
            .spawn(async move { aura_sync::git_diff(&dir, &p_bg, staged) });

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let res = bg.await;
                let _ = this.update(&mut cx, |this, cx| {
                    if this.state.git_selected_file.as_ref() == Some(&(path, staged)) {
                        match res {
                            Ok(diff) => this.state.git_selected_diff = diff,
                            Err(e) => this.state.git_error = Some(format!("{e:#}")),
                        }
                        cx.notify();
                    }
                });
            }
        })
        .detach();
    }

    fn spawn_git_mutation<F>(
        &mut self,
        reload_workspace: bool,
        clear_commit_msg: bool,
        clear_branch_input: bool,
        cx: &mut Context<Self>,
        op: F,
    ) where
        F: FnOnce(PathBuf) -> anyhow::Result<String> + Send + 'static,
    {
        let dir = self.state.git_workspace_dir();
        let selected = self.state.git_selected_file.clone();
        self.state.git_is_busy = true;
        self.state.git_error = None;
        cx.notify();

        let bg = cx.background_executor().spawn(async move {
            let output = op(dir.clone())?;
            let snap = aura_sync::git_status(&dir)?;
            let diff = if let Some((ref path, staged)) = selected {
                let exists = if staged {
                    snap.staged.iter().any(|f| &f.path == path)
                } else {
                    snap.unstaged.iter().any(|f| &f.path == path)
                };
                if exists {
                    aura_sync::git_diff(&dir, path, staged).unwrap_or_default()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            Ok::<_, anyhow::Error>((output, snap, diff))
        });

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let res = bg.await;
                let _ = this.update(&mut cx, |this, cx| {
                    this.state.git_is_busy = false;
                    match res {
                        Ok((output, snap, diff)) => {
                            if !output.trim().is_empty() {
                                this.state.git_last_output = Some(output.clone());
                                this.state.status_message = Some(output);
                            }
                            if clear_commit_msg {
                                this.state.git_commit_message.clear();
                            }
                            if clear_branch_input {
                                this.state.git_new_branch_name.clear();
                            }
                            this.state.git_selected_diff = diff;
                            this.state.git_snapshot = Some(snap);
                            this.state.git_error = None;
                            if reload_workspace {
                                let _ = this.state.reconcile_workspace_sync();
                            }
                        }
                        Err(e) => {
                            let msg = format!("{e:#}");
                            this.state.git_error = Some(msg.clone());
                            this.state.status_message = Some(format!("Git error: {msg}"));
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub(super) fn spawn_git_init(&mut self, cx: &mut Context<Self>) {
        self.spawn_git_mutation(false, false, false, cx, |dir| aura_sync::git_init(&dir));
    }

    pub(super) fn spawn_git_stage(&mut self, path: Option<String>, cx: &mut Context<Self>) {
        self.spawn_git_mutation(false, false, false, cx, move |dir| {
            aura_sync::git_stage(&dir, path.as_deref())?;
            Ok(match path {
                Some(p) => format!("Staged {p}"),
                None => "Staged all changes".to_string(),
            })
        });
    }

    pub(super) fn spawn_git_unstage(&mut self, path: Option<String>, cx: &mut Context<Self>) {
        self.spawn_git_mutation(false, false, false, cx, move |dir| {
            aura_sync::git_unstage(&dir, path.as_deref())?;
            Ok(match path {
                Some(p) => format!("Unstaged {p}"),
                None => "Unstaged all changes".to_string(),
            })
        });
    }

    pub(super) fn spawn_git_discard(&mut self, path: String, is_untracked: bool, cx: &mut Context<Self>) {
        self.spawn_git_mutation(true, false, false, cx, move |dir| {
            aura_sync::git_discard_file(&dir, &path, is_untracked)?;
            Ok(format!("Discarded changes in {path}"))
        });
    }

    pub(super) fn spawn_git_commit(&mut self, stage_all_if_none: bool, cx: &mut Context<Self>) {
        let msg = self.state.git_commit_message.clone();
        self.spawn_git_mutation(false, true, false, cx, move |dir| {
            aura_sync::git_commit(&dir, &msg, stage_all_if_none)
        });
    }

    pub(super) fn spawn_git_checkout_branch(&mut self, branch: String, cx: &mut Context<Self>) {
        self.spawn_git_mutation(true, false, false, cx, move |dir| {
            let out = aura_sync::git_checkout_branch(&dir, &branch)?;
            Ok(if out.is_empty() {
                format!("Switched to branch '{branch}'")
            } else {
                out
            })
        });
    }

    pub(super) fn spawn_git_create_branch(&mut self, cx: &mut Context<Self>) {
        let branch = self.state.git_new_branch_name.clone();
        self.spawn_git_mutation(false, false, true, cx, move |dir| {
            let out = aura_sync::git_create_branch(&dir, &branch)?;
            Ok(if out.is_empty() {
                format!("Created and switched to branch '{}'", branch.trim())
            } else {
                out
            })
        });
    }

    pub(super) fn spawn_git_pull(&mut self, cx: &mut Context<Self>) {
        self.spawn_git_mutation(true, false, false, cx, |dir| aura_sync::git_pull(&dir));
    }

    pub(super) fn spawn_git_push(&mut self, cx: &mut Context<Self>) {
        self.spawn_git_mutation(false, false, false, cx, |dir| aura_sync::git_push(&dir));
    }
}
