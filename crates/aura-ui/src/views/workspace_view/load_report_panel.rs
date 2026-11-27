//! Live Time-Series Charts, Per-Endpoint Aggregate Report Table, and CSV/JSON/HTML Export UI.

use super::load_actions::LoadExportFormat;
use super::AuraWorkspaceView;
use crate::app_state::{LoadChartMetric, LoadResultsViewTab};
use crate::theme::subtle_surface;
use aura_engine::{compute_aggregate_report, compute_time_series};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::base::Disableable as _;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Sizable, h_flex, v_flex};

impl AuraWorkspaceView {
    pub(super) fn render_load_results_section(
        &self,
        border: Hsla,
        muted: Hsla,
        fg: Hsla,
        radius: Pixels,
        cx: &mut Context<Self>,
    ) -> Div {
        let surface = subtle_surface(cx);
        let records: &[aura_engine::RunnerExecutionRecord] = match &self.state.load_report {
            Some(r) => &r.execution_log,
            None => &self.state.load_live_records,
        };
        let total_dur_ms = self
            .state
            .load_report
            .as_ref()
            .map(|r| r.total_duration_ms)
            .unwrap_or_else(|| records.iter().map(|r| r.elapsed_ms).max().unwrap_or(1));
        let has_data = !records.is_empty();
        let tab = self.state.load_results_tab;

        v_flex()
            .gap_2()
            .p_2()
            .rounded(radius)
            .bg(surface)
            .border_1()
            .border_color(border)
            .when_some(self.state.load_report.as_ref(), |el, r| {
                el.child(
                    h_flex()
                        .gap_1p5()
                        .flex_wrap()
                        .child(Tag::info().small().child(format!("Total: {}", r.total_requests)))
                        .child(Tag::success().small().child(format!("RPS: {:.1}/s", r.requests_per_second)))
                        .child(Tag::secondary().small().child(format!("Peak VUs: {}", r.peak_vus)))
                        .child(if r.failed_requests == 0 {
                            Tag::success().small().child("Errors: 0.0%")
                        } else {
                            Tag::danger().small().child(format!(
                                "Errors: {} ({:.1}%)",
                                r.failed_requests, r.error_rate_percent
                            ))
                        })
                        .child(Tag::secondary().small().child(format!("p50: {}ms", r.p50_latency_ms)))
                        .child(Tag::secondary().small().child(format!("p95: {}ms", r.p95_latency_ms)))
                        .child(Tag::secondary().small().child(format!("p99: {}ms", r.p99_latency_ms)))
                        .child(if r.sla_passed {
                            Tag::success().small().child("SLA PASSED")
                        } else {
                            Tag::danger().small().child("SLA FAILED")
                        }),
                )
            })
            // Listener Tabs & Export Actions
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        h_flex()
                            .gap_1()
                            .children(LoadResultsViewTab::ALL.into_iter().map(|t| {
                                let active = tab == t;
                                let b = Button::new(ElementId::Name(format!("load-rtab-{}", t.label()).into()))
                                    .label(t.label())
                                    .xsmall()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.state.load_results_tab = t;
                                        cx.notify();
                                    }));
                                if active { b.primary() } else { b.outline() }
                            })),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("load-exp-csv-btn")
                                    .label("CSV (.jtl)")
                                    .xsmall()
                                    .outline()
                                    .disabled(!has_data)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.spawn_export_load_report(LoadExportFormat::CsvJtl, cx);
                                    })),
                            )
                            .child(
                                Button::new("load-exp-json-btn")
                                    .label("JSON")
                                    .xsmall()
                                    .outline()
                                    .disabled(!has_data)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.spawn_export_load_report(LoadExportFormat::Json, cx);
                                    })),
                            )
                            .child(
                                Button::new("load-exp-html-btn")
                                    .label("HTML Report")
                                    .xsmall()
                                    .secondary()
                                    .disabled(!has_data)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.spawn_export_load_report(LoadExportFormat::Html, cx);
                                    })),
                            ),
                    ),
            )
            .child(match tab {
                LoadResultsViewTab::TimeSeriesCharts => {
                    self.render_time_series_charts(records, border, muted, fg, radius, cx)
                }
                LoadResultsViewTab::AggregateTable => {
                    self.render_aggregate_table(records, total_dur_ms, border, muted, fg, radius, cx)
                }
                LoadResultsViewTab::RawStream => {
                    self.render_raw_stream(records, border, muted, fg, radius, cx)
                }
            })
    }

    fn render_time_series_charts(
        &self,
        records: &[aura_engine::RunnerExecutionRecord],
        border: Hsla,
        muted: Hsla,
        fg: Hsla,
        radius: Pixels,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let metric = self.state.load_chart_metric;
        let buckets = compute_time_series(records, None);
        let chart_h = 92.0_f32;

        let max_val: f64 = buckets
            .iter()
            .map(|b| match metric {
                LoadChartMetric::Rps => b.rps,
                LoadChartMetric::ActiveVus => b.active_vus as f64,
                LoadChartMetric::LatencyPercentiles => b.p99_ms as f64,
                LoadChartMetric::ErrorRate => b.error_rate_percent,
            })
            .fold(0.0_f64, f64::max)
            .max(1.0);

        let bar_color = match metric {
            LoadChartMetric::Rps => cx.theme().blue,
            LoadChartMetric::ActiveVus => cx.theme().cyan,
            LoadChartMetric::LatencyPercentiles => cx.theme().yellow,
            LoadChartMetric::ErrorRate => cx.theme().red,
        };
        let green = cx.theme().green;
        let yellow = cx.theme().yellow;
        let red = cx.theme().red;

        v_flex()
            .gap_1p5()
            .p_2()
            .rounded(radius)
            .bg(cx.theme().background)
            .border_1()
            .border_color(border)
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        h_flex()
                            .gap_1()
                            .children(LoadChartMetric::ALL.into_iter().map(|m| {
                                let active = metric == m;
                                let b = Button::new(ElementId::Name(format!("load-metric-{}", m.label()).into()))
                                    .label(m.label())
                                    .xsmall()
                                    .rounded(ButtonRounded::Small)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.state.load_chart_metric = m;
                                        cx.notify();
                                    }));
                                if active { b.primary() } else { b.ghost() }
                            })),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted)
                            .child(format!("Peak: {:.1}", max_val)),
                    ),
            )
            .when(buckets.is_empty(), |el| {
                el.child(
                    div()
                        .h(px(chart_h))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .text_color(muted)
                        .child("Start a load test to see live RPS, Active VUs, p50/p95/p99, and Error Rate charts."),
                )
            })
            .when(!buckets.is_empty(), |el| {
                el.child(
                    div()
                        .flex()
                        .items_end()
                        .gap_1()
                        .h(px(chart_h + 16.0))
                        .pt_2()
                        .children(buckets.iter().take(20).map(|b| {
                            let val_label = match metric {
                                LoadChartMetric::Rps => format!("{:.0}", b.rps),
                                LoadChartMetric::ActiveVus => format!("{}", b.active_vus),
                                LoadChartMetric::LatencyPercentiles => format!("{}", b.p95_ms),
                                LoadChartMetric::ErrorRate => format!("{:.0}%", b.error_rate_percent),
                            };
                            let bars = if metric == LoadChartMetric::LatencyPercentiles {
                                let h50 = ((b.p50_ms as f64 / max_val) * chart_h as f64).max(2.0) as f32;
                                let h95 = ((b.p95_ms as f64 / max_val) * chart_h as f64).max(2.0) as f32;
                                let h99 = ((b.p99_ms as f64 / max_val) * chart_h as f64).max(2.0) as f32;
                                h_flex()
                                    .gap_0p5()
                                    .items_end()
                                    .justify_center()
                                    .w_full()
                                    .child(div().w(px(5.0)).h(px(h50)).rounded_t_sm().bg(green))
                                    .child(div().w(px(5.0)).h(px(h95)).rounded_t_sm().bg(yellow))
                                    .child(div().w(px(5.0)).h(px(h99)).rounded_t_sm().bg(red))
                            } else {
                                let raw = match metric {
                                    LoadChartMetric::Rps => b.rps,
                                    LoadChartMetric::ActiveVus => b.active_vus as f64,
                                    LoadChartMetric::ErrorRate => b.error_rate_percent,
                                    LoadChartMetric::LatencyPercentiles => 0.0,
                                };
                                let bh = ((raw / max_val) * chart_h as f64).max(2.0) as f32;
                                h_flex()
                                    .justify_center()
                                    .w_full()
                                    .child(div().w(px(14.0)).h(px(bh)).rounded_t_sm().bg(bar_color))
                            };

                            v_flex()
                                .flex_1()
                                .min_w(px(20.0))
                                .items_center()
                                .justify_end()
                                .gap_0p5()
                                .child(div().text_xs().text_color(fg).child(val_label))
                                .child(bars)
                        })),
                )
                .child(
                    h_flex()
                        .gap_1()
                        .children(buckets.iter().take(20).map(|b| {
                            div()
                                .flex_1()
                                .min_w(px(20.0))
                                .text_xs()
                                .text_color(muted)
                                .truncate()
                                .child(b.bucket_label.clone())
                        })),
                )
            })
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_aggregate_table(
        &self,
        records: &[aura_engine::RunnerExecutionRecord],
        total_dur_ms: u64,
        border: Hsla,
        muted: Hsla,
        fg: Hsla,
        radius: Pixels,
        cx: &Context<Self>,
    ) -> AnyElement {
        let rows = compute_aggregate_report(records, total_dur_ms);

        v_flex()
            .id("load-agg-scroll")
            .h(px(145.0))
            .overflow_y_scroll()
            .p_1p5()
            .gap_1()
            .rounded(radius)
            .bg(cx.theme().background)
            .border_1()
            .border_color(border)
            .child(
                h_flex()
                    .gap_1()
                    .px_1()
                    .pb_1()
                    .border_b_1()
                    .border_color(border)
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(muted)
                    .child(div().flex_1().child("Endpoint"))
                    .child(div().w(px(46.0)).child("N"))
                    .child(div().w(px(52.0)).child("Avg"))
                    .child(div().w(px(46.0)).child("Med"))
                    .child(div().w(px(46.0)).child("p90"))
                    .child(div().w(px(46.0)).child("p95"))
                    .child(div().w(px(46.0)).child("p99"))
                    .child(div().w(px(44.0)).child("Min"))
                    .child(div().w(px(46.0)).child("Max"))
                    .child(div().w(px(48.0)).child("Err%"))
                    .child(div().w(px(52.0)).child("RPS"))
                    .child(div().w(px(56.0)).child("KB/s")),
            )
            .when(rows.is_empty(), |el| {
                el.child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .p_2()
                        .child("No endpoint metrics recorded yet."),
                )
            })
            .children(rows.into_iter().map(|r| {
                h_flex()
                    .gap_1()
                    .px_1()
                    .py_0p5()
                    .text_xs()
                    .text_color(fg)
                    .child(
                        div()
                            .flex_1()
                            .truncate()
                            .child(format!("{} {}", r.method, r.request_name)),
                    )
                    .child(div().w(px(46.0)).child(r.samples.to_string()))
                    .child(div().w(px(52.0)).child(format!("{:.1}", r.avg_ms)))
                    .child(div().w(px(46.0)).child(r.median_ms.to_string()))
                    .child(div().w(px(46.0)).child(r.p90_ms.to_string()))
                    .child(div().w(px(46.0)).child(r.p95_ms.to_string()))
                    .child(div().w(px(46.0)).child(r.p99_ms.to_string()))
                    .child(div().w(px(44.0)).child(r.min_ms.to_string()))
                    .child(div().w(px(46.0)).child(r.max_ms.to_string()))
                    .child(div().w(px(48.0)).child(format!("{:.1}%", r.error_percent)))
                    .child(div().w(px(52.0)).child(format!("{:.1}", r.rps)))
                    .child(div().w(px(56.0)).child(format!("{:.1}", r.kb_per_sec)))
            }))
            .into_any_element()
    }

    fn render_raw_stream(
        &self,
        records: &[aura_engine::RunnerExecutionRecord],
        border: Hsla,
        muted: Hsla,
        fg: Hsla,
        radius: Pixels,
        cx: &Context<Self>,
    ) -> AnyElement {
        let recent: Vec<_> = records.iter().rev().take(35).cloned().collect();

        v_flex()
            .id("load-live-feed")
            .h(px(145.0))
            .overflow_y_scroll()
            .gap_1()
            .p_1p5()
            .rounded(radius)
            .bg(cx.theme().background)
            .border_1()
            .border_color(border)
            .when(recent.is_empty(), |box_el| {
                box_el.child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .child("No raw samples captured yet."),
                )
            })
            .children(recent.into_iter().map(|rec| {
                let status_tag = match rec.status {
                    200..=299 => Tag::success(),
                    300..=399 => Tag::info(),
                    400..=499 => Tag::warning(),
                    _ => Tag::danger(),
                };
                h_flex()
                    .justify_between()
                    .gap_2()
                    .text_xs()
                    .px_1p5()
                    .py_0p5()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .flex_1()
                            .overflow_hidden()
                            .child(status_tag.xsmall().child(rec.status.to_string()))
                            .child(Tag::secondary().xsmall().child(rec.method))
                            .child(div().text_color(fg).truncate().child(rec.request_name)),
                    )
                    .child(div().text_color(muted).child(format!(
                        "{} ms • {} B • {} VUs • @{:.2}s",
                        rec.latency_ms,
                        rec.size_bytes,
                        rec.active_vus,
                        rec.elapsed_ms as f64 / 1000.0
                    )))
            }))
            .into_any_element()
    }
}
