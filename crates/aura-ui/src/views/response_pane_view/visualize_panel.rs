//! Visualize (BI) panel for `ResponsePaneView`: KPI cards, Group-By builder, and Bar/Line/Donut charts.

use super::{ResponsePaneAction, ResponsePaneView};
use crate::services::{
    AggregationOp, ChartSeriesPoint, ChartType, ResponseBiService, ResponseDataService,
};
use aura_core::model::AuraResponse;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader};
use gpui_kit::component::Sizable;

const SERIES_PALETTE: [u32; 6] = [
    0x0066cc, // Action Blue
    0x28a745, // Green
    0xf5a623, // Amber
    0x8e44ad, // Purple
    0x2997ff, // Sky Blue
    0xe74c3c, // Red
];

impl ResponsePaneView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_visualize_panel<V: 'static>(
        resp: &AuraResponse,
        chart_type: ChartType,
        agg_op: AggregationOp,
        x_col_pref: Option<&str>,
        y_col_pref: Option<&str>,
        border: Rgba,
        muted: Rgba,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, ResponsePaneAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Div {
        let dataset = match ResponseDataService::extract_table_dataset(&resp.body) {
            Ok(ds) if !ds.columns.is_empty() && !ds.rows.is_empty() => ds,
            Ok(_) => {
                return div().child(Empty::new().header(EmptyHeader::new().description(
                    EmptyDescription::new().child("JSON array is empty — no data to visualize."),
                )));
            }
            Err(err) => {
                return div().child(Empty::new().header(EmptyHeader::new().description(
                    EmptyDescription::new()
                        .child(format!("Visualize BI requires JSON array/object data ({err}).")),
                )));
            }
        };

        let summary = ResponseBiService::summarize_dataset(&dataset);
        let x_col = x_col_pref
            .filter(|c| dataset.columns.iter().any(|dc| dc == c))
            .map(str::to_string)
            .or_else(|| summary.categorical_columns.first().cloned())
            .or_else(|| dataset.columns.first().cloned())
            .unwrap_or_default();

        let y_col = y_col_pref
            .filter(|c| summary.numeric_columns.iter().any(|nc| nc == c))
            .map(str::to_string)
            .or_else(|| summary.numeric_columns.first().cloned());

        let effective_op = if y_col.is_none() {
            AggregationOp::Count
        } else {
            agg_op
        };

        let points = ResponseBiService::aggregate_series(
            &dataset,
            &x_col,
            y_col.as_deref(),
            effective_op,
        );

        let y_summary = y_col
            .as_deref()
            .and_then(|yc| summary.columns.iter().find(|c| c.name == yc));

        let chart_btn = |id: &'static str, label: &'static str, target: ChartType| {
            let btn = Button::new(id)
                .label(label)
                .xsmall()
                .rounded(ButtonRounded::Small)
                .on_click(cx.listener(move |this, _, window, cx| {
                    on_action(this, ResponsePaneAction::SetBiChartType(target), window, cx);
                }));
            if chart_type == target {
                btn.primary()
            } else {
                btn.secondary()
            }
        };

        let cols_for_x = dataset.columns.clone();
        let num_cols_for_y = summary.numeric_columns.clone();

        div()
            .flex()
            .flex_col()
            .gap_3()
            .min_w_0()
            // Controls Bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(chart_btn("bi-chart-bar", "Bar", ChartType::Bar))
                            .child(chart_btn("bi-chart-line", "Line", ChartType::Line))
                            .child(chart_btn("bi-chart-donut", "Donut", ChartType::Donut)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .flex_wrap()
                            .child(
                                Button::new("bi-x-col-btn")
                                    .label(format!("Group: {x_col}"))
                                    .xsmall()
                                    .secondary()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            ResponsePaneAction::CycleBiXCol(cols_for_x.clone()),
                                            window,
                                            cx,
                                        );
                                    })),
                            )
                            .child(
                                Button::new("bi-y-col-btn")
                                    .label(format!(
                                        "Metric: {}",
                                        y_col.as_deref().unwrap_or("None")
                                    ))
                                    .xsmall()
                                    .secondary()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(
                                            this,
                                            ResponsePaneAction::CycleBiYCol(num_cols_for_y.clone()),
                                            window,
                                            cx,
                                        );
                                    })),
                            )
                            .child(
                                Button::new("bi-agg-op-btn")
                                    .label(format!("Agg: {}", effective_op.label()))
                                    .xsmall()
                                    .primary()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        on_action(this, ResponsePaneAction::CycleBiAggOp, window, cx);
                                    })),
                            ),
                    ),
            )
            // KPI Summary Cards
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(Self::kpi_card(
                        "ROWS",
                        format!("{}", summary.row_count),
                        &summary.source_path,
                        border,
                        muted,
                    ))
                    .child(Self::kpi_card(
                        "COLUMNS",
                        format!("{}", summary.column_count),
                        &format!(
                            "{} num / {} cat",
                            summary.numeric_columns.len(),
                            summary.categorical_columns.len()
                        ),
                        border,
                        muted,
                    ))
                    .child(Self::kpi_card(
                        "GROUPS",
                        format!("{}", points.len()),
                        &format!("by {x_col}"),
                        border,
                        muted,
                    ))
                    .child(Self::kpi_card(
                        "METRIC SUM / AVG",
                        y_summary
                            .and_then(|s| s.sum)
                            .map(format_num)
                            .unwrap_or_else(|| "--".to_string()),
                        &y_summary
                            .and_then(|s| s.avg)
                            .map(|a| format!("avg {}", format_num(a)))
                            .unwrap_or_else(|| "no numeric col".to_string()),
                        border,
                        muted,
                    )),
            )
            // Chart Surface
            .child(
                div()
                    .p_3()
                    .rounded_md()
                    .bg(rgb(0x222224))
                    .border_1()
                    .border_color(border)
                    .child(match chart_type {
                        ChartType::Bar => Self::render_bar_chart(&points, muted),
                        ChartType::Line => Self::render_line_chart(&points, border, muted),
                        ChartType::Donut => Self::render_donut_chart(&points, muted),
                    }),
            )
    }

    fn kpi_card(title: &str, value: String, sub: &str, border: Rgba, muted: Rgba) -> Div {
        div()
            .flex_1()
            .min_w(px(120.0))
            .p_2p5()
            .rounded_md()
            .bg(rgb(0x272729))
            .border_1()
            .border_color(border)
            .flex()
            .flex_col()
            .gap_0p5()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(muted)
                    .child(title.to_string()),
            )
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xffffff))
                    .child(value),
            )
            .child(div().text_xs().text_color(muted).child(sub.to_string()))
    }

    fn render_bar_chart(points: &[ChartSeriesPoint], muted: Rgba) -> Div {
        let max_val = points
            .iter()
            .map(|p| p.value.abs())
            .fold(0.0_f64, f64::max)
            .max(1e-9);

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(muted)
                    .child("BAR BREAKDOWN"),
            )
            .children(points.iter().take(20).enumerate().map(|(idx, pt)| {
                let ratio = (pt.value.abs() / max_val).clamp(0.02, 1.0) as f32;
                let color = rgb(SERIES_PALETTE[idx % SERIES_PALETTE.len()]);
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_xs()
                    .child(
                        div()
                            .w(px(110.0))
                            .text_color(rgb(0xd4d4d4))
                            .truncate()
                            .child(pt.label.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h(px(14.0))
                            .rounded_sm()
                            .bg(rgb(0x1b1b1d))
                            .overflow_hidden()
                            .child(div().h_full().w(relative(ratio)).bg(color).rounded_sm()),
                    )
                    .child(
                        div()
                            .w(px(95.0))
                            .font_family("monospace")
                            .text_color(rgb(0xffffff))
                            .text_right()
                            .child(format!("{} ({:.1}%)", format_num(pt.value), pt.share_pct)),
                    )
            }))
    }

    fn render_line_chart(points: &[ChartSeriesPoint], border: Rgba, muted: Rgba) -> Div {
        let max_val = points
            .iter()
            .map(|p| p.value)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_val = points.iter().map(|p| p.value).fold(f64::INFINITY, f64::min);
        let span = (max_val - min_val).abs().max(1e-9);
        let chart_h = 150.0_f32;

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_xs()
                    .text_color(muted)
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("SERIES TREND"))
                    .child(div().child(format!(
                        "Min: {}  •  Max: {}",
                        format_num(min_val),
                        format_num(max_val)
                    ))),
            )
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap_1p5()
                    .h(px(chart_h + 24.0))
                    .pt_2()
                    .border_b_1()
                    .border_color(border)
                    .children(points.iter().take(24).map(|pt| {
                        let norm = if (max_val - min_val).abs() < 1e-9 {
                            0.6_f32
                        } else {
                            ((pt.value - min_val) / span).clamp(0.08, 1.0) as f32
                        };
                        let col_h = norm * chart_h;
                        div()
                            .flex_1()
                            .min_w(px(16.0))
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_end()
                            .gap_1()
                            .child(
                                div()
                                    .w(px(8.0))
                                    .h(px(8.0))
                                    .rounded_full()
                                    .bg(rgb(0x2997ff)),
                            )
                            .child(div().w(px(2.0)).h(px(col_h)).bg(rgb(0x0066cc)))
                    })),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .children(points.iter().take(24).map(|pt| {
                        div()
                            .flex_1()
                            .min_w(px(16.0))
                            .text_xs()
                            .text_color(muted)
                            .truncate()
                            .child(pt.label.clone())
                    })),
            )
    }

    fn render_donut_chart(points: &[ChartSeriesPoint], muted: Rgba) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(muted)
                    .child("PROPORTIONAL SHARE"),
            )
            .child(
                div()
                    .flex()
                    .w_full()
                    .h(px(22.0))
                    .rounded_md()
                    .overflow_hidden()
                    .bg(rgb(0x1b1b1d))
                    .children(points.iter().take(12).enumerate().map(|(idx, pt)| {
                        let ratio = (pt.share_pct / 100.0).clamp(0.01, 1.0) as f32;
                        let color = rgb(SERIES_PALETTE[idx % SERIES_PALETTE.len()]);
                        div().h_full().w(relative(ratio)).bg(color)
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(points.iter().take(12).enumerate().map(|(idx, pt)| {
                        let color = rgb(SERIES_PALETTE[idx % SERIES_PALETTE.len()]);
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_xs()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(div().w(px(10.0)).h(px(10.0)).rounded_sm().bg(color))
                                    .child(div().text_color(rgb(0xd4d4d4)).child(pt.label.clone())),
                            )
                            .child(
                                div()
                                    .font_family("monospace")
                                    .text_color(rgb(0xffffff))
                                    .child(format!(
                                        "{} ({:.1}%)",
                                        format_num(pt.value),
                                        pt.share_pct
                                    )),
                            )
                    })),
            )
    }
}

fn format_num(v: f64) -> String {
    if (v.fract()).abs() < 1e-6 {
        format!("{:.0}", v)
    } else {
        format!("{:.2}", v)
    }
}
