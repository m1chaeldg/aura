# Aura Reference Guide

Complete reference for the Aura desktop UI, keyboard shortcuts, `.aura` TOML format, JavaScript scripting API, native extractors, and `aura load` CLI.

---

## 1. Desktop UI & Simplified Toolbar

Aura uses a focused, two-pane workspace:

- **Top Menu Bar**: `File`, `Workspace`, `View`, `Tools`, `AI`, `Help`
- **Top Toolbar (Primary Actions)**:
  - **Sidebar Toggle** & **Aura** badge
  - **Search (`Ctrl+K`)**: Opens the Command Palette to jump to requests or run any command.
  - **Layout Toggle**: Switches between **Side-by-Side** and **Stacked** split views (`Ctrl+R` resets 50/50).
  - **Workspace**: Opens a local collection folder.
  - **Git**: Shows current branch and uncommitted change count; opens the built-in Git modal.
  - **Tools**: Quick-opens the `Tools` menu (Cookie Jar, Global Proxy, Capture Proxy & Interceptor, Security Vaults & mTLS, Load Tester, Settings). When **Global Proxy**, **Capture Proxy**, or **Load Test** is active, a live status pill appears in the toolbar for one-click inspection.
  - **Environment Dropdown**: Switches active environment profiles or opens the Environment Editor.

---

## 2. Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl+Enter` | Send active request |
| `Ctrl+S` | Save active request to `.aura` file |
| `Ctrl+K` / `Ctrl+P` | Open Command Palette |
| `Ctrl+N` | Create new request |
| `Ctrl+W` | Close active tab |
| `Ctrl+R` | Reset split divider to 50/50 |

---

## 3. `.aura` TOML File Format Reference

Each `.aura` file represents one request and is safe to version-control with Git.

### Top-Level Fields

| Key | Type | Description |
|---|---|---|
| `version` | String | Format version (currently `"1.0"`). |
| `name` | String | Display name in sidebar and tabs. |
| `method` | String | `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, `OPTIONS`. |
| `url` | String | Target URL (supports `{{variable}}` templates). |
| `protocol` | String (optional) | `http` (default), `graphql`, `websocket`, `sse`, `grpc`, `soap`. |

### Sections

- `[settings]`: `inherit_auth`, `verify_ssl`, `follow_redirects`, `max_redirects`, `timeout_ms`.
- `[headers]` / `[params.query]` / `[variables]`: Key-value maps. Prefix or suffix a key with `~` (e.g. `~X-Debug = "1"`) to disable it without deleting.
- `[auth]`: Tagged by `type`: `inherit`, `none`, `bearer`, `basic`, `api_key`, `aws_sigv4`, `oauth2`, `digest`, `ntlm`, `oauth1`, `hawk`, `akamai_edgegrid`, `wsse`.
- `[body]`: Tagged by `type`: `None`, `Json`, `Xml`, `Text`, `Raw`, `Form`, `FormUrlEncoded`, `Multipart`, `Binary`, `GraphQL`.
- `[flow]`: Runner flow controls (`once_per_run`, `weight_percent`, `loop_count`, `condition_if`, `loop_while`, `max_loops`, `delay_ms`, `jitter_min_ms`, `jitter_max_ms`, `gaussian_mean_ms`, `gaussian_stddev_ms`, `max_duration_ms`, `min_size_bytes`, `max_size_bytes`).
- `[assert]`: Declarative assertions (`res.status = "== 200"`, `res.body.id = "isNumber"`, `res.duration_ms = "< 500"`).
- `[[extractors]]`: Zero-JS native extractors (`source`: `json_path`, `regex_body`, `regex_header`, `header`, `cookie`).
- `[scripts]`: `pre_request` and `post_response` JavaScript hooks; `tests` for BDD test scripts.

### Environment Profiles & Secrets (`environment/<name>.toml`)

Environment profiles live under `<workspace>/environment/<name>.toml` and support Git-safe secret references:

- `[variables]`: Plain key-value variables (`BASE_URL = "https://api.example.com"`). Prefix or suffix with `~` to disable.
- `[secrets]`: Secret variables whose plaintext values are never written to disk:
  - `API_KEY = "keyring"` — stored in the OS Keychain under `Aura:env-<env_name>:API_KEY`.
  - `VAULT_TOKEN = "vault://[profile@]secret/data/app#token"` — resolved from HashiCorp Vault KV v1/v2.
  - `DB_PASS = "aws-sm://[profile@]prod/db#password"` — resolved from AWS Secrets Manager.
  - `KP_PASS = "keepass://[profile@]Group/Entry#Password"` — resolved from a KeePass `.kdbx` database.
- Any `keyring://`, `vault://`, `aws-sm://`, or `keepass://` URI can also be used inline as `{{<uri>}}` in request URLs, headers, params, auth fields, or bodies. Resolved secrets are cached in memory (300s default TTL) and redacted from logs.

See [file_format.md](./file_format.md) and [examples/](./examples/) for complete examples.

---

## 4. JavaScript Scripting API (`deno_core` V8)

Both `aura.*` and Postman-compatible `pm.*` globals are available:

- **Variables**: `aura.variables.get/set`, `aura.environment.get/set`, `aura.setEnvVar(k, v)`, `aura.getSecret(k)`
- **Request/Response**: `aura.request.getBody()`, `aura.setHeader(k, v)`, `aura.response.status`, `aura.response.json()`, `response.headers`
- **Assertions**: `aura.test(name, fn)`, `describe(suite, fn)`, `it(name, fn)`, `expect(val).toBe(...)` / `.to.equal(...)`
- **HTTP & Chaining**: `await fetch(url, init)`, `await aura.sendRequest(req)`, `aura.setNextRequest(nameOrNull)`
- **Fake Data**: `faker.internet.email()`, `faker.person.fullName()`, `faker.string.uuid()`

See [scripting_and_chaining.md](./scripting_and_chaining.md) for detailed examples.

---

## 5. `aura load` CLI Reference

Run headless collection and load tests in CI/CD:

```bash
aura load <PATH> [OPTIONS]
```

### Common Options

| Flag | Description |
|---|---|
| `--env <NAME>` | Environment profile name to activate |
| `--vus <N>` | Virtual user concurrency (default: `1`) |
| `--iterations <N>` | Total iterations (fixed-iterations mode) |
| `--duration <SECS>` | Time-boxed duration in seconds |
| `--ramp-up <SECS>` | Ramp-up duration in seconds |
| `--ramp-down <SECS>` | Ramp-down duration in seconds |
| `--target-rps <RPS>` | Target requests-per-second rate limit |
| `--data <FILE>` | CSV or JSON data feeder file |
| `--max-p95-ms <MS>` | Fail SLA if p95 latency exceeds threshold |
| `--max-p99-ms <MS>` | Fail SLA if p99 latency exceeds threshold |
| `--max-error-rate <PCT>` | Fail SLA if error percentage exceeds threshold |
| `--report-html <FILE>` | Export standalone HTML report |
| `--report-json <FILE>` | Export JSON summary report |
| `--report-csv <FILE>` | Export JTL-compatible CSV sample log |

### Exit Codes

- `0`: All requests, assertions, and SLA thresholds passed.
- `1`: One or more assertions or SLA thresholds failed.
- `2`: Invalid CLI arguments or workspace/file load error.
- `3`: Network or transport execution error.
