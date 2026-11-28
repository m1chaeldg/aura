# Aura Reference Guide

Complete reference for the Aura desktop UI, keyboard shortcuts, `.aura` TOML format, JavaScript scripting API, native extractors, and `aura load` CLI.

> **New to Aura?** Start with the [User Guide](user_guide.md) for step-by-step walkthroughs on using Aura, writing pre/post scripts, and creating your own tests.

---

## 1. Desktop UI & Toolbar

Aura uses a focused, two-pane workspace:

- **Top Menu Bar**: `File`, `Workspace`, `View`, `Tools`, `AI`, `Help`
  - **`Tools` Menu**: Cookie Jar Manager, Global Proxy Settings, Capture Proxy & Interceptor, Security Vaults & mTLS, Load Tester, Settings & Auto-Update.
  - **`Help` Menu**: Help & Quick Reference, Documentation (GitHub), GitHub Repository, Check for Updates, About Aura & Version.
- **Top Toolbar (Primary Actions)**:
  - **Sidebar Toggle** & **Aura** badge
  - **Search (`Ctrl+K`)**: Opens the Command Palette to jump to requests or run any command.
  - **Layout Toggle**: Switches between **Side-by-Side** and **Stacked** split views (`Ctrl+R` resets 50/50).
  - **Workspace**: Opens a local collection folder.
  - **Git**: Shows current branch and uncommitted change count; opens the built-in Git modal.
  - **Live Status Pills** (shown only when active): **Proxy**, **Capture LIVE**, and **Load Test LIVE**.
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
| `Ctrl+\` | Toggle Side-by-Side / Stacked layout |
| `Ctrl+Shift+L` | Toggle Diagnostic Log Viewer |

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
- `[body]` / `[body.json]`: Tagged by `type`: `None`, `Json`, `Xml`, `Text`, `Raw`, `Form`, `FormUrlEncoded`, `Multipart`, `Binary`, `GraphQL`.
- `[flow]`: Runner flow controls (`once_per_run`, `weight_percent`, `loop_count`, `condition_if`, `loop_while`, `max_loops`, `delay_ms`, `jitter_min_ms`, `jitter_max_ms`, `gaussian_mean_ms`, `gaussian_stddev_ms`, `max_duration_ms`, `min_size_bytes`, `max_size_bytes`).
- `[assert]`: Declarative assertions (`res.status = "== 200"`, `res.body.id = "isNumber"`, `res.duration_ms = "< 500"`).
- `[extractors]`: Zero-JS native extractors (`var = "jsonpath:$.id"`, `regex:`, `regex-header:`, `header:`, `cookie:`).
- `[scripts.pre-request]`, `[scripts.post-response]`, `[scripts.tests]` (or `[scripts]`): JavaScript hooks & BDD test scripts.

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

- **Variables**: `aura.variables.get(key)`, `aura.variables.set(key, val)`, `pm.variables.get/set`, `pm.environment.get/set`
- **Response** (Post-response & Tests): `response.status` (or `code`), `response.duration_ms` (or `responseTime`), `response.body`, `response.headers`, `response.text()`, `response.json()` (also on `aura.response` and `pm.response`)
- **Assertions**: `describe(suite, fn)`, `it(name, fn)`, `test(name, fn)`, `aura.test(name, fn)`, `expect(val).toBe(...)` / `.toEqual(...)` / `.to.equal(...)`
- **HTTP & Chaining**: `await fetch(url, init)`, `await aura.sendRequest(req, cb)`, `aura.setNextRequest(nameOrNull)`
- **Fake Data**: `faker.internet.email()`, `faker.person.fullName()`, `faker.datatype.uuid()`, `faker.number.int({ min, max })`, etc.
- **Logging**: `console.log(...)`, `console.warn(...)`, `console.error(...)`

See [scripting_and_chaining.md](./scripting_and_chaining.md) for detailed examples and matcher reference.

---

## 5. `aura load` CLI Reference

Run headless collection and load tests in CI/CD:

```bash
aura load <collection_dir_or_file.aura> [OPTIONS]
```

### Options

| Flag | Description |
|---|---|
| `-c, --collection <path>` | Target `.aura` request file or collection directory |
| `-w, --workspace <dir>` | Workspace root directory (uses `workspace.json` order & `.aura/load-profiles`) |
| `-p, --profile <name>` | Load saved workspace profile from `.aura/load-profiles/<name>.json` |
| `-e, --env <path.toml>` | Environment TOML profile file |
| `-u, --vus, --concurrency <N>` | Virtual user concurrency (default: `2`) |
| `-n, --iterations <N>` | Total iterations for fixed mode (default: `10`) |
| `-d, --duration <secs>` | Time-boxed soak/stress duration in seconds |
| `--ramp-up <secs>` | Ramp-up period in seconds |
| `--ramp-down <secs>` | Ramp-down period in seconds |
| `--stages <v:s,v:s,...>` | Multi-stage load schedule (e.g. `10:30,100:60,10:30`) |
| `--target-rps <rps>` | Constant throughput shaping cap (requests/sec) |
| `--think-uniform <a:b>` | Uniform random think-time range in ms (e.g. `200:800`) |
| `--think-gaussian <m:s>` | Gaussian think-time mean:stddev in ms (e.g. `400:100`) |
| `--sync-barrier <n[:ms]>` | Synchronizing Rendezvous Barrier group size & optional timeout ms |
| `--weighted` | Distribute requests using `[flow] weight_percent` weights |
| `--data <file>` | CSV or JSON data feeder file |
| `--sharing all\|vu` | Data feeder sharing mode (default: `all`) |
| `--eof recycle\|stop` | Data feeder EOF behavior (default: `recycle`) |
| `--max-duration <ms>` | Per-sample max duration assertion (ms) |
| `--min-size <bytes>` | Per-sample min response size (bytes) |
| `--max-size <bytes>` | Per-sample max response size (bytes) |
| `--sla-p95 <ms>` | Global CI SLA gate for p95 latency (ms) |
| `--sla-p99 <ms>` | Global CI SLA gate for p99 latency (ms) |
| `--sla-error <pct>` | Global CI SLA gate for max error rate (%) |
| `--export-csv <path>` | Export raw JTL-style CSV samples |
| `--export-json <path>` | Export full JSON report + time series |
| `--export-html <path>` | Generate standalone HTML summary report |
| `-q, --quiet` | Suppress per-sample terminal output |
| `-h, --help` | Show CLI help message |

### Exit Codes

- `0`: All requests, assertions, and SLA thresholds passed.
- `1`: One or more assertions or SLA thresholds failed.
- `2`: Invalid CLI arguments or workspace/file load error.
- `3`: Network or transport execution error.
