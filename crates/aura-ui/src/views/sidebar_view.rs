//! Sidebar View: Displays workspace collections, folders, and request items using gpui_kit TreeView.

use crate::app_state::AuraAppState;
use crate::stores::workspace_store::{DragPlacement, WorkspaceTreeNode};
use crate::theme::icon;
use aura_core::model::HttpMethod;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::collapsible::Collapsible;
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::sidebar::{SidebarFooter, SidebarHeader};
use gpui_kit::component::tag::{Tag, TagVariant};
use gpui_kit::component::tree::{tree, TreeEntry, TreeItem, TreeState};
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone)]
pub enum SidebarNodeMeta {
    Folder {
        name: String,
        path: std::path::PathBuf,
        is_expanded: bool,
    },
    Request {
        name: String,
        method: HttpMethod,
        col_id: String,
        req_id_or_path: String,
        disk_path: Option<std::path::PathBuf>,
    },
}

#[derive(Debug, Clone)]
pub enum SidebarEnvAction {
    New,
    Paste,
    Edit(String),
    Activate(String),
    Duplicate(String),
    Copy(String),
    Delete(String),
}

pub struct SidebarView;

impl SidebarView {
    /// Converts workspace tree or collections into `gpui_kit` `TreeItem`s and metadata map.
    pub fn build_tree_items(
        state: &AuraAppState,
    ) -> (Vec<TreeItem>, HashMap<String, SidebarNodeMeta>) {
        let mut items = Vec::new();
        let mut meta = HashMap::new();

        if !state.workspace_store.tree.is_empty() {
            for node in &state.workspace_store.tree {
                if let crate::stores::workspace_store::WorkspaceTreeNode::Folder { name, .. } = node {
                    if aura_sync::workspace_meta::is_ignored_workspace_dir(name) {
                        continue;
                    }
                }
                items.push(node_to_tree_item(node, &mut meta));
            }
        } else {
            for col in &state.collections {
                if aura_sync::workspace_meta::is_ignored_workspace_dir(&col.name) {
                    continue;
                }
                let col_key = format!("col:{}", col.id);
                meta.insert(
                    col_key.clone(),
                    SidebarNodeMeta::Folder {
                        name: col.name.clone(),
                        path: std::path::PathBuf::from(&col.id),
                        is_expanded: true,
                    },
                );
                let mut children = Vec::new();
                for req in &col.requests {
                    let req_key = format!("colreq:{}:{}", col.id, req.id);
                    meta.insert(
                        req_key.clone(),
                        SidebarNodeMeta::Request {
                            name: req.name.clone(),
                            method: req.method,
                            col_id: col.id.clone(),
                            req_id_or_path: req.id.clone(),
                            disk_path: None,
                        },
                    );
                    children.push(TreeItem::new(req_key, req.name.clone()));
                }
                items.push(
                    TreeItem::new(col_key, col.name.clone())
                        .expanded(true)
                        .children(children),
                );
            }
        }

        (items, meta)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render<V: 'static>(
        state: &AuraAppState,
        tree_state: &Entity<TreeState>,
        node_meta: Rc<HashMap<String, SidebarNodeMeta>>,
        inline_rename_input: Option<&Entity<InputState>>,
        cx: &mut Context<V>,
        on_select: impl Fn(&mut V, &str, &str, &mut Window, &mut Context<V>) + 'static + Copy,
        on_new: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        on_toggle_folder: impl Fn(&mut V, &std::path::Path, &mut Window, &mut Context<V>)
            + 'static
            + Copy,
        on_context_menu: impl Fn(&mut V, &std::path::Path, bool, (f32, f32), &mut Window, &mut Context<V>)
            + 'static
            + Copy,
        on_new_in_folder: impl Fn(&mut V, &std::path::Path, &mut Window, &mut Context<V>)
            + 'static
            + Copy,
        on_duplicate: impl Fn(&mut V, &std::path::Path, &mut Window, &mut Context<V>) + 'static + Copy,
        on_drop_item: impl Fn(
                &mut V,
                &std::path::Path,
                &std::path::Path,
                DragPlacement,
                &mut Window,
                &mut Context<V>,
            )
            + 'static
            + Copy,
        on_env_action: impl Fn(&mut V, SidebarEnvAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> impl IntoElement {
        let sidebar_bg = cx.theme().sidebar;
        let sidebar_border = cx.theme().sidebar_border;
        let sidebar_fg = cx.theme().sidebar_foreground;
        let text_muted = cx.theme().muted_foreground;
        let green_color = cx.theme().success;
        let active_bg = cx.theme().list_active;
        let hover_bg = cx.theme().list_hover;
        let primary_color = cx.theme().primary;

        let is_empty = state.workspace_store.tree.is_empty() && state.collections.is_empty();

        let workspace_title = state
            .workspace_store
            .metadata
            .as_ref()
            .map(|m| m.name.clone())
            .unwrap_or_else(|| {
                if !state.collections.is_empty() {
                    format!("COLLECTIONS ({})", state.collections.len())
                } else {
                    "WORKSPACE".to_string()
                }
            });

        let root_for_drop = state.workspace_store.workspace_root.clone();

        let mut header_div = div()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .child(
                div()
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(text_muted)
                    .child(workspace_title),
            )
            .child(
                Button::new("sidebar-new-btn")
                    .label("+ Req")
                    .xsmall()
                    .ghost()
                    .rounded(ButtonRounded::Small)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        on_new(this, window, cx);
                    })),
            );

        if let Some(root_path) = root_for_drop {
            header_div = header_div.on_drop(cx.listener(
                move |this, payload: &crate::app_state::DragItemPayload, window, cx| {
                    if payload.path != root_path {
                        on_drop_item(
                            this,
                            &payload.path,
                            &root_path,
                            DragPlacement::Inside,
                            window,
                            cx,
                        );
                    }
                },
            ));
        }

        let content_el: AnyElement = if is_empty {
            Empty::new()
                .header(
                    EmptyHeader::new().description(
                        EmptyDescription::new().child("No requests or collections in workspace."),
                    ),
                )
                .into_any_element()
        } else {
            let view_entity = cx.entity().clone();
            let selected_path = state.workspace_store.selected_path.clone();
            let rename_path = state.inline_rename.as_ref().map(|r| r.path.clone());
            let rename_input_cloned = inline_rename_input.cloned();
            let drag_state = state.workspace_store.drag_state.clone();

            tree(tree_state, move |ix, entry: &TreeEntry, is_selected, _window, _app| {
                let id_str = entry.item().id.as_ref();
                let depth = entry.depth();
                let padding_left = px(8.0 + (depth as f32 * 12.0));
                let meta_entry = node_meta.get(id_str).cloned();

                match meta_entry {
                    Some(SidebarNodeMeta::Folder {
                        name,
                        path,
                        is_expanded,
                    }) => {
                        let is_node_selected =
                            is_selected || selected_path.as_deref() == Some(path.as_path());
                        let is_rename = rename_path.as_deref() == Some(path.as_path());
                        let is_inside_drop = drag_state
                            .as_ref()
                            .map(|d| {
                                d.hover_target_path == path
                                    && d.placement == DragPlacement::Inside
                            })
                            .unwrap_or(false);

                        let path_click = path.clone();
                        let path_ctx = path.clone();
                        let path_add = path.clone();
                        let path_drop = path.clone();
                        let view_click = view_entity.clone();
                        let view_ctx = view_entity.clone();
                        let view_add = view_entity.clone();
                        let view_more = view_entity.clone();
                        let view_drop = view_entity.clone();

                        let mut item = ListItem::new(ElementId::NamedInteger("tree-folder".into(), ix as u64))
                            .selected(is_node_selected)
                            .pl(padding_left)
                            .pr_2()
                            .py_1()
                            .text_xs()
                            .on_drag(
                                crate::app_state::DragItemPayload {
                                    path: path.clone(),
                                    is_folder: true,
                                },
                                |payload, _, _, cx| cx.new(|_| payload.clone()),
                            )
                            .on_drop(move |payload: &crate::app_state::DragItemPayload, window, app| {
                                if payload.path != path_drop && !path_drop.starts_with(&payload.path) {
                                    let _ = view_drop.update(app, |this, cx| {
                                        on_drop_item(
                                            this,
                                            &payload.path,
                                            &path_drop,
                                            DragPlacement::Inside,
                                            window,
                                            cx,
                                        );
                                    });
                                }
                            })
                            .on_click(move |_, window, app| {
                                if !is_rename {
                                    let _ = view_click.update(app, |this, cx| {
                                        on_toggle_folder(this, &path_click, window, cx);
                                    });
                                }
                            })
                            .on_mouse_down(MouseButton::Right, move |event, window, app| {
                                let clamped = clamp_menu_pos(event.position, window);
                                let _ = view_ctx.update(app, |this, cx| {
                                    on_context_menu(this, &path_ctx, true, clamped, window, cx);
                                });
                            })
                            .suffix(move |_, _| {
                                let p_add = path_add.clone();
                                let p_more = path.clone();
                                let v_add = view_add.clone();
                                let v_more = view_more.clone();
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_0p5()
                                    .child(
                                        Button::new(ElementId::NamedInteger("folder-add".into(), ix as u64))
                                            .label("+")
                                            .xsmall()
                                            .ghost()
                                            .on_click(move |_, window, app| {
                                                let _ = v_add.update(app, |this, cx| {
                                                    on_new_in_folder(this, &p_add, window, cx);
                                                });
                                            }),
                                    )
                                    .child(
                                        Button::new(ElementId::NamedInteger("folder-more".into(), ix as u64))
                                            .label("⋯")
                                            .xsmall()
                                            .ghost()
                                            .on_mouse_down(MouseButton::Left, |_, _, app| {
                                                app.stop_propagation();
                                            })
                                            .on_click(move |event, window, app| {
                                                app.stop_propagation();
                                                let clamped = clamp_menu_pos(event.position(), window);
                                                let _ = v_more.update(app, |this, cx| {
                                                    on_context_menu(this, &p_more, true, clamped, window, cx);
                                                });
                                            }),
                                    )
                                    .into_any_element()
                            });

                        if is_inside_drop {
                            item = item.border_1().border_color(primary_color);
                        }

                        let chevron_icon = if is_expanded {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        };
                        let folder_icon = if is_expanded {
                            IconName::FolderOpen
                        } else {
                            IconName::Folder
                        };

                        item.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .overflow_hidden()
                                .child(icon(chevron_icon).xsmall().text_color(text_muted))
                                .child(icon(folder_icon).xsmall().text_color(if is_expanded {
                                    primary_color
                                } else {
                                    text_muted
                                }))
                                .child(if is_rename {
                                    if let Some(ref input) = rename_input_cloned {
                                        Input::new(input)
                                            .id("sidebar-inline-rename-folder")
                                            .xsmall()
                                            .w_full()
                                            .into_any_element()
                                    } else {
                                        div().child(name).into_any_element()
                                    }
                                } else {
                                    div().child(name).into_any_element()
                                }),
                        )
                    }
                    Some(SidebarNodeMeta::Request {
                        name,
                        method,
                        col_id,
                        req_id_or_path,
                        disk_path,
                    }) => {
                        let is_node_selected = is_selected
                            || disk_path
                                .as_deref()
                                .map(|p| selected_path.as_deref() == Some(p))
                                .unwrap_or(false);
                        let is_rename = disk_path
                            .as_deref()
                            .map(|p| rename_path.as_deref() == Some(p))
                            .unwrap_or(false);

                        let c_id = col_id.clone();
                        let r_id = req_id_or_path.clone();
                        let view_click = view_entity.clone();
                        let disk_for_ctx = disk_path.clone();
                        let disk_for_dup = disk_path.clone();
                        let disk_for_more = disk_path.clone();
                        let disk_for_drag = disk_path.clone();
                        let disk_for_drop = disk_path.clone();
                        let view_ctx = view_entity.clone();
                        let view_dup = view_entity.clone();
                        let view_more = view_entity.clone();
                        let view_drop = view_entity.clone();

                        let mut item = ListItem::new(ElementId::NamedInteger("tree-req".into(), ix as u64))
                            .selected(is_node_selected)
                            .pl(padding_left)
                            .pr_2()
                            .py_1()
                            .text_xs()
                            .on_click(move |_, window, app| {
                                if !is_rename {
                                    let _ = view_click.update(app, |this, cx| {
                                        on_select(this, &c_id, &r_id, window, cx);
                                    });
                                }
                            });

                        if let Some(dp) = disk_for_drag {
                            item = item.on_drag(
                                crate::app_state::DragItemPayload {
                                    path: dp,
                                    is_folder: false,
                                },
                                |payload, _, _, cx| cx.new(|_| payload.clone()),
                            );
                        }

                        if let Some(dp) = disk_for_drop {
                            item = item.on_drop(move |payload: &crate::app_state::DragItemPayload, window, app| {
                                if payload.path != dp {
                                    if let Some(parent_dir) = dp.parent() {
                                        let target_dir = parent_dir.to_path_buf();
                                        let _ = view_drop.update(app, |this, cx| {
                                            on_drop_item(
                                                this,
                                                &payload.path,
                                                &target_dir,
                                                DragPlacement::Above,
                                                window,
                                                cx,
                                            );
                                        });
                                    }
                                }
                            });
                        }

                        if let Some(dp) = disk_for_ctx {
                            item = item.on_mouse_down(MouseButton::Right, move |event, window, app| {
                                let clamped = clamp_menu_pos(event.position, window);
                                let _ = view_ctx.update(app, |this, cx| {
                                    on_context_menu(this, &dp, false, clamped, window, cx);
                                });
                            });
                        }

                        if let (Some(dp_dup), Some(dp_more)) = (disk_for_dup, disk_for_more) {
                            item = item.suffix(move |_, _| {
                                let d1 = dp_dup.clone();
                                let d2 = dp_more.clone();
                                let v1 = view_dup.clone();
                                let v2 = view_more.clone();
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_0p5()
                                    .child(
                                        Button::new(ElementId::NamedInteger("req-dup".into(), ix as u64))
                                            .label("❐")
                                            .xsmall()
                                            .ghost()
                                            .on_click(move |_, window, app| {
                                                let _ = v1.update(app, |this, cx| {
                                                    on_duplicate(this, &d1, window, cx);
                                                });
                                            }),
                                    )
                                    .child(
                                        Button::new(ElementId::NamedInteger("req-more".into(), ix as u64))
                                            .label("⋯")
                                            .xsmall()
                                            .ghost()
                                            .on_mouse_down(MouseButton::Left, |_, _, app| {
                                                app.stop_propagation();
                                            })
                                            .on_click(move |event, window, app| {
                                                app.stop_propagation();
                                                let clamped = clamp_menu_pos(event.position(), window);
                                                let _ = v2.update(app, |this, cx| {
                                                    on_context_menu(this, &d2, false, clamped, window, cx);
                                                });
                                            }),
                                    )
                                    .into_any_element()
                            });
                        }

                        item.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .overflow_hidden()
                                .child(method_tag(method))
                                .child(if is_rename {
                                    if let Some(ref input) = rename_input_cloned {
                                        Input::new(input)
                                            .id("sidebar-inline-rename-file")
                                            .xsmall()
                                            .w_full()
                                            .into_any_element()
                                    } else {
                                        div().child(name).into_any_element()
                                    }
                                } else {
                                    div().child(name).into_any_element()
                                }),
                        )
                    }
                    None => ListItem::new(ElementId::NamedInteger("tree-unknown".into(), ix as u64))
                        .child(entry.item().label.clone()),
                }
            })
            .size_full()
            .into_any_element()
        };

        if state.is_sidebar_collapsed {
            return div()
                .id("workspace-sidebar")
                .flex()
                .flex_col()
                .items_center()
                .size_full()
                .py_3()
                .bg(sidebar_bg)
                .border_r_1()
                .border_color(sidebar_border)
                .child(
                    Button::new("sidebar-collapsed-new-btn")
                        .icon(icon(IconName::Plus))
                        .xsmall()
                        .ghost()
                        .rounded(ButtonRounded::Small)
                        .tooltip("New Request")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            on_new(this, window, cx);
                        })),
                );
        }

        div()
            .id("workspace-sidebar")
            .flex()
            .flex_col()
            .size_full()
            .bg(sidebar_bg)
            .border_r_1()
            .border_color(sidebar_border)
            .overflow_hidden()
            .child(
                div()
                    .flex_shrink_0()
                    .px_3()
                    .pt_3()
                    .pb_2()
                    .border_b_1()
                    .border_color(sidebar_border)
                    .child(SidebarHeader::new().child(header_div)),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .px_3()
                    .pt_2()
                    .pb_1()
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(text_muted)
                    .child("COLLECTIONS"),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_hidden()
                    .px_1()
                    .child(
                        Collapsible::new()
                            .open(true)
                            .size_full()
                            .content(content_el),
                    ),
            )
            .child({
                let has_clip = state.clipboard_environment.is_some();
                let active_env_id = state.active_environment_id.clone();
                let envs: Vec<(String, String, usize)> = state
                    .environments
                    .iter()
                    .map(|(_, e)| (e.id.clone(), e.name.clone(), e.variables.len()))
                    .collect();

                div()
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(sidebar_border)
                    .px_2()
                    .py_2()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_1()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(text_muted)
                                    .child(format!("ENVIRONMENTS ({})", envs.len())),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .when(has_clip, |el| {
                                        el.child(
                                            Button::new("sidebar-env-paste-btn")
                                                .label("Paste")
                                                .xsmall()
                                                .ghost()
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    on_env_action(
                                                        this,
                                                        SidebarEnvAction::Paste,
                                                        window,
                                                        cx,
                                                    );
                                                })),
                                        )
                                    })
                                    .child(
                                        Button::new("sidebar-env-new-btn")
                                            .icon(icon(IconName::Plus))
                                            .xsmall()
                                            .ghost()
                                            .tooltip("New Environment")
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                on_env_action(
                                                    this,
                                                    SidebarEnvAction::New,
                                                    window,
                                                    cx,
                                                );
                                            })),
                                    ),
                            ),
                    )
                    .when(envs.is_empty(), |el| {
                        el.child(
                            div()
                                .px_1()
                                .text_xs()
                                .text_color(text_muted)
                                .child("No environments yet"),
                        )
                    })
                    .child(
                        div()
                            .max_h(px(140.0))
                            .id("sidebar-envs-scroll")
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .children(envs.into_iter().enumerate().map(|(idx, (eid, ename, var_count))| {
                                let is_active = active_env_id.as_deref() == Some(eid.as_str());
                                let eid_edit = eid.clone();
                                let eid_act = eid.clone();
                                let eid_dup = eid.clone();
                                let eid_copy = eid.clone();
                                let eid_del = eid.clone();

                                div()
                                    .id(ElementId::NamedInteger("sidebar-env-row".into(), idx as u64))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded(cx.theme().radius)
                                    .bg(if is_active {
                                        active_bg
                                    } else {
                                        sidebar_bg
                                    })
                                    .hover(move |s| s.bg(hover_bg))
                                    .child(
                                        div()
                                            .id(ElementId::NamedInteger(
                                                "sidebar-env-edit".into(),
                                                idx as u64,
                                            ))
                                            .flex()
                                            .items_center()
                                            .gap_1p5()
                                            .flex_1()
                                            .min_w_0()
                                            .cursor_pointer()
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                on_env_action(
                                                    this,
                                                    SidebarEnvAction::Edit(eid_edit.clone()),
                                                    window,
                                                    cx,
                                                );
                                            }))
                                            .child(
                                                div()
                                                    .w(px(6.0))
                                                    .h(px(6.0))
                                                    .rounded_full()
                                                    .bg(if is_active {
                                                        green_color
                                                    } else {
                                                        text_muted.opacity(0.45)
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(if is_active {
                                                        primary_color
                                                    } else {
                                                        sidebar_fg
                                                    })
                                                    .truncate()
                                                    .child(format!("{ename} ({var_count})")),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_0p5()
                                            .child(
                                                Button::new(ElementId::NamedInteger(
                                                    "sidebar-env-act".into(),
                                                    idx as u64,
                                                ))
                                                .label(if is_active { "●" } else { "○" })
                                                .xsmall()
                                                .ghost()
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    on_env_action(
                                                        this,
                                                        SidebarEnvAction::Activate(eid_act.clone()),
                                                        window,
                                                        cx,
                                                    );
                                                })),
                                            )
                                            .child(
                                                Button::new(ElementId::NamedInteger(
                                                    "sidebar-env-dup".into(),
                                                    idx as u64,
                                                ))
                                                .label("⧉")
                                                .xsmall()
                                                .ghost()
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    on_env_action(
                                                        this,
                                                        SidebarEnvAction::Duplicate(eid_dup.clone()),
                                                        window,
                                                        cx,
                                                    );
                                                })),
                                            )
                                            .child(
                                                Button::new(ElementId::NamedInteger(
                                                    "sidebar-env-copy".into(),
                                                    idx as u64,
                                                ))
                                                .label("⎘")
                                                .xsmall()
                                                .ghost()
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    on_env_action(
                                                        this,
                                                        SidebarEnvAction::Copy(eid_copy.clone()),
                                                        window,
                                                        cx,
                                                    );
                                                })),
                                            )
                                            .child(
                                                Button::new(ElementId::NamedInteger(
                                                    "sidebar-env-del".into(),
                                                    idx as u64,
                                                ))
                                                .label("×")
                                                .xsmall()
                                                .ghost()
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    on_env_action(
                                                        this,
                                                        SidebarEnvAction::Delete(eid_del.clone()),
                                                        window,
                                                        cx,
                                                    );
                                                })),
                                            ),
                                    )
                            })),
                    )
            })
            .child(
                div()
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(sidebar_border)
                    .child(
                        SidebarFooter::new().child(
                            h_flex()
                                .gap_2()
                                .px_3()
                                .py_2()
                                .text_xs()
                                .text_color(text_muted)
                                .child(icon(IconName::Check).xsmall().text_color(green_color))
                                .child("Workspace Synced"),
                        ),
                    ),
            )
    }
}

fn clamp_menu_pos(pos: Point<Pixels>, window: &Window) -> (f32, f32) {
    let vp = window.viewport_size();
    let max_x: f32 = (vp.width - px(200.0)).into();
    let max_y: f32 = (vp.height - px(260.0)).into();
    let px_val: f32 = pos.x.into();
    let py_val: f32 = pos.y.into();
    (px_val.clamp(8.0, max_x.max(8.0)), py_val.clamp(8.0, max_y.max(8.0)))
}

fn method_tag(method: HttpMethod) -> Tag {
    let variant = match method {
        HttpMethod::GET => TagVariant::Primary,
        HttpMethod::POST => TagVariant::Success,
        HttpMethod::PUT => TagVariant::Warning,
        HttpMethod::PATCH => TagVariant::Info,
        HttpMethod::DELETE => TagVariant::Danger,
        _ => TagVariant::Secondary,
    };
    Tag::new()
        .with_variant(variant)
        .outline()
        .xsmall()
        .child(method.as_str())
}

fn node_to_tree_item(
    node: &WorkspaceTreeNode,
    meta: &mut HashMap<String, SidebarNodeMeta>,
) -> TreeItem {
    match node {
        WorkspaceTreeNode::Folder {
            name,
            path,
            is_expanded,
            children,
        } => {
            let key = format!("folder:{}", path.display());
            meta.insert(
                key.clone(),
                SidebarNodeMeta::Folder {
                    name: name.clone(),
                    path: path.clone(),
                    is_expanded: *is_expanded,
                },
            );
            let child_items = children
                .iter()
                .map(|c| node_to_tree_item(c, meta))
                .collect::<Vec<_>>();
            TreeItem::new(key, name.clone())
                .expanded(*is_expanded)
                .children(child_items)
        }
        WorkspaceTreeNode::Request { name, path, method } => {
            let path_str = path.to_string_lossy().to_string();
            let key = format!("file:{}", path.display());
            meta.insert(
                key.clone(),
                SidebarNodeMeta::Request {
                    name: name.clone(),
                    method: *method,
                    col_id: String::new(),
                    req_id_or_path: path_str,
                    disk_path: Some(path.clone()),
                },
            );
            TreeItem::new(key, name.clone())
        }
    }
}
