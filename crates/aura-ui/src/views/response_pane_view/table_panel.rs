//! Auto-flattening Data Table panel for `ResponsePaneView`.

use super::{ResponsePaneAction, ResponsePaneView};
use crate::services::ResponseDataService;
use crate::theme::icon;
use aura_core::model::AuraResponse;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};

impl ResponsePaneView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_table_panel<V: 'static>(
        resp: &AuraResponse,
        search_query: &str,
        sort_col: Option<usize>,
        sort_asc: bool,
        search_input: Option<&Entity<InputState>>,
        border: Hsla,
        muted: Hsla,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, ResponsePaneAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        match ResponseDataService::extract_table_dataset(&resp.body) {
            Err(err) => div().child(
                Empty::new().header(
                    EmptyHeader::new().description(
                        EmptyDescription::new()
                            .child(format!("Table viewer requires JSON data ({err}).")),
                    ),
                ),
            ),
            Ok(dataset) if dataset.columns.is_empty() => div().child(
                Empty::new().header(
                    EmptyHeader::new().description(
                        EmptyDescription::new().child("JSON array is empty - no rows to display."),
                    ),
                ),
            ),
            Ok(dataset) => {
                let rows = ResponseDataService::filter_and_sort_table(
                    &dataset,
                    search_query,
                    sort_col,
                    sort_asc,
                );
                let total_rows = dataset.rows.len();
                let shown_rows = rows.len();

                let table_bg = cx.theme().background;
                let head_bg = cx.theme().list_head;
                let even_bg = cx.theme().list_even;
                let hover_bg = cx.theme().list_hover;
                let primary_color = cx.theme().primary;
                let fg_color = cx.theme().foreground;
                let radius = cx.theme().radius;

                v_flex()
                    .gap_2()
                    .min_w_0()
                    .child(
                        h_flex()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child(format!(
                                        "DATA TABLE ({} · {}/{} rows)",
                                        dataset.source_path, shown_rows, total_rows
                                    )),
                            )
                            .when_some(search_input, |el, input| {
                                el.child(
                                    div()
                                        .w(px(220.0))
                                        .child(
                                            Input::new(input)
                                                .prefix(icon(IconName::Search).xsmall())
                                                .xsmall(),
                                        ),
                                )
                            }),
                    )
                    .child(
                        v_flex()
                            .rounded(radius)
                            .border_1()
                            .border_color(border)
                            .bg(table_bg)
                            .overflow_hidden()
                            .child(
                                h_flex()
                                    .bg(head_bg)
                                    .border_b_1()
                                    .border_color(border)
                                    .children(dataset.columns.iter().enumerate().map(
                                        |(col_idx, col_name)| {
                                            let is_sorted = sort_col == Some(col_idx);
                                            h_flex()
                                                .id(ElementId::NamedInteger(
                                                    "resp-tbl-hdr".into(),
                                                    col_idx as u64,
                                                ))
                                                .flex_1()
                                                .min_w(px(90.0))
                                                .gap_1()
                                                .px_2()
                                                .py_1p5()
                                                .cursor_pointer()
                                                .text_xs()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(primary_color)
                                                .hover(move |s| s.bg(hover_bg))
                                                .child(col_name.clone())
                                                .when(is_sorted, |el| {
                                                    el.child(
                                                        icon(if sort_asc {
                                                            IconName::ChevronUp
                                                        } else {
                                                            IconName::ChevronDown
                                                        })
                                                        .xsmall(),
                                                    )
                                                })
                                                .on_click(cx.listener(
                                                    move |this, _, window, cx| {
                                                        on_action(
                                                            this,
                                                            ResponsePaneAction::ToggleTableSort(
                                                                col_idx,
                                                            ),
                                                            window,
                                                            cx,
                                                        );
                                                    },
                                                ))
                                        },
                                    )),
                            )
                            .children(rows.into_iter().take(250).enumerate().map(
                                |(row_idx, row)| {
                                    let bg = if row_idx % 2 == 0 { table_bg } else { even_bg };
                                    h_flex()
                                        .bg(bg)
                                        .border_b_1()
                                        .border_color(border.opacity(0.5))
                                        .children(row.into_iter().map(|cell| {
                                            div()
                                                .flex_1()
                                                .min_w(px(90.0))
                                                .px_2()
                                                .py_1()
                                                .font_family("monospace")
                                                .text_xs()
                                                .text_color(fg_color)
                                                .truncate()
                                                .child(cell)
                                        }))
                                },
                            )),
                    )
            }
        }
    }
}
