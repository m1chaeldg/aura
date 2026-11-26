//! GraphQL schema introspection parsing, query linting, and autocomplete helpers.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub const INTROSPECTION_QUERY: &str = r#"query IntrospectionQuery {
  __schema {
    queryType { name }
    mutationType { name }
    subscriptionType { name }
    types {
      kind
      name
      description
      fields(includeDeprecated: true) {
        name
        description
        args {
          name
          description
          type { ...TypeRef }
          defaultValue
        }
        type { ...TypeRef }
      }
      inputFields {
        name
        description
        type { ...TypeRef }
        defaultValue
      }
      enumValues(includeDeprecated: true) {
        name
        description
      }
    }
  }
}

fragment TypeRef on __Type {
  kind
  name
  ofType {
    kind
    name
    ofType {
      kind
      name
      ofType {
        kind
        name
        ofType {
          kind
          name
        }
      }
    }
  }
}"#;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphQLArgInfo {
    pub name: String,
    pub type_name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphQLFieldInfo {
    pub name: String,
    pub return_type: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub args: Vec<GraphQLArgInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphQLTypeInfo {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub fields: Vec<GraphQLFieldInfo>,
    #[serde(default)]
    pub enum_values: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GraphQLSchema {
    pub query_type: Option<String>,
    pub mutation_type: Option<String>,
    pub subscription_type: Option<String>,
    pub types: HashMap<String, GraphQLTypeInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphQLLintDiagnostic {
    pub line: usize,
    pub column: usize,
    pub severity: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphQLCompletionItem {
    pub label: String,
    pub kind: &'static str,
    pub detail: String,
    pub insert_text: String,
}

fn format_type_ref(val: &Value) -> String {
    let kind = val["kind"].as_str().unwrap_or("");
    let name = val["name"].as_str();
    let of_type = &val["ofType"];
    match kind {
        "NON_NULL" => format!("{}!", format_type_ref(of_type)),
        "LIST" => format!("[{}]", format_type_ref(of_type)),
        _ => name.unwrap_or("Unknown").to_string(),
    }
}

pub fn unwrap_type_name(type_ref: &str) -> &str {
    type_ref.trim_matches(|c| c == '[' || c == ']' || c == '!')
}

pub fn parse_introspection_response(json_str: &str) -> Result<GraphQLSchema> {
    let root: Value =
        serde_json::from_str(json_str).context("Invalid GraphQL introspection JSON response")?;
    if let Some(errors) = root.get("errors").and_then(|e| e.as_array()) {
        if !errors.is_empty() && root.get("data").is_none() {
            let msg = errors[0]["message"]
                .as_str()
                .unwrap_or("Introspection failed");
            return Err(anyhow!("GraphQL introspection error: {msg}"));
        }
    }

    let schema_val = root
        .get("data")
        .and_then(|d| d.get("__schema"))
        .or_else(|| root.get("__schema"))
        .ok_or_else(|| anyhow!("Response does not contain data.__schema"))?;

    let query_type = schema_val["queryType"]["name"]
        .as_str()
        .map(ToString::to_string);
    let mutation_type = schema_val["mutationType"]["name"]
        .as_str()
        .map(ToString::to_string);
    let subscription_type = schema_val["subscriptionType"]["name"]
        .as_str()
        .map(ToString::to_string);

    let mut types = HashMap::new();
    if let Some(arr) = schema_val["types"].as_array() {
        for t in arr {
            let Some(name) = t["name"].as_str() else {
                continue;
            };
            let kind = t["kind"].as_str().unwrap_or("OBJECT").to_string();
            let description = t["description"].as_str().map(ToString::to_string);

            let mut fields = Vec::new();
            let raw_fields = t["fields"]
                .as_array()
                .or_else(|| t["inputFields"].as_array());
            if let Some(f_arr) = raw_fields {
                for f in f_arr {
                    let Some(fname) = f["name"].as_str() else {
                        continue;
                    };
                    let return_type = format_type_ref(&f["type"]);
                    let fdesc = f["description"].as_str().map(ToString::to_string);
                    let mut args = Vec::new();
                    if let Some(a_arr) = f["args"].as_array() {
                        for a in a_arr {
                            if let Some(aname) = a["name"].as_str() {
                                args.push(GraphQLArgInfo {
                                    name: aname.to_string(),
                                    type_name: format_type_ref(&a["type"]),
                                    description: a["description"].as_str().map(ToString::to_string),
                                });
                            }
                        }
                    }
                    fields.push(GraphQLFieldInfo {
                        name: fname.to_string(),
                        return_type,
                        description: fdesc,
                        args,
                    });
                }
            }

            let mut enum_values = Vec::new();
            if let Some(ev_arr) = t["enumValues"].as_array() {
                for ev in ev_arr {
                    if let Some(ev_name) = ev["name"].as_str() {
                        enum_values.push(ev_name.to_string());
                    }
                }
            }

            types.insert(
                name.to_string(),
                GraphQLTypeInfo {
                    name: name.to_string(),
                    kind,
                    description,
                    fields,
                    enum_values,
                },
            );
        }
    }

    Ok(GraphQLSchema {
        query_type,
        mutation_type,
        subscription_type,
        types,
    })
}

pub fn lint_graphql_query(
    query: &str,
    schema: Option<&GraphQLSchema>,
) -> Vec<GraphQLLintDiagnostic> {
    let mut diags = Vec::new();
    let trimmed = query.trim();
    if trimmed.is_empty() {
        diags.push(GraphQLLintDiagnostic {
            line: 1,
            column: 1,
            severity: "warning",
            message: "GraphQL query is empty.".to_string(),
        });
        return diags;
    }

    // Track delimiters, comments, and strings
    let mut brace_stack: Vec<(usize, usize)> = Vec::new();
    let mut paren_stack: Vec<(usize, usize)> = Vec::new();
    let mut in_string = false;
    let mut escape = false;
    let mut in_comment = false;
    let mut line = 1usize;
    let mut col = 1usize;

    let chars: Vec<char> = query.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\n' {
            if in_string {
                diags.push(GraphQLLintDiagnostic {
                    line,
                    column: col,
                    severity: "error",
                    message: "Unterminated string literal before newline.".to_string(),
                });
                in_string = false;
            }
            in_comment = false;
            line += 1;
            col = 1;
            i += 1;
            continue;
        }
        if in_comment {
            col += 1;
            i += 1;
            continue;
        }
        if in_string {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            col += 1;
            i += 1;
            continue;
        }

        match ch {
            '#' => in_comment = true,
            '"' => {
                if i + 2 < chars.len() && chars[i + 1] == '"' && chars[i + 2] == '"' {
                    // Block string: advance until closing """
                    i += 3;
                    col += 3;
                    let mut closed = false;
                    while i < chars.len() {
                        if chars[i] == '\n' {
                            line += 1;
                            col = 1;
                        } else {
                            col += 1;
                        }
                        if i + 2 < chars.len()
                            && chars[i] == '"'
                            && chars[i + 1] == '"'
                            && chars[i + 2] == '"'
                        {
                            i += 2;
                            col += 2;
                            closed = true;
                            break;
                        }
                        i += 1;
                    }
                    if !closed {
                        diags.push(GraphQLLintDiagnostic {
                            line,
                            column: col,
                            severity: "error",
                            message: "Unterminated block string literal.".to_string(),
                        });
                    }
                } else {
                    in_string = true;
                }
            }
            '{' => brace_stack.push((line, col)),
            '}' => {
                if brace_stack.pop().is_none() {
                    diags.push(GraphQLLintDiagnostic {
                        line,
                        column: col,
                        severity: "error",
                        message: "Unexpected closing brace '}'.".to_string(),
                    });
                }
            }
            '(' => paren_stack.push((line, col)),
            ')' => {
                if paren_stack.pop().is_none() {
                    diags.push(GraphQLLintDiagnostic {
                        line,
                        column: col,
                        severity: "error",
                        message: "Unexpected closing parenthesis ')'.".to_string(),
                    });
                }
            }
            _ => {}
        }
        col += 1;
        i += 1;
    }

    if in_string {
        diags.push(GraphQLLintDiagnostic {
            line,
            column: col,
            severity: "error",
            message: "Unterminated string literal at end of query.".to_string(),
        });
    }
    if let Some((bl, bc)) = brace_stack.pop() {
        diags.push(GraphQLLintDiagnostic {
            line: bl,
            column: bc,
            severity: "error",
            message: "Unclosed selection set '{'.".to_string(),
        });
    }
    if let Some((pl, pc)) = paren_stack.pop() {
        diags.push(GraphQLLintDiagnostic {
            line: pl,
            column: pc,
            severity: "error",
            message: "Unclosed argument/variable list '('.".to_string(),
        });
    }

    // Validate fragment definitions vs spreads
    let mut defined_fragments = HashSet::new();
    for raw_line in query.lines() {
        let s = raw_line.trim();
        if let Some(rest) = s.strip_prefix("fragment ") {
            if let Some(name) = rest.split_whitespace().next() {
                defined_fragments.insert(name.to_string());
            }
        }
    }
    for (idx, raw_line) in query.lines().enumerate() {
        let s = raw_line.split('#').next().unwrap_or("");
        let mut search = s;
        while let Some(pos) = search.find("...") {
            let after = &search[pos + 3..];
            if !after.trim_start().starts_with("on ") {
                let frag: String = after
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !frag.is_empty() && !defined_fragments.contains(&frag) {
                    diags.push(GraphQLLintDiagnostic {
                        line: idx + 1,
                        column: pos + 1,
                        severity: "error",
                        message: format!("Fragment '{frag}' is not defined."),
                    });
                }
            }
            search = after;
        }
    }

    // Optional schema-aware root field validation
    if let Some(schema) = schema {
        let first_word = trimmed
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .find(|s| !s.is_empty())
            .unwrap_or("query");
        let root_type_name = match first_word {
            "mutation" => schema.mutation_type.as_deref(),
            "subscription" => schema.subscription_type.as_deref(),
            _ => schema.query_type.as_deref(),
        };
        if let Some(rt_name) = root_type_name {
            if let Some(rt) = schema.types.get(rt_name) {
                let valid_fields: HashSet<&str> =
                    rt.fields.iter().map(|f| f.name.as_str()).collect();
                // Inspect top-level brace depth 1 field identifiers
                let mut depth = 0i32;
                for (line_idx, raw_line) in query.lines().enumerate() {
                    let clean = raw_line.split('#').next().unwrap_or("");
                    let prev_depth = depth;
                    for ch in clean.chars() {
                        if ch == '{' {
                            depth += 1;
                        } else if ch == '}' {
                            depth -= 1;
                        }
                    }
                    if prev_depth == 1 {
                        let candidate = clean.trim();
                        if !candidate.is_empty()
                            && !candidate.starts_with('}')
                            && !candidate.starts_with("...")
                        {
                            let header = candidate
                                .split(['(', '{', '@'])
                                .next()
                                .unwrap_or(candidate);
                            let token = header
                                .split(':')
                                .last()
                                .unwrap_or(header)
                                .trim()
                                .split(|c: char| !c.is_alphanumeric() && c != '_')
                                .find(|s| !s.is_empty());
                            if let Some(field) = token {
                                if !field.starts_with("__") && !valid_fields.contains(field) {
                                    diags.push(GraphQLLintDiagnostic {
                                        line: line_idx + 1,
                                        column: 1,
                                        severity: "warning",
                                        message: format!(
                                            "Field '{field}' was not found on schema type '{rt_name}'."
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    diags
}

pub fn graphql_completions(
    query: &str,
    schema: Option<&GraphQLSchema>,
) -> Vec<GraphQLCompletionItem> {
    let mut items = Vec::new();
    let lower = query.to_ascii_lowercase();
    let trimmed = lower.trim();

    if trimmed.is_empty() || !trimmed.contains('{') {
        for (kw, snippet, desc) in [
            ("query", "query {\n  \n}", "Query operation"),
            ("mutation", "mutation {\n  \n}", "Mutation operation"),
            (
                "subscription",
                "subscription {\n  \n}",
                "Subscription operation",
            ),
            ("fragment", "fragment Fields on Type {\n  \n}", "Reusable fragment"),
        ] {
            items.push(GraphQLCompletionItem {
                label: kw.to_string(),
                kind: "Keyword",
                detail: desc.to_string(),
                insert_text: snippet.to_string(),
            });
        }
    }

    items.push(GraphQLCompletionItem {
        label: "__schema".to_string(),
        kind: "Field",
        detail: "Introspection schema root".to_string(),
        insert_text: "__schema {\n  types {\n    name\n  }\n}".to_string(),
    });
    items.push(GraphQLCompletionItem {
        label: "__typename".to_string(),
        kind: "Field",
        detail: "Runtime type name".to_string(),
        insert_text: "__typename".to_string(),
    });

    if let Some(schema) = schema {
        let mut seen = HashSet::new();
        // Prioritize Query/Mutation/Subscription fields first
        for root_opt in [
            schema.query_type.as_deref(),
            schema.mutation_type.as_deref(),
            schema.subscription_type.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(t) = schema.types.get(root_opt) {
                for f in &t.fields {
                    if seen.insert(format!("{}.{}", t.name, f.name)) {
                        let arg_hint = if f.args.is_empty() {
                            String::new()
                        } else {
                            let joined: Vec<String> = f
                                .args
                                .iter()
                                .map(|a| format!("{}: ", a.name))
                                .collect();
                            format!("({})", joined.join(", "))
                        };
                        items.push(GraphQLCompletionItem {
                            label: f.name.clone(),
                            kind: "Field",
                            detail: format!("{}.{} -> {}", t.name, f.name, f.return_type),
                            insert_text: format!("{}{}", f.name, arg_hint),
                        });
                    }
                }
            }
        }

        // Also expose non-introspection object fields and args
        let mut type_names: Vec<&String> = schema.types.keys().collect();
        type_names.sort();
        for tname in type_names {
            if tname.starts_with("__") {
                continue;
            }
            if let Some(t) = schema.types.get(tname) {
                for f in &t.fields {
                    if seen.insert(format!("{}.{}", t.name, f.name)) {
                        items.push(GraphQLCompletionItem {
                            label: f.name.clone(),
                            kind: "Field",
                            detail: format!("{}.{} -> {}", t.name, f.name, f.return_type),
                            insert_text: f.name.clone(),
                        });
                    }
                }
            }
        }
    }

    items
}
