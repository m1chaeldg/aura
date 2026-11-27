//! Load-test time-series bucketing, Per-Endpoint Aggregate Reports,
//! JMeter `.jtl`-compatible CSV/JSON raw export, and standalone HTML report generation.

use crate::runner::{RunnerExecutionRecord, RunnerReport};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// Single time-window bucket for live / post-run UI charts & HTML reporting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeSeriesBucket {
    pub bucket_label: String,
    pub bucket_start_ms: u64,
    pub bucket_end_ms: u64,
    pub rps: f64,
    pub active_vus: usize,
    pub p50_ms: u64,
    pub p95_ms: u64,
    pub p99_ms: u64,
    pub error_rate_percent: f64,
    pub sample_count: usize,
}

/// JMeter Aggregate Report row grouped by endpoint (`request_name` + `method`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EndpointAggregateRow {
    pub request_name: String,
    pub method: String,
    pub samples: usize,
    pub avg_ms: f64,
    pub median_ms: u64,
    pub p90_ms: u64,
    pub p95_ms: u64,
    pub p99_ms: u64,
    pub min_ms: u64,
    pub max_ms: u64,
    pub error_percent: f64,
    pub rps: f64,
    pub kb_per_sec: f64,
}

/// Computes time-series buckets (`RPS`, `Active VUs`, `p50`/`p95`/`p99`, `Error %`).
/// When `bucket_ms_opt` is `None`, automatically selects a bucket size yielding ~8..=20 buckets.
pub fn compute_time_series(
    records: &[RunnerExecutionRecord],
    bucket_ms_opt: Option<u64>,
) -> Vec<TimeSeriesBucket> {
    if records.is_empty() {
        return Vec::new();
    }

    let max_elapsed = records.iter().map(|r| r.elapsed_ms).max().unwrap_or(0);
    let bucket_ms = bucket_ms_opt.unwrap_or_else(|| {
        if max_elapsed <= 1_000 {
            (max_elapsed / 8).max(50)
        } else if max_elapsed <= 20_000 {
            1_000
        } else {
            (max_elapsed / 16).max(1_000)
        }
    });
    let bucket_ms = bucket_ms.max(1);

    let mut groups: BTreeMap<usize, Vec<&RunnerExecutionRecord>> = BTreeMap::new();
    for rec in records {
        let idx = (rec.elapsed_ms / bucket_ms) as usize;
        groups.entry(idx).or_default().push(rec);
    }

    let bucket_secs = (bucket_ms as f64 / 1000.0).max(0.001);
    let mut out = Vec::with_capacity(groups.len());

    for (idx, items) in groups {
        let start_ms = idx as u64 * bucket_ms;
        let end_ms = start_ms + bucket_ms;
        let count = items.len();
        let failed = items.iter().filter(|r| !r.passed).count();
        let max_vus = items.iter().map(|r| r.active_vus).max().unwrap_or(1);

        let mut lats: Vec<u64> = items.iter().map(|r| r.latency_ms).collect();
        lats.sort_unstable();

        out.push(TimeSeriesBucket {
            bucket_label: format!("{:.1}s", end_ms as f64 / 1000.0),
            bucket_start_ms: start_ms,
            bucket_end_ms: end_ms,
            rps: count as f64 / bucket_secs,
            active_vus: max_vus,
            p50_ms: percentile_sorted(&lats, 50.0),
            p95_ms: percentile_sorted(&lats, 95.0),
            p99_ms: percentile_sorted(&lats, 99.0),
            error_rate_percent: (failed as f64 / count as f64) * 100.0,
            sample_count: count,
        });
    }

    out
}

/// Computes Per-Endpoint Aggregate Report metrics (`Samples`, `Avg`, `Median`, `p90`, `p95`, `p99`, `Min`, `Max`, `Error %`, `RPS`, `KB/s`).
pub fn compute_aggregate_report(
    records: &[RunnerExecutionRecord],
    total_duration_ms: u64,
) -> Vec<EndpointAggregateRow> {
    if records.is_empty() {
        return Vec::new();
    }

    // Preserve first-seen endpoint order cleanly
    let mut order: Vec<(String, String)> = Vec::new();
    let mut grouped: BTreeMap<(String, String), Vec<&RunnerExecutionRecord>> = BTreeMap::new();

    for rec in records {
        let key = (rec.request_name.clone(), rec.method.clone());
        if !grouped.contains_key(&key) {
            order.push(key.clone());
        }
        grouped.entry(key).or_default().push(rec);
    }

    let total_secs = (total_duration_ms as f64 / 1000.0).max(0.001);
    let mut rows = Vec::with_capacity(order.len());

    for key in order {
        let Some(items) = grouped.get(&key) else {
            continue;
        };
        let samples = items.len();
        let mut lats: Vec<u64> = items.iter().map(|r| r.latency_ms).collect();
        lats.sort_unstable();

        let sum_lat: u64 = lats.iter().sum();
        let failed = items.iter().filter(|r| !r.passed).count();
        let total_bytes: u64 = items.iter().map(|r| r.size_bytes as u64).sum();

        rows.push(EndpointAggregateRow {
            request_name: key.0,
            method: key.1,
            samples,
            avg_ms: sum_lat as f64 / samples as f64,
            median_ms: percentile_sorted(&lats, 50.0),
            p90_ms: percentile_sorted(&lats, 90.0),
            p95_ms: percentile_sorted(&lats, 95.0),
            p99_ms: percentile_sorted(&lats, 99.0),
            min_ms: *lats.first().unwrap_or(&0),
            max_ms: *lats.last().unwrap_or(&0),
            error_percent: (failed as f64 / samples as f64) * 100.0,
            rps: samples as f64 / total_secs,
            kb_per_sec: (total_bytes as f64 / 1024.0) / total_secs,
        });
    }

    rows
}

fn percentile_sorted(sorted: &[u64], pct: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = (pct / 100.0 * (sorted.len() - 1) as f64).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

/// Exports raw per-request telemetry to JMeter `.jtl`-compatible RFC-4180 CSV.
pub fn export_samples_jtl_csv(records: &[RunnerExecutionRecord]) -> String {
    let mut out = String::from(
        "elapsed_ms,iteration,request_name,method,status,latency_ms,size_bytes,active_vus,passed,error\n",
    );
    for r in records {
        let err = r.error.as_deref().unwrap_or("");
        let _ = writeln!(
            out,
            "{},{},{},{},{},{},{},{},{},{}",
            r.elapsed_ms,
            r.iteration,
            csv_escape(&r.request_name),
            csv_escape(&r.method),
            r.status,
            r.latency_ms,
            r.size_bytes,
            r.active_vus,
            r.passed,
            csv_escape(err),
        );
    }
    out
}

#[derive(Serialize)]
struct JsonReportBundle<'a> {
    report: &'a RunnerReport,
    aggregate_endpoints: Vec<EndpointAggregateRow>,
    time_series: Vec<TimeSeriesBucket>,
}

/// Exports full `RunnerReport` + aggregate table + time series + raw samples to pretty JSON.
pub fn export_samples_json(report: &RunnerReport) -> anyhow::Result<String> {
    let bundle = JsonReportBundle {
        report,
        aggregate_endpoints: compute_aggregate_report(
            &report.execution_log,
            report.total_duration_ms,
        ),
        time_series: compute_time_series(&report.execution_log, None),
    };
    Ok(serde_json::to_string_pretty(&bundle)?)
}

/// Generates a standalone, dependency-free HTML load test report with inline charts and aggregate table.
pub fn generate_html_report(title: &str, report: &RunnerReport) -> String {
    let aggs = compute_aggregate_report(&report.execution_log, report.total_duration_ms);
    let series = compute_time_series(&report.execution_log, None);
    let sla_badge = if report.sla_passed {
        r#"<span class="badge pass">SLA PASSED</span>"#
    } else {
        r#"<span class="badge fail">SLA FAILED</span>"#
    };

    let mut rows_html = String::new();
    for a in &aggs {
        let _ = write!(
            rows_html,
            "<tr><td>{}</td><td><span class=\"tag\">{}</span></td><td>{}</td><td>{:.1} ms</td><td>{} ms</td><td>{} ms</td><td>{} ms</td><td>{} ms</td><td>{} ms</td><td>{} ms</td><td>{:.1}%</td><td>{:.2}</td><td>{:.2}</td></tr>",
            html_escape(&a.request_name),
            html_escape(&a.method),
            a.samples,
            a.avg_ms,
            a.median_ms,
            a.p90_ms,
            a.p95_ms,
            a.p99_ms,
            a.min_ms,
            a.max_ms,
            a.error_percent,
            a.rps,
            a.kb_per_sec,
        );
    }

    let rps_svg = build_svg_bar_chart(
        "Throughput (RPS over Time)",
        "#2997ff",
        &series
            .iter()
            .map(|b| (b.bucket_label.as_str(), b.rps))
            .collect::<Vec<_>>(),
    );
    let p95_svg = build_svg_bar_chart(
        "p95 Latency (ms over Time)",
        "#f5a623",
        &series
            .iter()
            .map(|b| (b.bucket_label.as_str(), b.p95_ms as f64))
            .collect::<Vec<_>>(),
    );

    let mut violations_html = String::new();
    if !report.sla_violations.is_empty() {
        violations_html.push_str("<div class=\"card\"><h3>SLA Gate Violations</h3><ul>");
        for v in &report.sla_violations {
            let _ = write!(
                violations_html,
                "<li><strong>{}</strong>: actual {} (threshold {})</li>",
                html_escape(&v.metric),
                html_escape(&v.actual),
                html_escape(&v.threshold)
            );
        }
        violations_html.push_str("</ul></div>");
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>{title} - Aura Load Report</title>
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; background: #1d1d1f; color: #f5f5f7; margin: 0; padding: 24px; }}
  h1, h2, h3 {{ margin: 0 0 12px 0; font-weight: 600; }}
  .header {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; border-bottom: 1px solid #333; padding-bottom: 14px; }}
  .badge {{ padding: 4px 10px; border-radius: 6px; font-size: 12px; font-weight: 600; }}
  .pass {{ background: rgba(40,167,69,0.2); color: #3ddc84; border: 1px solid #28a745; }}
  .fail {{ background: rgba(231,76,60,0.2); color: #ff6b6b; border: 1px solid #e74c3c; }}
  .kpis {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px; margin-bottom: 20px; }}
  .kpi {{ background: #272729; border: 1px solid #333; border-radius: 8px; padding: 12px; }}
  .kpi .label {{ font-size: 12px; color: #a1a1a6; }}
  .kpi .val {{ font-size: 20px; font-weight: 600; margin-top: 4px; }}
  .charts {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(360px, 1fr)); gap: 16px; margin-bottom: 20px; }}
  .card {{ background: #272729; border: 1px solid #333; border-radius: 8px; padding: 16px; margin-bottom: 16px; }}
  table {{ width: 100%; border-collapse: collapse; font-size: 13px; }}
  th, td {{ text-align: left; padding: 8px 10px; border-bottom: 1px solid #333; }}
  th {{ color: #a1a1a6; font-weight: 600; }}
  .tag {{ background: #333336; padding: 2px 6px; border-radius: 4px; font-size: 11px; }}
</style>
</head>
<body>
  <div class="header">
    <div><h1>{title}</h1><div style="color:#a1a1a6;font-size:13px;">Generated by Aura Load Runner</div></div>
    {sla_badge}
  </div>
  <div class="kpis">
    <div class="kpi"><div class="label">Total Samples</div><div class="val">{total}</div></div>
    <div class="kpi"><div class="label">Throughput</div><div class="val">{rps:.2} /s</div></div>
    <div class="kpi"><div class="label">Error Rate</div><div class="val">{err:.2}%</div></div>
    <div class="kpi"><div class="label">Peak VUs</div><div class="val">{vus}</div></div>
    <div class="kpi"><div class="label">p50 / p95 / p99</div><div class="val">{p50} / {p95} / {p99} ms</div></div>
    <div class="kpi"><div class="label">Duration</div><div class="val">{dur} ms</div></div>
  </div>
  {violations_html}
  <div class="charts">{rps_svg}{p95_svg}</div>
  <div class="card">
    <h3>Per-Endpoint Aggregate Report</h3>
    <table>
      <thead>
        <tr><th>Endpoint</th><th>Method</th><th>Samples</th><th>Avg</th><th>Median</th><th>p90</th><th>p95</th><th>p99</th><th>Min</th><th>Max</th><th>Err %</th><th>RPS</th><th>KB/s</th></tr>
      </thead>
      <tbody>{rows_html}</tbody>
    </table>
  </div>
</body>
</html>"#,
        title = html_escape(title),
        total = report.total_requests,
        rps = report.requests_per_second,
        err = report.error_rate_percent,
        vus = report.peak_vus,
        p50 = report.p50_latency_ms,
        p95 = report.p95_latency_ms,
        p99 = report.p99_latency_ms,
        dur = report.total_duration_ms,
    )
}

fn build_svg_bar_chart(title: &str, color: &str, points: &[(&str, f64)]) -> String {
    let w = 480.0_f64;
    let h = 150.0_f64;
    let max_val = points
        .iter()
        .map(|(_, v)| *v)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let count = points.len().max(1);
    let bar_slot = (w - 20.0) / count as f64;
    let bar_w = (bar_slot * 0.7).max(4.0);

    let mut bars = String::new();
    for (idx, (label, val)) in points.iter().enumerate() {
        let norm = (val / max_val).clamp(0.0, 1.0);
        let bh = (norm * (h - 36.0)).max(2.0);
        let x = 10.0 + idx as f64 * bar_slot;
        let y = h - 22.0 - bh;
        let _ = write!(
            bars,
            "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{bar_w:.1}\" height=\"{bh:.1}\" fill=\"{color}\" rx=\"2\"><title>{label}: {val:.2}</title></rect>"
        );
    }

    format!(
        "<div class=\"card\"><h3>{}</h3><svg viewBox=\"0 0 480 150\" width=\"100%\" height=\"150\">{}</svg></div>",
        html_escape(title),
        bars
    )
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
