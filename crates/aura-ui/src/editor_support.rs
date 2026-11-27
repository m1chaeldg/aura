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
    GraphQL,
}

/// Live request/tab context fed into `AuraCompletionProvider` so autocomplete
/// suggests the current tab's variables, query parameters, and headers.
#[derive(Debug, Clone, Default)]
pub struct EditorContextData {
    pub variables: Vec<(String, String, bool)>,
    pub query_params: Vec<(String, String, bool)>,
    pub headers: Vec<(String, String, bool)>,
    pub graphql_schema: Option<aura_core::graphql::GraphQLSchema>,
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
    // Aura Sandbox & Postman Compatibility
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
        description: "Registers a test case (alias for it() / test()).",
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
        label: "fetch",
        detail: "(url: string, init?: RequestInit) => Promise<Response>",
        insert_text: "const res = await fetch(\"https://postman-echo.com/get\");\nconst data = await res.json();\n",
        description: "Performs an async HTTP/HTTPS request inside pre-request, post-response, or test scripts.",
    },
    ReferenceEntry {
        category: "Aura Sandbox",
        label: "aura.sendRequest",
        detail: "(req: string | RequestSpec, cb?: (err, res) => void) => Promise<Response>",
        insert_text: "const res = await aura.sendRequest({ url: \"https://postman-echo.com/get\", method: \"GET\" });\n",
        description: "Sends an auxiliary HTTP request asynchronously (Postman pm.sendRequest compatible).",
    },
    ReferenceEntry {
        category: "Aura Sandbox",
        label: "aura.setNextRequest",
        detail: "(nameOrId: string | null) => void",
        insert_text: "aura.setNextRequest(\"Next Request Name\");\n",
        description: "Controls Collection Runner chaining order; pass null to halt the chain cleanly.",
    },
    ReferenceEntry {
        category: "Aura Sandbox",
        label: "aura.execution.setNextRequest",
        detail: "(nameOrId: string | null) => void",
        insert_text: "aura.execution.setNextRequest(null);\n",
        description: "Alias for aura.setNextRequest matching Postman execution API.",
    },
    ReferenceEntry {
        category: "Aura Sandbox",
        label: "pm.* (Postman Alias)",
        detail: "pm.variables, pm.environment, pm.test, pm.expect, pm.sendRequest, pm.response",
        insert_text: "pm.test(\"Status is 200\", () => {\n    pm.expect(pm.response.code).to.equal(200);\n});\n",
        description: "Drop-in Postman pm.* compatibility aliases for variables, tests, and responses.",
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
        detail: "number (alias: response.code)",
        insert_text: "response.status",
        description: "HTTP status code returned by the server (e.g. 200, 404).",
    },
    ReferenceEntry {
        category: "Response Object",
        label: "response.duration_ms",
        detail: "number (alias: response.responseTime)",
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
        label: "response.text()",
        detail: "() => string",
        insert_text: "response.text()",
        description: "Returns the raw response body string.",
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
        description: "Groups related test assertions into a named suite (supports nested suites).",
    },
    ReferenceEntry {
        category: "Jasmine / Chai BDD",
        label: "it / test",
        detail: "(testName: string, fn: () => void | Promise<void>) => Promise",
        insert_text: "test(\"passes\", () => {\n    expect(response.status).toBe(200);\n});\n",
        description: "Defines an individual sync or async test case (test() is an alias for it()).",
    },
    ReferenceEntry {
        category: "Jasmine / Chai BDD",
        label: "expect(...).not",
        detail: "Negated Expectation chain",
        insert_text: "expect(response.status).not.toBe(500);",
        description: "Negates any Jasmine matcher (e.g., .not.toBe, .not.toContain).",
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
        label: "expect(...).toMatch",
        detail: "(pattern: RegExp | string) => void",
        insert_text: "expect(response.body).toMatch(/\"id\"\\s*:/);",
        description: "Asserts string matches a RegExp or pattern.",
    },
    ReferenceEntry {
        category: "Jasmine / Chai BDD",
        label: "expect(...).toBeDefined / toBeUndefined / toBeNull",
        detail: "() => void",
        insert_text: "expect(data.id).toBeDefined();",
        description: "Checks defined, undefined, or null identity.",
    },
    ReferenceEntry {
        category: "Jasmine / Chai BDD",
        label: "expect(...).toBeTruthy / toBeFalsy",
        detail: "() => void",
        insert_text: "expect(data.ok).toBeTruthy();",
        description: "Asserts JS truthiness or falsiness.",
    },
    ReferenceEntry {
        category: "Jasmine / Chai BDD",
        label: "expect(...).toBeGreaterThan / toBeLessThan",
        detail: "(n: number) => void (also >= / <= variants)",
        insert_text: "expect(response.duration_ms).toBeLessThan(500);",
        description: "Numeric comparison: toBeGreaterThan, toBeGreaterThanOrEqual, toBeLessThan, toBeLessThanOrEqual.",
    },
    ReferenceEntry {
        category: "Jasmine / Chai BDD",
        label: "expect(...).toThrow",
        detail: "() => void",
        insert_text: "expect(() => JSON.parse(\"bad\")).toThrow();",
        description: "Asserts that the target function throws an error.",
    },
    ReferenceEntry {
        category: "Jasmine / Chai BDD",
        label: "expect(...).to.equal / to.deep.equal",
        detail: "(expected: any) => void",
        insert_text: "expect(response.status).to.equal(200);",
        description: "Chai BDD strict (===) or deep structural equality assertion.",
    },
    ReferenceEntry {
        category: "Jasmine / Chai BDD",
        label: "expect(...).to.be.a / true / false",
        detail: "(type: string) => void",
        insert_text: "expect(data.items).to.be.a(\"object\");",
        description: "Chai BDD typeof check or boolean literal check (.to.be.true / .to.be.false).",
    },
    ReferenceEntry {
        category: "Jasmine / Chai BDD",
        label: "expect(...).to.have.property",
        detail: "(prop: string) => void",
        insert_text: "expect(data).to.have.property(\"id\");",
        description: "Chai BDD assertion checking own property existence.",
    },
    // Faker & Offline Data (all namespaces from prelude.js)
    ReferenceEntry {
        category: "Faker (Offline)",
        label: "faker.person.fullName() / firstName / lastName / jobTitle",
        detail: "() => string (also faker.name.*)",
        insert_text: "faker.person.fullName()",
        description: "Generates person names and job titles offline.",
    },
    ReferenceEntry {
        category: "Faker (Offline)",
        label: "faker.internet.email() / exampleEmail / userName",
        detail: "() => string",
        insert_text: "faker.internet.email()",
        description: "Generates offline email addresses and usernames.",
    },
    ReferenceEntry {
        category: "Faker (Offline)",
        label: "faker.internet.password(len?) / url / ip / ipv6 / color",
        detail: "() => string",
        insert_text: "faker.internet.password(16)",
        description: "Generates passwords, URLs, IPv4/IPv6 addresses, and hex colors.",
    },
    ReferenceEntry {
        category: "Faker (Offline)",
        label: "faker.string.uuid() / alphanumeric(len?) / sample(len?)",
        detail: "() => string (also datatype.uuid, random.uuid)",
        insert_text: "faker.string.uuid()",
        description: "Generates UUID v4, alphanumeric tokens, and word samples.",
    },
    ReferenceEntry {
        category: "Faker (Offline)",
        label: "faker.number.int(opts?) / float(opts?)",
        detail: "(opts?: { min?, max?, precision? }) => number",
        insert_text: "faker.number.int({ min: 1, max: 100 })",
        description: "Generates random integers or rounded floats in [min, max].",
    },
    ReferenceEntry {
        category: "Faker (Offline)",
        label: "faker.datatype.boolean() / random.boolean()",
        detail: "() => boolean",
        insert_text: "faker.datatype.boolean()",
        description: "Generates a random boolean value.",
    },
    ReferenceEntry {
        category: "Faker (Offline)",
        label: "faker.location.city() / country / countryCode / streetAddress",
        detail: "() => string (also faker.address.*)",
        insert_text: "faker.location.city()",
        description: "Generates cities, countries, country codes, and street addresses.",
    },
    ReferenceEntry {
        category: "Faker (Offline)",
        label: "faker.company.name() / commerce.price() / productName()",
        detail: "() => string",
        insert_text: "faker.company.name()",
        description: "Generates company names, prices, and product names.",
    },
    ReferenceEntry {
        category: "Faker (Offline)",
        label: "faker.lorem.word() / words(count?) / sentence()",
        detail: "() => string",
        insert_text: "faker.lorem.sentence()",
        description: "Generates placeholder words and sentences offline.",
    },
    ReferenceEntry {
        category: "Faker (Offline)",
        label: "faker.date.recent() / past() / future() / now()",
        detail: "() => Date | number",
        insert_text: "faker.date.recent().toISOString()",
        description: "Generates recent, past, or future Date objects, or epoch ms via now().",
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
        label: "[variables]",
        detail: "Request-scoped variables",
        insert_text: "\n[variables]\nbase_url = \"https://postman-echo.com\"\n",
        description: "Request-level variables (highest precedence in {{var}} cascade).",
    },
    ReferenceEntry {
        category: "Spec Sections",
        label: "[auth]",
        detail: "Authentication configuration",
        insert_text: "\n[auth]\ntype = \"bearer\"\ntoken = \"{{api_token}}\"\n",
        description: "Supports bearer, basic, apikey, awssigv4, oauth2, oauth1, digest, hawk, ntlm, wsse.",
    },
    ReferenceEntry {
        category: "Spec Sections",
        label: "[body]",
        detail: "Request payload",
        insert_text: "\n[body]\ntype = \"json\"\ndata = '''\n{\n  \"id\": \"{{$uuid}}\"\n}\n'''\n",
        description: "Supports json, text, xml, form, multipart, graphql, and binary.",
    },
    ReferenceEntry {
        category: "Spec Sections",
        label: "[scripts]",
        detail: "Pre-request, post-response, and test JS",
        insert_text: "\n[scripts]\npre_request = \"\"\"\naura.variables.set(\"ts\", Date.now());\n\"\"\"\ntests = \"\"\"\naura.test(\"200 OK\", () => aura.expect(response.status).toBe(200));\n\"\"\"\n",
        description: "Sandboxed deno_core JavaScript hooks executed around the request.",
    },
    ReferenceEntry {
        category: "Spec Sections",
        label: "[flow]",
        detail: "Runner retry & flow rules",
        insert_text: "\n[flow]\nonce_per_run = false\nretry_until = \"response.status == 200\"\nretry_interval_ms = 1000\nmax_retries = 3\n",
        description: "Controls collection runner retries, condition_if, and once-per-run behavior.",
    },
    ReferenceEntry {
        category: "Spec Sections",
        label: "[assert]",
        detail: "No-code declarative assertions",
        insert_text: "\n[assert]\nres.status: eq 200\nres.body.id: isNumber\nres.duration_ms: lt 1000\n",
        description: "Operators: eq, neq, gt, gte, lt, lte, contains, notContains, startsWith, endsWith, matches, in, notIn, isDefined, isNull, isEmpty, isNumber, isString, isArray, lengthEq.",
    },
    ReferenceEntry {
        category: "Spec Sections",
        label: "[extract]",
        detail: "Zero-JS native extractors",
        insert_text: "\n[extract]\ntoken = \"jsonpath:$.data.token\"\nreq_id = \"header:x-request-id\"\n",
        description: "High-speed native extractors: jsonpath:, regex:, regex-header:, header:, cookie:.",
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
        label: "{{$guid}}",
        detail: "Dynamic GUID / UUID v4",
        insert_text: "{{$guid}}",
        description: "Alias for {{$uuid}} expanding to a fresh UUID v4.",
    },
    ReferenceEntry {
        category: "Template Built-ins",
        label: "{{$timestamp}} / {{$isoTimestamp}}",
        detail: "Unix epoch / ISO-8601",
        insert_text: "{{$timestamp}}",
        description: "Expands to current Unix epoch seconds or ISO-8601 timestamp.",
    },
    ReferenceEntry {
        category: "Template Built-ins",
        label: "{{$randomInt}} / {{$randomFloat}} / {{$randomBool}}",
        detail: "Random primitives",
        insert_text: "{{$randomInt}}",
        description: "Expands to random integer, float, or boolean.",
    },
    ReferenceEntry {
        category: "Template Built-ins",
        label: "{{$randomEmail}} / {{$randomUserName}} / {{$randomFullName}}",
        detail: "Random identity fields",
        insert_text: "{{$randomEmail}}",
        description: "Expands to offline generated email, username, or full name.",
    },
    ReferenceEntry {
        category: "Template Built-ins",
        label: "{{$faker.person.fullName}} / {{$faker.internet.email}}",
        detail: "Inline Faker templates",
        insert_text: "{{$faker.internet.email}}",
        description: "Supports $faker.person.*, $faker.internet.*, $faker.location.*, $faker.company.*, $faker.commerce.*, $faker.lorem.*, $faker.number.*, $faker.string.*",
    },
    ReferenceEntry {
        category: "Template Built-ins",
        label: "vault://path#key / aws-sm://secret#key / keychain://svc/acct",
        detail: "Secret Vault URI",
        insert_text: "vault://secret/data/prod#api_key",
        description: "Resolves secrets at runtime from HashiCorp Vault, AWS Secrets Manager, or OS Keychain.",
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
        CompletionLanguage::GraphQL => {
            let mut full_text = String::new();
            let gql_items =
                aura_core::graphql::graphql_completions(prefix, ctx.graphql_schema.as_ref());
            for item in gql_items {
                if matches_prefix(&item.label, &item.insert_text, &needle) {
                    full_text.clear();
                    full_text.push_str(&item.detail);
                    items.push(make_item(
                        &item.label,
                        &full_text,
                        &item.insert_text,
                        "GraphQL completion",
                        if item.kind == "Keyword" {
                            CompletionItemKind::KEYWORD
                        } else {
                            CompletionItemKind::FIELD
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
