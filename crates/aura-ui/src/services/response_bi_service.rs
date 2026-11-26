//! Response BI Service: Dataset profiling, KPI summaries, and Group-By chart aggregation.

use crate::services::TableDataset;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChartType {
    #[default]
    Bar,
    Line,
    Donut,
    Pivot,
}

impl ChartType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Bar => "Bar",
            Self::Line => "Line",
            Self::Donut => "Donut",
            Self::Pivot => "Pivot",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Bar => Self::Line,
            Self::Line => Self::Donut,
            Self::Donut => Self::Pivot,
            Self::Pivot => Self::Bar,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AggregationOp {
    #[default]
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

impl AggregationOp {
    pub fn label(self) -> &'static str {
        match self {
            Self::Count => "Count",
            Self::Sum => "Sum",
            Self::Avg => "Avg",
            Self::Min => "Min",
            Self::Max => "Max",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Count => Self::Sum,
            Self::Sum => Self::Avg,
            Self::Avg => Self::Min,
            Self::Min => Self::Max,
            Self::Max => Self::Count,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColumnSummary {
    pub name: String,
    pub is_numeric: bool,
    pub non_null_count: usize,
    pub distinct_count: usize,
    pub sum: Option<f64>,
    pub avg: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChartSeriesPoint {
    pub label: String,
    pub value: f64,
    pub share_pct: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BiSummary {
    pub row_count: usize,
    pub column_count: usize,
    pub source_path: String,
    pub columns: Vec<ColumnSummary>,
    pub numeric_columns: Vec<String>,
    pub categorical_columns: Vec<String>,
}

/// Cross-tabulated Pivot Table result.
#[derive(Debug, Clone, PartialEq)]
pub struct PivotTable {
    pub row_field: String,
    pub col_field: String,
    pub col_headers: Vec<String>,
    /// Each row: (row_label, cell_values_per_col_header, row_total)
    pub rows: Vec<(String, Vec<Option<f64>>, f64)>,
    pub col_totals: Vec<f64>,
    pub grand_total: f64,
}

pub struct ResponseBiService;

impl ResponseBiService {
    pub fn summarize_dataset(dataset: &TableDataset) -> BiSummary {
        let mut columns = Vec::with_capacity(dataset.columns.len());
        let mut numeric_columns = Vec::new();
        let mut categorical_columns = Vec::new();

        for (col_idx, name) in dataset.columns.iter().enumerate() {
            let mut non_null_count = 0usize;
            let mut distinct = std::collections::HashSet::new();
            let mut nums = Vec::new();

            for row in &dataset.rows {
                let raw = row.get(col_idx).map(String::as_str).unwrap_or("").trim();
                if !raw.is_empty() && raw != "null" {
                    non_null_count += 1;
                    distinct.insert(raw.to_string());
                    if let Ok(n) = raw.parse::<f64>() {
                        if n.is_finite() {
                            nums.push(n);
                        }
                    }
                }
            }

            let is_numeric = non_null_count > 0 && nums.len() == non_null_count;
            let (sum, avg, min, max) = if is_numeric {
                let s: f64 = nums.iter().sum();
                let a = s / nums.len() as f64;
                let mn = nums.iter().copied().fold(f64::INFINITY, f64::min);
                let mx = nums.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                numeric_columns.push(name.clone());
                (Some(s), Some(a), Some(mn), Some(mx))
            } else {
                categorical_columns.push(name.clone());
                (None, None, None, None)
            };

            columns.push(ColumnSummary {
                name: name.clone(),
                is_numeric,
                non_null_count,
                distinct_count: distinct.len(),
                sum,
                avg,
                min,
                max,
            });
        }

        BiSummary {
            row_count: dataset.rows.len(),
            column_count: dataset.columns.len(),
            source_path: dataset.source_path.clone(),
            columns,
            numeric_columns,
            categorical_columns,
        }
    }

    pub fn aggregate_series(
        dataset: &TableDataset,
        x_col: &str,
        y_col: Option<&str>,
        op: AggregationOp,
    ) -> Vec<ChartSeriesPoint> {
        let Some(x_idx) = dataset.columns.iter().position(|c| c == x_col) else {
            return Vec::new();
        };
        let y_idx = y_col.and_then(|yc| dataset.columns.iter().position(|c| c == yc));

        let mut order: Vec<String> = Vec::new();
        let mut groups: std::collections::HashMap<String, Vec<f64>> =
            std::collections::HashMap::new();
        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        for row in &dataset.rows {
            let key = row
                .get(x_idx)
                .map(String::as_str)
                .unwrap_or("(empty)")
                .trim();
            let group_key = if key.is_empty() {
                "(empty)".to_string()
            } else {
                key.to_string()
            };

            if !counts.contains_key(&group_key) {
                order.push(group_key.clone());
            }
            *counts.entry(group_key.clone()).or_insert(0) += 1;

            if let Some(yi) = y_idx {
                if let Some(val) = row.get(yi).and_then(|s| s.trim().parse::<f64>().ok()) {
                    if val.is_finite() {
                        groups.entry(group_key).or_default().push(val);
                    }
                }
            }
        }

        let mut points: Vec<ChartSeriesPoint> = order
            .into_iter()
            .map(|label| {
                let count = counts.get(&label).copied().unwrap_or(0) as f64;
                let vals = groups.get(&label);
                let value = match op {
                    AggregationOp::Count => count,
                    AggregationOp::Sum => vals.map(|v| v.iter().sum()).unwrap_or(0.0),
                    AggregationOp::Avg => vals
                        .filter(|v| !v.is_empty())
                        .map(|v| v.iter().sum::<f64>() / v.len() as f64)
                        .unwrap_or(0.0),
                    AggregationOp::Min => vals
                        .filter(|v| !v.is_empty())
                        .map(|v| v.iter().copied().fold(f64::INFINITY, f64::min))
                        .unwrap_or(0.0),
                    AggregationOp::Max => vals
                        .filter(|v| !v.is_empty())
                        .map(|v| v.iter().copied().fold(f64::NEG_INFINITY, f64::max))
                        .unwrap_or(0.0),
                };
                ChartSeriesPoint {
                    label,
                    value,
                    share_pct: 0.0,
                }
            })
            .collect();

        let total: f64 = points.iter().map(|p| p.value.max(0.0)).sum();
        if total > 0.0 {
            for p in &mut points {
                p.share_pct = (p.value.max(0.0) / total) * 100.0;
            }
        }

        points
    }

    /// Compute a cross-tabulated Pivot Table (row_col × col_col) with the chosen AggregationOp.
    pub fn compute_pivot_table(
        dataset: &TableDataset,
        row_col: &str,
        col_col: &str,
        val_col: Option<&str>,
        op: AggregationOp,
    ) -> PivotTable {
        use std::collections::BTreeSet;

        let row_idx = dataset.columns.iter().position(|c| c == row_col);
        let col_idx = dataset.columns.iter().position(|c| c == col_col);
        let val_idx = val_col.and_then(|yc| dataset.columns.iter().position(|c| c == yc));

        let (Some(r_idx), Some(c_idx)) = (row_idx, col_idx) else {
            return PivotTable {
                row_field: row_col.to_string(),
                col_field: col_col.to_string(),
                col_headers: Vec::new(),
                rows: Vec::new(),
                col_totals: Vec::new(),
                grand_total: 0.0,
            };
        };

        let mut row_keys_set = BTreeSet::new();
        let mut col_keys_set = BTreeSet::new();
        let mut cells: std::collections::HashMap<(String, String), Vec<f64>> =
            std::collections::HashMap::new();
        let mut counts: std::collections::HashMap<(String, String), usize> =
            std::collections::HashMap::new();

        for row in &dataset.rows {
            let rk = row
                .get(r_idx)
                .map(|s| s.trim())
                .filter(|s| !s.is_empty() && *s != "null")
                .unwrap_or("(empty)")
                .to_string();
            let ck = row
                .get(c_idx)
                .map(|s| s.trim())
                .filter(|s| !s.is_empty() && *s != "null")
                .unwrap_or("(empty)")
                .to_string();

            row_keys_set.insert(rk.clone());
            col_keys_set.insert(ck.clone());
            *counts.entry((rk.clone(), ck.clone())).or_insert(0) += 1;

            if let Some(vi) = val_idx {
                if let Some(n) = row
                    .get(vi)
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .filter(|n| n.is_finite())
                {
                    cells.entry((rk, ck)).or_default().push(n);
                }
            }
        }

        let col_headers: Vec<String> = col_keys_set.into_iter().take(16).collect();
        let row_keys: Vec<String> = row_keys_set.into_iter().take(100).collect();

        let eval_cell = |rk: &str, ck: &str| -> Option<f64> {
            let key = (rk.to_string(), ck.to_string());
            let cnt = *counts.get(&key).unwrap_or(&0);
            if cnt == 0 {
                return None;
            }
            if op == AggregationOp::Count || val_idx.is_none() {
                return Some(cnt as f64);
            }
            let nums = cells.get(&key)?;
            if nums.is_empty() {
                return None;
            }
            Some(match op {
                AggregationOp::Count => cnt as f64,
                AggregationOp::Sum => nums.iter().sum(),
                AggregationOp::Avg => nums.iter().sum::<f64>() / nums.len() as f64,
                AggregationOp::Min => nums.iter().copied().fold(f64::INFINITY, f64::min),
                AggregationOp::Max => nums.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            })
        };

        let mut col_totals = vec![0.0_f64; col_headers.len()];
        let mut grand_total = 0.0_f64;
        let mut rows = Vec::with_capacity(row_keys.len());

        for rk in row_keys {
            let mut vals = Vec::with_capacity(col_headers.len());
            let mut row_total = 0.0_f64;
            for (ci, ck) in col_headers.iter().enumerate() {
                let v = eval_cell(&rk, ck);
                if let Some(num) = v {
                    row_total += num;
                    col_totals[ci] += num;
                    grand_total += num;
                }
                vals.push(v);
            }
            rows.push((rk, vals, row_total));
        }

        PivotTable {
            row_field: row_col.to_string(),
            col_field: col_col.to_string(),
            col_headers,
            rows,
            col_totals,
            grand_total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::ResponseDataService;

    fn sample_dataset() -> TableDataset {
        let json = r#"[
            {"region": "North", "revenue": 100, "units": 2},
            {"region": "South", "revenue": 300, "units": 6},
            {"region": "North", "revenue": 200, "units": 4}
        ]"#;
        ResponseDataService::extract_table_dataset(json).expect("sample dataset")
    }

    #[test]
    fn test_summarize_dataset_kpis_and_column_types() {
        let ds = sample_dataset();
        let summary = ResponseBiService::summarize_dataset(&ds);
        assert_eq!(summary.row_count, 3);
        assert_eq!(summary.column_count, 3);
        assert_eq!(summary.numeric_columns, vec!["revenue", "units"]);
        assert_eq!(summary.categorical_columns, vec!["region"]);

        let rev = summary
            .columns
            .iter()
            .find(|c| c.name == "revenue")
            .expect("revenue summary");
        assert!(rev.is_numeric);
        assert_eq!(rev.sum, Some(600.0));
        assert_eq!(rev.avg, Some(200.0));
        assert_eq!(rev.min, Some(100.0));
        assert_eq!(rev.max, Some(300.0));

        let reg = summary
            .columns
            .iter()
            .find(|c| c.name == "region")
            .expect("region summary");
        assert!(!reg.is_numeric);
        assert_eq!(reg.distinct_count, 2);
    }

    #[test]
    fn test_aggregate_series_group_by_sum_avg_count() {
        let ds = sample_dataset();

        let sum_pts = ResponseBiService::aggregate_series(
            &ds,
            "region",
            Some("revenue"),
            AggregationOp::Sum,
        );
        assert_eq!(sum_pts.len(), 2);
        let north = sum_pts.iter().find(|p| p.label == "North").unwrap();
        let south = sum_pts.iter().find(|p| p.label == "South").unwrap();
        assert_eq!(north.value, 300.0);
        assert_eq!(south.value, 300.0);
        assert!((north.share_pct - 50.0).abs() < 0.01);

        let avg_pts = ResponseBiService::aggregate_series(
            &ds,
            "region",
            Some("revenue"),
            AggregationOp::Avg,
        );
        let north_avg = avg_pts.iter().find(|p| p.label == "North").unwrap();
        assert_eq!(north_avg.value, 150.0);

        let count_pts =
            ResponseBiService::aggregate_series(&ds, "region", None, AggregationOp::Count);
        let north_cnt = count_pts.iter().find(|p| p.label == "North").unwrap();
        assert_eq!(north_cnt.value, 2.0);
    }
}
