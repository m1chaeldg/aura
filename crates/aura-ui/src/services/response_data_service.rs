//! Response Data Service: Tree building and tabular dataset extraction, filtering, and sorting.

use anyhow::{anyhow, Result};
use std::collections::{BTreeSet, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeNodeKind {
    Object,
    Array,
    String,
    Number,
    Bool,
    Null,
    XmlElement,
}

impl TreeNodeKind {
    pub fn badge(self) -> &'static str {
        match self {
            Self::Object => "obj",
            Self::Array => "arr",
            Self::String => "str",
            Self::Number => "num",
            Self::Bool => "bool",
            Self::Null => "null",
            Self::XmlElement => "xml",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode {
    pub id: String,
    pub key: String,
    pub kind: TreeNodeKind,
    pub preview: String,
    pub copy_value: String,
    pub depth: usize,
    pub has_children: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableDataset {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub source_path: String,
}

pub struct ResponseDataService;

impl ResponseDataService {
    pub fn build_visible_tree(
        body: &str,
        collapsed_ids: &HashSet<String>,
    ) -> Result<Vec<TreeNode>> {
        let trimmed = body.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("Response body is empty"));
        }
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            let mut out = Vec::new();
            flatten_json_tree("$", "$", &val, 0, collapsed_ids, &mut out);
            return Ok(out);
        }
        if trimmed.starts_with('<') {
            return build_xml_tree(trimmed, collapsed_ids);
        }
        Err(anyhow!("Body is neither valid JSON nor XML"))
    }

    pub fn extract_table_dataset(body: &str) -> Result<TableDataset> {
        let trimmed = body.trim();
        let val: serde_json::Value =
            serde_json::from_str(trimmed).map_err(|e| anyhow!("Table requires JSON: {e}"))?;

        let (arr, source_path) = match &val {
            serde_json::Value::Array(items) => (items.as_slice(), "$".to_string()),
            serde_json::Value::Object(map) => {
                let preferred = ["data", "items", "results", "records", "rows"];
                let found_preferred = preferred.iter().find_map(|key| {
                    map.get(*key)
                        .and_then(|v| v.as_array())
                        .map(|a| (a.as_slice(), format!("$.{key}")))
                });
                if let Some(found) = found_preferred {
                    found
                } else if let Some((k, a)) = map.iter().find_map(|(k, v)| {
                    v.as_array()
                        .filter(|a| a.iter().any(|item| item.is_object()))
                        .map(|a| (k, a.as_slice()))
                }) {
                    (a, format!("$.{k}"))
                } else {
                    // Single object fallback: Key / Value table
                    let rows = map
                        .iter()
                        .map(|(k, v)| vec![k.clone(), format_cell_value(v)])
                        .collect();
                    return Ok(TableDataset {
                        columns: vec!["Key".to_string(), "Value".to_string()],
                        rows,
                        source_path: "$".to_string(),
                    });
                }
            }
            _ => return Err(anyhow!("JSON must be an array or object to view as a table")),
        };

        if arr.is_empty() {
            return Ok(TableDataset {
                columns: Vec::new(),
                rows: Vec::new(),
                source_path,
            });
        }

        let mut col_set = BTreeSet::new();
        let mut flat_rows: Vec<std::collections::BTreeMap<String, String>> =
            Vec::with_capacity(arr.len());

        for item in arr {
            let mut row_map = std::collections::BTreeMap::new();
            match item {
                serde_json::Value::Object(obj) => {
                    for (k, v) in obj {
                        if let serde_json::Value::Object(nested) = v {
                            if nested.is_empty() {
                                col_set.insert(k.clone());
                                row_map.insert(k.clone(), "{}".to_string());
                            } else {
                                for (nk, nv) in nested {
                                    let full_key = format!("{k}.{nk}");
                                    col_set.insert(full_key.clone());
                                    row_map.insert(full_key, format_cell_value(nv));
                                }
                            }
                        } else {
                            col_set.insert(k.clone());
                            row_map.insert(k.clone(), format_cell_value(v));
                        }
                    }
                }
                other => {
                    col_set.insert("value".to_string());
                    row_map.insert("value".to_string(), format_cell_value(other));
                }
            }
            flat_rows.push(row_map);
        }

        let columns: Vec<String> = col_set.into_iter().collect();
        let rows = flat_rows
            .into_iter()
            .map(|row_map| {
                columns
                    .iter()
                    .map(|col| row_map.get(col).cloned().unwrap_or_default())
                    .collect()
            })
            .collect();

        Ok(TableDataset {
            columns,
            rows,
            source_path,
        })
    }

    pub fn filter_and_sort_table(
        dataset: &TableDataset,
        query: &str,
        sort_col: Option<usize>,
        sort_asc: bool,
    ) -> Vec<Vec<String>> {
        let q = query.trim().to_ascii_lowercase();
        let mut rows: Vec<Vec<String>> = if q.is_empty() {
            dataset.rows.clone()
        } else {
            dataset
                .rows
                .iter()
                .filter(|row| row.iter().any(|cell| cell.to_ascii_lowercase().contains(&q)))
                .cloned()
                .collect()
        };

        if let Some(col_idx) = sort_col {
            rows.sort_by(|a, b| {
                let va = a.get(col_idx).map(String::as_str).unwrap_or("");
                let vb = b.get(col_idx).map(String::as_str).unwrap_or("");
                let ord = match (va.parse::<f64>(), vb.parse::<f64>()) {
                    (Ok(na), Ok(nb)) => na
                        .partial_cmp(&nb)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    _ => va.to_ascii_lowercase().cmp(&vb.to_ascii_lowercase()),
                };
                if sort_asc {
                    ord
                } else {
                    ord.reverse()
                }
            });
        }

        rows
    }
}

fn format_cell_value(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(a) => format!("[{} items]", a.len()),
        serde_json::Value::Object(o) => format!("{{{} keys}}", o.len()),
    }
}

fn flatten_json_tree(
    id: &str,
    key: &str,
    val: &serde_json::Value,
    depth: usize,
    collapsed_ids: &HashSet<String>,
    out: &mut Vec<TreeNode>,
) {
    match val {
        serde_json::Value::Object(map) => {
            let has_children = !map.is_empty();
            out.push(TreeNode {
                id: id.to_string(),
                key: key.to_string(),
                kind: TreeNodeKind::Object,
                preview: format!("{{{} keys}}", map.len()),
                copy_value: serde_json::to_string_pretty(val).unwrap_or_default(),
                depth,
                has_children,
            });
            if has_children && !collapsed_ids.contains(id) {
                for (k, v) in map {
                    let child_id = format!("{id}.{k}");
                    flatten_json_tree(&child_id, k, v, depth + 1, collapsed_ids, out);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            let has_children = !arr.is_empty();
            out.push(TreeNode {
                id: id.to_string(),
                key: key.to_string(),
                kind: TreeNodeKind::Array,
                preview: format!("[{} items]", arr.len()),
                copy_value: serde_json::to_string_pretty(val).unwrap_or_default(),
                depth,
                has_children,
            });
            if has_children && !collapsed_ids.contains(id) {
                for (idx, v) in arr.iter().enumerate() {
                    let child_id = format!("{id}[{idx}]");
                    let child_key = format!("[{idx}]");
                    flatten_json_tree(&child_id, &child_key, v, depth + 1, collapsed_ids, out);
                }
            }
        }
        serde_json::Value::String(s) => out.push(TreeNode {
            id: id.to_string(),
            key: key.to_string(),
            kind: TreeNodeKind::String,
            preview: format!("\"{s}\""),
            copy_value: s.clone(),
            depth,
            has_children: false,
        }),
        serde_json::Value::Number(n) => out.push(TreeNode {
            id: id.to_string(),
            key: key.to_string(),
            kind: TreeNodeKind::Number,
            preview: n.to_string(),
            copy_value: n.to_string(),
            depth,
            has_children: false,
        }),
        serde_json::Value::Bool(b) => out.push(TreeNode {
            id: id.to_string(),
            key: key.to_string(),
            kind: TreeNodeKind::Bool,
            preview: b.to_string(),
            copy_value: b.to_string(),
            depth,
            has_children: false,
        }),
        serde_json::Value::Null => out.push(TreeNode {
            id: id.to_string(),
            key: key.to_string(),
            kind: TreeNodeKind::Null,
            preview: "null".to_string(),
            copy_value: "null".to_string(),
            depth,
            has_children: false,
        }),
    }
}

struct XmlElem {
    name: String,
    attrs: String,
    text: String,
    children: Vec<XmlElem>,
}

fn build_xml_tree(raw: &str, collapsed_ids: &HashSet<String>) -> Result<Vec<TreeNode>> {
    let roots = parse_xml_elements(raw)?;
    if roots.is_empty() {
        return Err(anyhow!("No XML elements found"));
    }
    let mut out = Vec::new();
    for (idx, root) in roots.iter().enumerate() {
        let id = format!("/{}{}", root.name, if idx == 0 { String::new() } else { format!("[{idx}]") });
        flatten_xml_tree(&id, root, 0, collapsed_ids, &mut out);
    }
    Ok(out)
}

fn flatten_xml_tree(
    id: &str,
    elem: &XmlElem,
    depth: usize,
    collapsed_ids: &HashSet<String>,
    out: &mut Vec<TreeNode>,
) {
    let has_children = !elem.children.is_empty();
    let preview = if !elem.text.is_empty() && !has_children {
        elem.text.clone()
    } else if !elem.attrs.is_empty() {
        format!("<{} {}> ({} children)", elem.name, elem.attrs, elem.children.len())
    } else {
        format!("({} children)", elem.children.len())
    };
    let copy_value = if !elem.text.is_empty() {
        elem.text.clone()
    } else {
        elem.name.clone()
    };

    out.push(TreeNode {
        id: id.to_string(),
        key: elem.name.clone(),
        kind: TreeNodeKind::XmlElement,
        preview,
        copy_value,
        depth,
        has_children,
    });

    if has_children && !collapsed_ids.contains(id) {
        for (idx, child) in elem.children.iter().enumerate() {
            let child_id = format!("{id}/{}[{idx}]", child.name);
            flatten_xml_tree(&child_id, child, depth + 1, collapsed_ids, out);
        }
    }
}

fn parse_xml_elements(raw: &str) -> Result<Vec<XmlElem>> {
    let mut stack: Vec<XmlElem> = Vec::new();
    let mut roots: Vec<XmlElem> = Vec::new();
    let mut rest = raw.trim();

    while !rest.is_empty() {
        if rest.starts_with("<!--") {
            let end = rest.find("-->").ok_or_else(|| anyhow!("Unterminated XML comment"))?;
            rest = &rest[end + 3..];
        } else if rest.starts_with("<![CDATA[") {
            let end = rest.find("]]>").ok_or_else(|| anyhow!("Unterminated CDATA"))?;
            let text = &rest[9..end];
            if let Some(top) = stack.last_mut() {
                top.text.push_str(text);
            }
            rest = &rest[end + 3..];
        } else if rest.starts_with("<?") || rest.starts_with("<!") {
            let end = rest.find('>').ok_or_else(|| anyhow!("Unterminated XML decl"))?;
            rest = &rest[end + 1..];
        } else if rest.starts_with("</") {
            let end = rest.find('>').ok_or_else(|| anyhow!("Unterminated closing tag"))?;
            if let Some(completed) = stack.pop() {
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(completed);
                } else {
                    roots.push(completed);
                }
            }
            rest = &rest[end + 1..];
        } else if rest.starts_with('<') {
            let end = rest.find('>').ok_or_else(|| anyhow!("Unterminated tag"))?;
            let inside = rest[1..end].trim();
            let self_closing = inside.ends_with('/');
            let header = inside.trim_end_matches('/').trim();
            let mut parts = header.splitn(2, char::is_whitespace);
            let name = parts.next().unwrap_or("").to_string();
            let attrs = parts.next().unwrap_or("").trim().to_string();
            let elem = XmlElem {
                name,
                attrs,
                text: String::new(),
                children: Vec::new(),
            };
            if self_closing {
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(elem);
                } else {
                    roots.push(elem);
                }
            } else {
                stack.push(elem);
            }
            rest = &rest[end + 1..];
        } else if let Some(next_tag) = rest.find('<') {
            let text = rest[..next_tag].trim();
            if !text.is_empty() {
                if let Some(top) = stack.last_mut() {
                    if !top.text.is_empty() {
                        top.text.push(' ');
                    }
                    top.text.push_str(text);
                }
            }
            rest = &rest[next_tag..];
        } else {
            break;
        }
    }

    while let Some(completed) = stack.pop() {
        if let Some(parent) = stack.last_mut() {
            parent.children.push(completed);
        } else {
            roots.push(completed);
        }
    }

    Ok(roots)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_visible_tree_json_expand_collapse() {
        let json = r#"{"user":{"name":"Ada","age":30},"tags":["rust","gpui"]}"#;
        let expanded = ResponseDataService::build_visible_tree(json, &HashSet::new())
            .expect("parse json tree");
        assert_eq!(expanded[0].id, "$");
        assert_eq!(expanded[0].kind, TreeNodeKind::Object);
        assert!(expanded.iter().any(|n| n.id == "$.user.name" && n.preview == "\"Ada\""));
        assert!(expanded.iter().any(|n| n.id == "$.tags[1]" && n.preview == "\"gpui\""));

        let mut collapsed = HashSet::new();
        collapsed.insert("$.user".to_string());
        let after_collapse =
            ResponseDataService::build_visible_tree(json, &collapsed).expect("collapsed tree");
        assert!(!after_collapse.iter().any(|n| n.id == "$.user.name"));
        assert!(after_collapse.iter().any(|n| n.id == "$.tags[0]"));
    }

    #[test]
    fn test_build_visible_tree_xml() {
        let xml = r#"<users><user id="1"><name>Ada</name></user></users>"#;
        let nodes =
            ResponseDataService::build_visible_tree(xml, &HashSet::new()).expect("parse xml tree");
        assert_eq!(nodes[0].key, "users");
        assert_eq!(nodes[0].kind, TreeNodeKind::XmlElement);
        assert!(nodes.iter().any(|n| n.key == "name" && n.preview == "Ada"));
    }

    #[test]
    fn test_extract_table_dataset_nested_array_and_flattening() {
        let json = r#"{
            "meta": {"total": 2},
            "data": [
                {"id": 10, "user": {"name": "Ada"}, "score": 95.5},
                {"id": 2, "user": {"name": "Grace"}, "score": 99.0}
            ]
        }"#;
        let ds = ResponseDataService::extract_table_dataset(json).expect("extract table");
        assert_eq!(ds.source_path, "$.data");
        assert_eq!(ds.columns, vec!["id", "score", "user.name"]);
        assert_eq!(ds.rows.len(), 2);

        // Numeric sort ascending on "id" (2 before 10, not "10" before "2")
        let sorted = ResponseDataService::filter_and_sort_table(&ds, "", Some(0), true);
        assert_eq!(sorted[0][0], "2");
        assert_eq!(sorted[1][0], "10");

        // Case-insensitive search filter
        let filtered = ResponseDataService::filter_and_sort_table(&ds, "grace", None, true);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0][2], "Grace");
    }
}
