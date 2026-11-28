# Aura User Guide

Welcome to **Aura** — a high-performance, offline-first desktop API client and load-testing tool built with Rust, GPUI, Tokio, and `deno_core`.

This guide walks through everything you need to use Aura day-to-day: managing workspaces, sending requests, configuring environments and secrets, writing **pre-request** and **post-response** scripts, creating **no-code and JavaScript tests**, chaining requests, and running load tests.

---

## Table of Contents

1. [Workspaces & Collections](#1-workspaces--collections)
2. [Creating & Sending Requests](#2-creating--sending-requests)
3. [Environments, Variables & Git-Safe Secrets](#3-environments-variables--git-safe-secrets)
4. [Pre-Request Scripts](#4-pre-request-scripts)
5. [Post-Response Scripts](#5-post-response-scripts)
6. [Creating Your Own Tests](#6-creating-your-own-tests)
7. [Zero-JS Native Extractors](#7-zero-js-native-extractors)
8. [Collection Runner & Request Chaining](#8-collection-runner--request-chaining)
9. [Load Testing (UI & `aura load` CLI)](#9-load-testing-ui--aura-load-cli)
10. [Tools Menu & Network Utilities](#10-tools-menu--network-utilities)

---

## 1. Workspaces & Collections

Aura stores every request as a human-readable, Git-friendly `.aura` TOML file inside a normal folder on your disk.

- **Open a Workspace**: Click **Workspace** in the top toolbar (or **Workspace → Open Workspace Folder...**). Aura indexes all `.aura` files and `environment/*.toml` profiles in that folder and automatically watches for external filesystem changes.
- **Create Requests & Folders**: Use the sidebar context menu (right-click) or **File → New Request (`Ctrl+N`)** to create requests or nested folders.
- **Import Existing Collections**: Use **Workspace → Import WSDL...** or import Postman Collections (v2.1), Insomnia exports, OpenAPI v2/v3 specs, and raw cURL commands.
- **Git Integration**: Click **Git** in the toolbar (or **Workspace → Git Status & Sync**) to inspect `git status`, view diffs, commit, switch branches, and push/pull without leaving Aura.

---

## 2. Creating & Sending Requests

1. **Method & URL**: Choose the HTTP method (`GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, `OPTIONS`) and enter your URL (e.g., `{{base_url}}/api/users`).
2. **Protocol Support**: Aura supports **HTTP/REST**, **GraphQL** (dedicated query & variables editor), **WebSockets (`ws://` / `wss://`)**, **Server-Sent Events (SSE)**, **gRPC**, and **SOAP/WSDL**.
3. **Request Tabs**:
   - **Params**: Query parameters (prefix/suffix with `~` or uncheck to disable without deleting).
   - **Headers**: Custom HTTP headers.
   - **Auth**: Bearer, Basic, API Key, AWS SigV4, OAuth2, Digest, NTLM, OAuth1, Hawk, Akamai EdgeGrid, WSSE, or Inherit from collection.
   - **Body**: JSON, XML, Text, Raw, Form URL-Encoded, Multipart (relative file paths), Binary, or GraphQL.
   - **Scripts**: Pre-Request and Post-Response JavaScript hooks.
   - **Tests**: No-Code Declarative Assertions (`[assert]`) and JavaScript BDD Tests (`[scripts.tests]`).
   - **Extractors**: Zero-JS native response extractors (`[[extractors]]`).
   - **Spec**: Edit the raw `.aura` TOML file directly with live bidirectional sync.
4. **Send & Inspect**: Press **Send** (or `Ctrl+Enter`). Inspect responses in **JSON**, **Table**, **Tree**, **Headers**, **Raw**, **Tests**, or **Visualize** tabs.

---

## 3. Environments, Variables & Git-Safe Secrets

### Variable Resolution Precedence

Variables use `{{variable_name}}` syntax in URLs, headers, query params, auth fields, and bodies. Aura resolves them in this order:

1. **Request** (`[variables]` in the `.aura` file or set via `aura.variables.set(...)`)
2. **Active Environment** (`environment/<name>.toml`)
3. **Collection** (collection-level variables)
4. **Built-in Dynamic Variables**: `{{$uuid}}`, `{{$timestamp}}`, `{{$randomInt}}`, `{{$randomEmail}}`, `{{$faker.*}}`
5. **OS Environment Variables**: `{{$MY_ENV_VAR}}`

### Creating & Switching Environments

- Use the **Environment** dropdown on the right side of the top toolbar to switch profiles (e.g. `dev`, `staging`, `prod`) or click **New Environment...** (`Workspace → New Environment...`).
- Toggle any variable row between **Plain** and **Secret** in the Environment Editor.
- Secret values are stored in your **OS Keychain / Credential Manager** (or resolved via `vault://`, `aws-sm://`, `keepass://`, `keyring://` URIs configured in **Tools → Security Vaults & mTLS**), so plaintext secrets are never committed to Git.

---

## 4. Pre-Request Scripts

Pre-request scripts run in a sandboxed `deno_core` V8 JavaScript isolate **before** the HTTP request is sent. Use them to compute dynamic values, generate fake test data, or fetch an auth token.

Open a request → **Scripts** tab → **Pre-Request**:

```javascript
// 1. Generate dynamic fake data with built-in faker
const email = faker.internet.email();
aura.variables.set("user_email", email);
aura.variables.set("request_nonce", faker.datatype.uuid());

// 2. Fetch an OAuth token asynchronously before the main request runs
const tokenRes = await fetch("https://postman-echo.com/get?token=demo-123");
if (tokenRes.ok) {
    const data = await tokenRes.json();
    aura.variables.set("auth_token", data.args.token);
}

console.log("Prepared request for:", email);
```

Then reference `{{user_email}}`, `{{request_nonce}}`, and `{{auth_token}}` in your request URL, headers, or JSON body.

---

## 5. Post-Response Scripts

Post-response scripts run **after** the HTTP response arrives. Use them to inspect the response, save extracted values into variables for subsequent requests, send auxiliary webhooks, or control Collection Runner chaining.

Open a request → **Scripts** tab → **Post-Response**:

```javascript
console.log("Status:", response.status, "Duration:", response.duration_ms, "ms");

if (response.status === 200) {
    const body = response.json();
    if (body && body.json && body.json.email) {
        aura.variables.set("last_created_email", body.json.email);
    }
}

// Optional: send an auxiliary webhook with aura.sendRequest()
await aura.sendRequest({
    url: "https://postman-echo.com/post",
    method: "POST",
    headers: { "content-type": "application/json" },
    body: { event: "request_completed", status: response.status }
});
```

---

## 6. Creating Your Own Tests

Aura gives you two ways to write tests — and you can combine both on the same request.

### Option A: No-Code Declarative Assertions (`[assert]`)

1. Open the **Tests** tab on any request.
2. Under **NO-CODE ASSERTIONS (`[assert]`)**, click **Add Rule** or click a quick preset chip (`Status 200`, `body.id isNumber`, `Time < 500ms`).
3. Set the target (e.g. `res.status`, `res.duration_ms`, `res.headers.content-type`, `res.body.id`), choose an operator (`==`, `!=`, `<`, `<=`, `>`, `>=`, `contains`, `matches`, `isNumber`, `isArray`, `isDefined`, etc.), and enter the expected value.
4. Click **Send** (`Ctrl+Enter`). Results appear in the **Tests** response tab.

In `.aura` TOML format:

```toml
[assert]
res.status = "== 200"
res.duration_ms = "< 2000"
res.headers.content-type = "contains application/json"
res.body.json.email = "isString"
~res.body.debug_flag = "== true"   # Prefix with ~ to disable temporarily
```

### Option B: JavaScript BDD Tests (`describe` / `it` / `aura.test` / `expect`)

In the **Tests** tab (JavaScript editor) or `[scripts.tests]` in `.aura`:

```javascript
describe("User Creation API", () => {
    it("returns 200 OK within 2 seconds", () => {
        expect(response.status).toBe(200);
        expect(response.duration_ms).toBeLessThan(2000);
    });

    it("echoes back the generated email", () => {
        const data = response.json();
        expect(data.json).toBeDefined();
        expect(data.json.email).toContain("@");
    });
});

// Postman-compatible syntax also works:
aura.test("Content-Type header is JSON", () => {
    expect(response.headers["content-type"]).toContain("application/json");
});
```

See [Scripting, Assertions & Chaining](scripting_and_chaining.md) for the full `expect(...)` matcher list and [docs/examples/rest_crud/02_complete_scripts_and_tests.aura](examples/rest_crud/02_complete_scripts_and_tests.aura) for a complete runnable example.

---

## 7. Zero-JS Native Extractors

For high-throughput load testing where you want to correlate values without spinning up V8, add **Extractors** in the **Extractors** tab (`[extractors]` in `.aura`):

```toml
[extractors]
created_item_id = "jsonpath:$.id"
~disabled_cookie = "cookie:session_id"
```

Supported prefixes: `jsonpath:`, `regex:`, `regex-header:`, `header:`, and `cookie:`.

---

## 8. Collection Runner & Request Chaining

Use `aura.setNextRequest(nameOrNull)` in a post-response or test script to branch dynamically between requests in a collection run:

```javascript
if (response.status === 200) {
    const data = response.json();
    aura.variables.set("auth_token", data.token);
    aura.setNextRequest("02 Fetch User Profile");
} else {
    aura.setNextRequest("03 Fallback Auth Error");
}
// Stop the current iteration cleanly: aura.setNextRequest(null);
```

See the runnable chaining example in [docs/examples/chaining/](examples/chaining/).

---

## 9. Load Testing (UI & `aura load` CLI)

- **UI Load Tester**: Open **Tools → Load Tester** from the top menu to configure Virtual Users (VUs), iterations or duration, ramp-up/ramp-down, multi-stage spike profiles, target RPS, think-time, CSV/JSON data feeders, and SLA thresholds.
- **Headless CLI (`aura load`)**: Run load tests in CI/CD pipelines:
  ```bash
  aura load ./my-collection --env ./environment/staging.toml --vus 10 --duration 30 --sla-p95 250 --sla-error 1.0 --export-html ./report.html
  ```
  Run `aura load --help` or see [Reference Guide](reference.md#5-aura-load-cli-reference) for all flags and exit codes.

---

## 10. Tools Menu & Network Utilities

All secondary tools are organized under the **Tools** menu in the top menu bar (and searchable via Command Palette `Ctrl+K`):

- **Cookie Jar Manager**: Inspect, edit, and clear persisted cookies.
- **Global Proxy Settings**: Route requests through an HTTP, HTTPS, or SOCKS5 proxy.
- **Capture Proxy & Interceptor**: Start a local MITM capture proxy to inspect and intercept traffic.
- **Security Vaults & mTLS**: Configure HashiCorp Vault, AWS Secrets Manager, KeePass (`.kdbx`), OS Keychain entries, and mTLS client certificates.
- **Load Tester**: Run multi-VU load tests and inspect live metrics.
- **Settings & Auto-Update**: Configure auto-save, keybinding presets (VS Code, IntelliJ, Sublime), and GitHub Releases update checks.

When **Global Proxy**, **Capture Proxy**, or **Load Tester** is active, a live status pill appears in the top toolbar for one-click access.
