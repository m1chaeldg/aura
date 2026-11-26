//! Code editor intelligence: autocomplete (`CompletionProvider`), JS/Spec
//! reference catalog, and active variable/parameter context for `EditorState`.

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;
use gpui::{App, Task, Window};
use gpui_kit::component::input::{CompletionProvider, Rope};
use lsp_types::{
    CompletionContext, CompletionItem, CompletionItemKind, CompletionResponse, Documentation,
    MarkupContent, MarkupKind,
};

/// Language mode for `AuraCompletionProvider`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionLanguage {
    JavaScript { include_response: bool },
    TomlSpec,
}

/// Live request/tab context fed into `AuraCompletionProvider` so autocomplete
/// suggests the current tab's variables, query parameters, and headers.
#[derive(Debug, Clone, Default)]
pub struct EditorContextData {
    pub variables: Vec<(String, String, bool)>,
    pub query_params: Vec<(String, String, bool)>,
    pub headers: Vec<(String, String, bool)>,
}

/// Static documentation entry used for both autocomplete and the Reference panel.
#[derive(Debug, Clone, Copy)]
pub struct ReferenceEntry {
    pub category: &'static str,
    pub label: &'static str,
    pub detail: &'static str,
    pub insert_text: &'static str,
    pub description: &'static str,
}

pub const JS_REFERENCE_ENTRIES: &[ReferenceEntry] = &[
    // Aura Sandbox
    ReferenceEntry {
        category: "Aura Sandbox",
        label: "aura.variables.get",
        detail: "(key: string) => string | undefined",
        insert_text: "aura.variables.get(\"key\")",
        description: "Reads a variable from the active request or environment scope.",
    },
    ReferenceEntry {
        category: "Aura Sandbox",
        label: "aura.variables.set",
        detail: "(key: string, value: any) => void",
        insert_text: "aura.variables.set(\"key\", \"value\");\n",
        description: "Sets or updates a variable for subsequent scripts and requests.",
    },
    ReferenceEntry {
        category: "Aura Sandbox",
        label: "aura.test",
        detail: "(name: string, fn: () => void | Promise<void>) => void",
        insert_text: "aura.test(\"should succeed\", () => {\n    aura.expect(response.status).toBe(200);\n});\n",
        description: "Registers a test case (alias for it()).",
    },
    ReferenceEntry {
        category: "Aura Sandbox",
        label: "aura.expect",
        detail: "(actual: any) => Expectation",
        insert_text: "aura.expect(response.status).toBe(200);",
        description: "Creates a Jasmine/Chai expectation (alias for global expect()).",
    },
    ReferenceEntry {
        category: "Aura Sandbox",
        label: "console.log",
        detail: "(...args: any[]) => void",
        insert_text: "console.log(\"debug:\", );",
        description: "Writes an INFO entry to Aura's Console output.",
    },
    ReferenceEntry {
        category: "Aura Sandbox",
        label: "console.warn",
        detail: "(...args: any[]) => void",
        insert_text: "console.warn(\"warning:\", );",
        description: "Writes a WARN entry to Aura's Console output.",
    },
    ReferenceEntry {
        category: "Aura Sandbox",
        label: "console.error",
        detail: "(...args: any[]) => void",
        insert_text: "console.error(\"error:\", );",
        description: "Writes an ERROR entry to Aura's Console output.",
    },
    // Response Object
    ReferenceEntry {
        category: "Response Object",
        label: "response.status",
        detail: "number",
        insert_text: "response.status",
        description: "HTTP status code returned by the server (e.g. 200, 404).",
    },
    ReferenceEntry {
        category: "Response Object",
        label: "response.duration_ms",
        detail: "number",
        insert_text: "response.duration_ms",
        description: "Total request round-trip duration in milliseconds.",
    },
    ReferenceEntry {
        category: "Response Object",
        label: "response.body",
        detail: "string",
        insert_text: "response.body",
        description: "Raw response body text.",
    },
    ReferenceEntry {
        category: "Response Object",
        label: "response.headers",
        detail: "Record<string, string>",
        insert_text: "response.headers[\"content-type\"]",
        description: "Map of lowercase response header names to values.",
    },
    ReferenceEntry {
        category: "Response Object",
        label: "response.json()",
        detail: "() => any",
        insert_text: "response.json()",
        description: "Parses response.body as JSON and returns the object.",
    },
    // Jasmine / Chai BDD
    ReferenceEntry {
        category: "Jasmine / Chai BDD",
        label: "describe",
        detail: "(suiteName: string, fn: () => void) => void",
        insert_text: "describe(\"Suite\", () => {\n    it(\"works\", () => {\n        expect(response.status).toBe(200);\n    });\n});\n",
        description: "Groups related test assertions into a named suite.",
    },
    ReferenceEntry {
        category: "Jasmine / Chai BDD",
        label: "it",
        detail: "(testName: string, fn: () => void | Promise<void>) => Promise",
        insert_text: "it(\"passes\", () => {\n    expect(response.status).toBe(200);\n});\n",
        description: "Defines an individual sync or async test case.",
    },
    ReferenceEntry {
        category: "Jasmine / Chai BDD",
        label: "expect(...).toBe",
        detail: "(expected: any) => void",
        insert_text: "expect(response.status).toBe(200);",
        description: "Strict equality check (===).",
    },
    ReferenceEntry {
        category: "Jasmine / Chai BDD",
        label: "expect(...).toEqual",
        detail: "(expected: any) => void",
        insert_text: "expect(data).toEqual({});",
        description: "Deep structural equality check for objects and arrays.",
    },
    ReferenceEntry {
        category: "Jasmine / Chai BDD",
        label: "expect(...).toContain",
        detail: "(item: any) => void",
        insert_text: "expect(response.body).toContain(\"ok\");",
        description: "Checks that a string or array contains the given item.",
    },
    ReferenceEntry {
        category: "Jasmine / Chai BDD",
        label: "expect(...).toBeDefined",
        detail: "() => void",
        insert_text: "expect(data.id).toBeDefined();",
        description: "Asserts the value is not undefined.",
    },
    ReferenceEntry {
        category: "Jasmine / Chai BDD",
        label: "expect(...).toBeLessThan",
        detail: "(n: number) => void",
        insert_text: "expect(response.duration_ms).toBeLessThan(500);",
        description: "Asserts a numeric value is strictly less than n.",
    },
    ReferenceEntry {
        category: "Jasmine / Chai BDD",
        label: "expect(...).to.have.property",
        detail: "(prop: string) => void",
        insert_text: "expect(data).to.have.property(\"id\");",
        description: "Chai BDD assertion checking own property existence.",
    },
    // Faker & Offline Data
    ReferenceEntry {
        category: "Faker (Offline)",
        label: "faker.internet.email()",
        detail: "() => string",
        insert_text: "faker.internet.email()",
        description: "Generates a deterministic/random offline email address.",
    },
    ReferenceEntry {
        category: "Faker (Offline)",
        label: "faker.person.fullName()",
        detail: "() => string",
        insert_text: "faker.person.fullName()",
        description: "Generates a full name offline.",
    },
    ReferenceEntry {
        category: "Faker (Offline)",
        label: "faker.string.uuid()",
        detail: "() => string",
        insert_text: "faker.string.uuid()",
        description: "Generates a UUID v4 string offline.",
    },
    ReferenceEntry {
        category: "Faker (Offline)",
        label: "faker.number.int()",
        detail: "(opts?: number | { min?: number, max?: number }) => number",
        insert_text: "faker.number.int({ min: 1, max: 100 })",
        description: "Generates a random integer in [min, max].",
    },
    ReferenceEntry {
        category: "Faker (Offline)",
        label: "faker.lorem.sentence()",
        detail: "() => string",
        insert_text: "faker.lorem.sentence()",
        description: "Generates a short placeholder sentence.",
    },
];

pub const SPEC_REFERENCE_ENTRIES: &[ReferenceEntry] = &[
    ReferenceEntry {
        category: "Spec Sections",
        label: "[settings]",
        detail: "Request execution settings",
        insert_text: "\n[settings]\ninherit_auth = true\nverify_ssl = true\nfollow_redirects = true\nmax_redirects = 10\ntimeout_ms = 0\n",
        description: "Configures SSL, redirects, auth inheritance, and timeout.",
    },
    ReferenceEntry {
        category: "Spec Sections",
        label: "[headers]",
        detail: "HTTP request headers",
        insert_text: "\n[headers]\nContent-Type = \"application/json\"\nAccept = \"application/json\"\n",
        description: "Key-value map of HTTP headers (prefix disabled keys with ~).",
    },
    ReferenceEntry {
        category: "Spec Sections",
        label: "[params.query]",
        detail: "URL query parameters",
        insert_text: "\n[params.query]\npage = \"1\"\n",
        description: "Query string parameters appended to the request URL.",
    },
    ReferenceEntry {
        category: "Spec Sections",
        label: "[auth]",
        detail: "Authentication configuration",
        insert_text: "\n[auth]\ntype = \"bearer\"\ntoken = \"{{api_token}}\"\n",
        description: "Supports bearer, basic, apikey, awssigv4, and oauth2.",
    },
    ReferenceEntry {
        category: "Spec Sections",
        label: "[flow]",
        detail: "Runner retry & flow rules",
        insert_text: "\n[flow]\nonce_per_run = false\nretry_until = \"response.status == 200\"\nretry_interval_ms = 1000\nmax_retries = 3\n",
        description: "Controls collection runner retries and once-per-run behavior.",
    },
    ReferenceEntry {
        category: "Template Built-ins",
        label: "{{$uuid}}",
        detail: "Dynamic UUID v4",
        insert_text: "{{$uuid}}",
        description: "Expands to a fresh UUID v4 on each request execution.",
    },
    ReferenceEntry {
        category: "Template Built-ins",
        label: "{{$timestamp}}",
        detail: "Unix epoch seconds",
        insert_text: "{{$timestamp}}",
        description: "Expands to the current Unix timestamp.",
    },
    ReferenceEntry {
        category: "Template Built-ins",
        label: "{{$randomInt}}",
        detail: "Random integer",
        insert_text: "{{$randomInt}}",
        description: "Expands to a random integer.",
    },
    ReferenceEntry {
        category: "Template Built-ins",
        label: "{{$randomEmail}}",
        detail: "Random email address",
        insert_text: "{{$randomEmail}}",
        description: "Expands to a generated email address.",
    },
];

/// LSP `CompletionProvider` backing Aura's JavaScript and `.aura` Spec editors.
pub struct AuraCompletionProvider {
    language: CompletionLanguage,
    context: Rc<RefCell<EditorContextData>>,
}

impl AuraCompletionProvider {
    pub fn new(language: CompletionLanguage, context: Rc<RefCell<EditorContextData>>) -> Self {
        Self { language, context }
    }
}

impl CompletionProvider for AuraCompletionProvider {
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        _trigger: CompletionContext,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<Result<CompletionResponse>> {
        let prefix = extract_completion_prefix(text, offset);
        let ctx = self.context.borrow();
        let items = collect_completion_items(self.language, &prefix, &ctx);
        Task::ready(Ok(CompletionResponse::Array(items)))
    }

    fn is_completion_trigger(&self, _offset: usize, new_text: &str, _cx: &mut App) -> bool {
        new_text.chars().any(|c| {
            c.is_alphanumeric() || matches!(c, '.' | '_' | '{' | '$' | '[' | '"' | '\'')
        })
    }
}

/// Extracts the token prefix immediately before `offset` (including `.` and `{{`).
pub fn extract_completion_prefix(text: &Rope, offset: usize) -> String {
    let clamped = offset.min(text.len());
    let mut start = clamped;
    for ch in text.chars_at(clamped).reversed() {
        if ch.is_alphanumeric() || matches!(ch, '_' | '.' | '{' | '$' | '[' | '-') {
            start -= ch.len_utf8();
        } else {
            break;
        }
    }
    text.slice(start..clamped).to_string()
}

/// Pure completion builder used by `AuraCompletionProvider` and unit tests.
pub fn collect_completion_items(
    language: CompletionLanguage,
    prefix: &str,
    ctx: &EditorContextData,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let needle = prefix.to_ascii_lowercase();

    match language {
        CompletionLanguage::JavaScript { include_response } => {
            for entry in JS_REFERENCE_ENTRIES {
                if !include_response && entry.category == "Response Object" {
                    continue;
                }
                if matches_prefix(entry.label, entry.insert_text, &needle) {
                    items.push(make_item(
                        entry.label,
                        entry.detail,
                        entry.insert_text,
                        entry.description,
                        if entry.insert_text.contains('(') {
                            CompletionItemKind::FUNCTION
                        } else {
                            CompletionItemKind::PROPERTY
                        },
                    ));
                }
            }

            // Active request/workspace variables
            for (key, val, enabled) in &ctx.variables {
                if !enabled || key.trim().is_empty() {
                    continue;
                }
                let label = format!("aura.variables.get(\"{key}\")");
                if matches_prefix(&label, key, &needle) {
                    items.push(make_item(
                        &label,
                        &format!("Variable = {val}"),
                        &label,
                        "Active Aura variable in current scope.",
                        CompletionItemKind::VARIABLE,
                    ));
                }
            }

            // Active query parameters
            for (key, val, enabled) in &ctx.query_params {
                if !enabled || key.trim().is_empty() {
                    continue;
                }
                let label = format!("param:{key}");
                if matches_prefix(&label, key, &needle) {
                    items.push(make_item(
                        &label,
                        &format!("Query Param = {val}"),
                        key,
                        "Query parameter defined on this request.",
                        CompletionItemKind::FIELD,
                    ));
                }
            }
        }
        CompletionLanguage::TomlSpec => {
            for entry in SPEC_REFERENCE_ENTRIES {
                if matches_prefix(entry.label, entry.insert_text, &needle) {
                    items.push(make_item(
                        entry.label,
                        entry.detail,
                        entry.insert_text,
                        entry.description,
                        if entry.label.starts_with('[') {
                            CompletionItemKind::MODULE
                        } else {
                            CompletionItemKind::SNIPPET
                        },
                    ));
                }
            }

            for (key, val, enabled) in &ctx.variables {
                if !enabled || key.trim().is_empty() {
                    continue;
                }
                let token = format!("{{{{{key}}}}}");
                if matches_prefix(&token, key, &needle) {
                    items.push(make_item(
                        &token,
                        &format!("Variable = {val}"),
                        &token,
                        "Template variable from active request/workspace.",
                        CompletionItemKind::VARIABLE,
                    ));
                }
            }
        }
    }

    items
}

fn matches_prefix(label: &str, alt: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let l = label.to_ascii_lowercase();
    let a = alt.to_ascii_lowercase();
    l.contains(needle) || a.contains(needle)
}

fn make_item(
    label: &str,
    detail: &str,
    insert_text: &str,
    doc: &str,
    kind: CompletionItemKind,
) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        detail: Some(detail.to_string()),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: doc.to_string(),
        })),
        insert_text: Some(insert_text.to_string()),
        ..Default::default()
    }
}
