//! Modular Built-in Git UI modal: status, diff viewer, stage/commit, branches, and push/pull.

use super::AuraWorkspaceView;
use crate::input_registry::InputKey;
use crate::theme::{icon, subtle_surface};
use aura_sync::{GitFileEntry, GitFileStatusKind, GitStatusSnapshot};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::base::Disableable as _;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::dialog::{Dialog, DialogFooter};
use gpui_kit::component::input::Input;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};

impl AuraWorkspaceView {
    pub(super) fn render_git_modal(&self, el: Div, cx: &mut Context<Self>) -> Div {
        if !self.state.is_git_modal_open {
            return el;
        }

        let border = cx.theme().border;
        let muted = cx.theme().muted_foreground;
        let fg = cx.theme().foreground;
        let radius = cx.theme().radius;
        let surface = subtle_surface(cx);
        let busy = self.state.git_is_busy;
        let snap_opt = self.state.git_snapshot.clone();
        let dir_display = self.state.git_workspace_dir().display().to_string();

        let content = match snap_opt {
            Some(ref snap) if snap.is_repo => {
                self.render_git_repo_body(snap, border, muted, fg, radius, surface, busy, cx)
            }
            _ => self.render_git_uninitialized_body(&dir_display, border, muted, fg, radius, surface, busy, cx),
        };

        el.child(
            Dialog::new(cx)
                .w(px(900.0))
                .title("Git Version Control")
                .on_close({
                    let view = cx.entity().clone();
                    move |_, _, app| {
                        let _ = view.update(app, |this, cx| {
                            this.state.is_git_modal_open = false;
                            cx.notify();
                        });
                    }
                })
                .child(content)
                .footer(
                    DialogFooter::new()
                        .child(
                            Button::new("git-refresh-footer-btn")
                                .icon(icon(IconName::RefreshCw))
                                .label(if busy { "Working..." } else { "Refresh Status" })
                                .small()
                                .outline()
                                .rounded(ButtonRounded::Medium)
                                .disabled(busy)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.spawn_git_refresh(cx);
                                })),
                        )
                        .child(
                            Button::new("git-close-btn")
                                .label("Close")
                                .primary()
                                .small()
                                .rounded(ButtonRounded::Medium)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.state.is_git_modal_open = false;
                                    cx.notify();
                                })),
                        ),
                ),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_git_uninitialized_body(
        &self,
        dir_display: &str,
        border: Hsla,
        muted: Hsla,
        fg: Hsla,
        radius: Pixels,
        surface: Hsla,
        busy: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        v_flex()
            .gap_3()
            .p_4()
            .rounded(radius)
            .bg(surface)
            .border_1()
            .border_color(border)
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(fg)
                    .child("This workspace is not yet a Git repository"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child(format!("Workspace path: {dir_display}")),
            )
            .when_some(self.state.git_error.clone(), |d, err| {
                d.child(div().text_xs().text_color(cx.theme().red).child(err))
            })
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("git-init-repo-btn")
                            .icon(icon(IconName::Plus))
                            .label("Initialize Git Repository")
                            .primary()
                            .small()
                            .rounded(ButtonRounded::Medium)
                            .disabled(busy)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.spawn_git_init(cx);
                            })),
                    ),
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_git_repo_body(
        &self,
        snap: &GitStatusSnapshot,
        border: Hsla,
        muted: Hsla,
        fg: Hsla,
        radius: Pixels,
        surface: Hsla,
        busy: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let sync_label = match (snap.ahead, snap.behind) {
            (0, 0) => None,
            (a, b) => Some(format!("ahead {a} / behind {b}")),
        };

        v_flex()
            .gap_2p5()
            .max_h(px(580.0))
            // Header toolbar: branch badge + remote Pull/Push
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .p_2()
                    .rounded(radius)
                    .bg(surface)
                    .border_1()
                    .border_color(border)
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(Tag::info().small().child(format!("Branch: {}", snap.branch)))
                            .when_some(sync_label, |el, s| {
                                el.child(Tag::secondary().xsmall().child(s))
                            })
                            .when(!snap.has_remote, |el| {
                                el.child(
                                    div()
                                        .text_xs()
                                        .text_color(muted)
                                        .child("(No remote configured)"),
                                )
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_1p5()
                            .child(
                                Button::new("git-pull-btn")
                                    .label("Pull")
                                    .xsmall()
                                    .outline()
                                    .rounded(ButtonRounded::Small)
                                    .disabled(busy || !snap.has_remote)
                                    .tooltip("Pull changes from remote (git pull)")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.spawn_git_pull(cx);
                                    })),
                            )
                            .child(
                                Button::new("git-push-btn")
                                    .label("Push")
                                    .xsmall()
                                    .primary()
                                    .rounded(ButtonRounded::Small)
                                    .disabled(busy || !snap.has_remote)
                                    .tooltip("Push commits to remote (git push)")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.spawn_git_push(cx);
                                    })),
                            ),
                    ),
            )
            // Branch switcher & new branch creation
            .child(self.render_git_branches_bar(snap, border, muted, radius, surface, busy, cx))
            // Split View: Changes + Commit on Left | Diff Viewer on Right
            .child(
                h_flex()
                    .gap_2p5()
                    .h(px(330.0))
                    .items_stretch()
                    .child(self.render_git_changes_pane(snap, border, muted, fg, radius, surface, busy, cx))
                    .child(self.render_git_diff_pane(border, muted, fg, radius, surface, cx)),
            )
            // Recent commits strip
            .when(!snap.recent_commits.is_empty(), |el| {
                el.child(self.render_git_recent_commits(snap, border, muted, fg, radius, surface))
            })
            // Status / error banners
            .when_some(self.state.git_last_output.clone(), |el, out| {
                el.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().green)
                        .truncate()
                        .child(out),
                )
            })
            .when_some(self.state.git_error.clone(), |el, err| {
                el.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().red)
                        .child(err),
                )
            })
    }

    #[allow(clippy::too_many_arguments)]
    fn render_git_branches_bar(
        &self,
        snap: &GitStatusSnapshot,
        border: Hsla,
        muted: Hsla,
        radius: Pixels,
        surface: Hsla,
        busy: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        h_flex()
            .justify_between()
            .gap_2()
            .p_2()
            .rounded(radius)
            .bg(surface)
            .border_1()
            .border_color(border)
            .child(
                h_flex()
                    .gap_1()
                    .flex_wrap()
                    .flex_1()
                    .items_center()
                    .child(div().text_xs().text_color(muted).child("Branches:"))
                    .children(snap.branches.iter().enumerate().map(|(idx, b)| {
                        let name = b.name.clone();
                        let click_name = name.clone();
                        let btn = Button::new(ElementId::NamedInteger(
                            "git-branch-chip".into(),
                            idx as u64,
                        ))
                        .label(name)
                        .xsmall()
                        .rounded(ButtonRounded::Small)
                        .disabled(busy || b.is_current)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.spawn_git_checkout_branch(click_name.clone(), cx);
                        }));
                        if b.is_current { btn.primary() } else { btn.outline() }
                    })),
            )
            .child(
                h_flex()
                    .gap_1p5()
                    .items_center()
                    .child(div().w(px(170.0)).when_some(
                        self.inputs.get_input(&InputKey::GitNewBranchName),
                        |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                    ))
                    .child(
                        Button::new("git-create-branch-btn")
                            .icon(icon(IconName::Plus))
                            .label("New Branch")
                            .xsmall()
                            .secondary()
                            .rounded(ButtonRounded::Small)
                            .disabled(busy || self.state.git_new_branch_name.trim().is_empty())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.spawn_git_create_branch(cx);
                            })),
                    ),
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_git_changes_pane(
        &self,
        snap: &GitStatusSnapshot,
        border: Hsla,
        muted: Hsla,
        fg: Hsla,
        radius: Pixels,
        surface: Hsla,
        busy: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let can_commit = !busy
            && !self.state.git_commit_message.trim().is_empty()
            && snap.total_changes() > 0;
        let has_staged = !snap.staged.is_empty();

        v_flex()
            .w(px(320.0))
            .h_full()
            .gap_2()
            .child(
                v_flex()
                    .id("git-files-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .gap_2()
                    .p_2()
                    .rounded(radius)
                    .bg(surface)
                    .border_1()
                    .border_color(border)
                    // Staged files section
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(fg)
                                    .child(format!("STAGED CHANGES ({})", snap.staged.len())),
                            )
                            .when(has_staged, |el| {
                                el.child(
                                    Button::new("git-unstage-all-btn")
                                        .label("Unstage All")
                                        .xsmall()
                                        .ghost()
                                        .disabled(busy)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.spawn_git_unstage(None, cx);
                                        })),
                                )
                            }),
                    )
                    .children(snap.staged.iter().enumerate().map(|(idx, entry)| {
                        self.render_git_file_row(idx, entry, true, border, fg, radius, busy, cx)
                    }))
                    // Unstaged files section
                    .child(
                        h_flex()
                            .justify_between()
                            .pt_1()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(fg)
                                    .child(format!("WORKING CHANGES ({})", snap.unstaged.len())),
                            )
                            .when(!snap.unstaged.is_empty(), |el| {
                                el.child(
                                    Button::new("git-stage-all-btn")
                                        .label("Stage All")
                                        .xsmall()
                                        .ghost()
                                        .disabled(busy)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.spawn_git_stage(None, cx);
                                        })),
                                )
                            }),
                    )
                    .when(snap.total_changes() == 0, |el| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(muted)
                                .py_2()
                                .child("Working tree clean — no uncommitted changes."),
                        )
                    })
                    .children(snap.unstaged.iter().enumerate().map(|(idx, entry)| {
                        self.render_git_file_row(idx, entry, false, border, fg, radius, busy, cx)
                    })),
            )
            // Commit composer box
            .child(
                v_flex()
                    .gap_1p5()
                    .p_2()
                    .rounded(radius)
                    .bg(surface)
                    .border_1()
                    .border_color(border)
                    .child(div().when_some(
                        self.inputs.get_input(&InputKey::GitCommitMessage),
                        |d, inp| d.child(Input::new(inp).xsmall().w_full()),
                    ))
                    .child(
                        Button::new("git-commit-btn")
                            .icon(icon(IconName::Check))
                            .label(if has_staged {
                                "Commit Staged"
                            } else {
                                "Stage All & Commit"
                            })
                            .xsmall()
                            .primary()
                            .rounded(ButtonRounded::Small)
                            .disabled(!can_commit)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.spawn_git_commit(true, cx);
                            })),
                    ),
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_git_file_row(
        &self,
        idx: usize,
        entry: &GitFileEntry,
        staged: bool,
        border: Hsla,
        fg: Hsla,
        radius: Pixels,
        busy: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let is_selected =
            self.state.git_selected_file.as_ref() == Some(&(entry.path.clone(), staged));
        let kind_color = match entry.kind {
            GitFileStatusKind::Added | GitFileStatusKind::Untracked => cx.theme().green,
            GitFileStatusKind::Deleted => cx.theme().red,
            GitFileStatusKind::Conflicted => cx.theme().red,
            _ => cx.theme().yellow,
        };
        let path_select = entry.path.clone();
        let path_action = entry.path.clone();
        let path_discard = entry.path.clone();
        let is_untracked = entry.kind == GitFileStatusKind::Untracked;
        let prefix = if staged { "staged" } else { "work" };

        h_flex()
            .id(ElementId::Name(format!("git-row-{prefix}-{idx}").into()))
            .justify_between()
            .gap_1p5()
            .px_2()
            .py_1()
            .rounded(radius)
            .border_1()
            .border_color(if is_selected { cx.theme().primary } else { border })
            .bg(if is_selected {
                cx.theme().primary.opacity(0.14)
            } else {
                gpui::transparent_black()
            })
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.spawn_git_select_file(path_select.clone(), staged, cx);
            }))
            .child(
                h_flex()
                    .gap_1p5()
                    .flex_1()
                    .overflow_hidden()
                    .items_center()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(kind_color)
                            .child(entry.kind.short_code()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(fg)
                            .truncate()
                            .child(entry.path.clone()),
                    ),
            )
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        Button::new(ElementId::Name(format!("git-act-{prefix}-{idx}").into()))
                            .label(if staged { "-" } else { "+" })
                            .xsmall()
                            .ghost()
                            .disabled(busy)
                            .tooltip(if staged { "Unstage file" } else { "Stage file" })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if staged {
                                    this.spawn_git_unstage(Some(path_action.clone()), cx);
                                } else {
                                    this.spawn_git_stage(Some(path_action.clone()), cx);
                                }
                            })),
                    )
                    .when(!staged, |el| {
                        el.child(
                            Button::new(ElementId::Name(format!("git-disc-{idx}").into()))
                                .icon(icon(IconName::X))
                                .xsmall()
                                .ghost()
                                .disabled(busy)
                                .tooltip("Discard changes")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.spawn_git_discard(path_discard.clone(), is_untracked, cx);
                                })),
                        )
                    }),
            )
    }

    fn render_git_diff_pane(
        &self,
        border: Hsla,
        muted: Hsla,
        fg: Hsla,
        radius: Pixels,
        surface: Hsla,
        cx: &Context<Self>,
    ) -> Div {
        let header_title = match &self.state.git_selected_file {
            Some((path, staged)) => format!(
                "{path} ({})",
                if *staged { "Staged Diff" } else { "Working Diff" }
            ),
            None => "Diff Viewer".to_string(),
        };
        let diff_text = &self.state.git_selected_diff;
        let green = cx.theme().green;
        let red = cx.theme().red;
        let blue = cx.theme().blue;

        v_flex()
            .flex_1()
            .h_full()
            .p_2()
            .gap_1p5()
            .rounded(radius)
            .bg(surface)
            .border_1()
            .border_color(border)
            .overflow_hidden()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(fg)
                    .truncate()
                    .child(header_title),
            )
            .child(
                v_flex()
                    .id("git-diff-lines-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .p_2()
                    .rounded(radius)
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(border)
                    .when(diff_text.trim().is_empty(), |el| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(muted)
                                .child("Select a changed file on the left to inspect its diff."),
                        )
                    })
                    .children(diff_text.lines().take(400).map(|line| {
                        let (line_fg, line_bg) = if line.starts_with('+') && !line.starts_with("+++") {
                            (green, green.opacity(0.10))
                        } else if line.starts_with('-') && !line.starts_with("---") {
                            (red, red.opacity(0.10))
                        } else if line.starts_with("@@") {
                            (blue, blue.opacity(0.08))
                        } else if line.starts_with("diff ") || line.starts_with("+++") || line.starts_with("---") {
                            (muted, gpui::transparent_black())
                        } else {
                            (fg, gpui::transparent_black())
                        };
                        div()
                            .w_full()
                            .px_1p5()
                            .py_0p5()
                            .bg(line_bg)
                            .text_xs()
                            .text_color(line_fg)
                            .child(line.to_string())
                    })),
            )
    }

    fn render_git_recent_commits(
        &self,
        snap: &GitStatusSnapshot,
        border: Hsla,
        muted: Hsla,
        fg: Hsla,
        radius: Pixels,
        surface: Hsla,
    ) -> Div {
        v_flex()
            .gap_1()
            .p_2()
            .rounded(radius)
            .bg(surface)
            .border_1()
            .border_color(border)
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(muted)
                    .child("RECENT COMMITS"),
            )
            .children(snap.recent_commits.iter().take(4).map(|c| {
                h_flex()
                    .justify_between()
                    .gap_2()
                    .text_xs()
                    .child(
                        h_flex()
                            .gap_2()
                            .flex_1()
                            .overflow_hidden()
                            .child(Tag::secondary().xsmall().child(c.short_hash.clone()))
                            .child(div().text_color(fg).truncate().child(c.message.clone())),
                    )
                    .child(
                        div()
                            .text_color(muted)
                            .child(format!("{} • {}", c.author, c.relative_time)),
                    )
            }))
    }
}
