//! Auto-flattening Data Table panel for `ResponsePaneView`.

use super::{ResponsePaneAction, ResponsePaneView};
use crate::services::ResponseDataService;
use aura_core::model::AuraResponse;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::Sizable;

impl ResponsePaneView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_table_panel<V: 'static>(
        resp: &AuraResponse,
        search_query: &str,
        sort_col: Option<usize>,
        sort_asc: bool,
        search_input: Option<&Entity<InputState>>,
        border: Rgba,
        muted: Rgba,
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
                        EmptyDescription::new().child("JSON array is empty — no rows to display."),
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

                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .min_w_0()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(muted)
                                    .child(format!(
                                        "DATA TABLE ({} • {}/{} rows)",
                                        dataset.source_path, shown_rows, total_rows
                                    )),
                            )
                            .when_some(search_input, |el, input| {
                                el.child(
                                    div()
                                        .w(px(220.0))
                                        .child(Input::new(input).xsmall()),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .rounded_md()
                            .border_1()
                            .border_color(border)
                            .bg(rgb(0x222224))
                            .overflow_hidden()
                            // Column Header Row
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .bg(rgb(0x2a2a2c))
                                    .border_b_1()
                                    .border_color(border)
                                    .children(dataset.columns.iter().enumerate().map(
                                        |(col_idx, col_name)| {
                                            let indicator = if sort_col == Some(col_idx) {
                                                if sort_asc { " ▲" } else { " ▼" }
                                            } else {
                                                ""
                                            };
                                            div()
                                                .id(ElementId::NamedInteger(
                                                    "resp-tbl-hdr".into(),
                                                    col_idx as u64,
                                                ))
                                                .flex_1()
                                                .min_w(px(90.0))
                                                .px_2()
                                                .py_1p5()
                                                .cursor_pointer()
                                                .text_xs()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(rgb(0x2997ff))
                                                .hover(|s| s.bg(rgb(0x333336)))
                                                .child(format!("{col_name}{indicator}"))
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
                            // Data Rows (capped at 250 for smooth 60fps rendering)
                            .children(rows.into_iter().take(250).enumerate().map(
                                |(row_idx, row)| {
                                    let bg = if row_idx % 2 == 0 {
                                        rgb(0x222224)
                                    } else {
                                        rgb(0x262629)
                                    };
                                    div()
                                        .flex()
                                        .items_center()
                                        .bg(bg)
                                        .border_b_1()
                                        .border_color(rgb(0x2d2d30))
                                        .children(row.into_iter().map(|cell| {
                                            div()
                                                .flex_1()
                                                .min_w(px(90.0))
                                                .px_2()
                                                .py_1()
                                                .font_family("monospace")
                                                .text_xs()
                                                .text_color(rgb(0xd4d4d4))
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
