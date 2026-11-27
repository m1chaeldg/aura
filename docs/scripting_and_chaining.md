# Scripting, Declarative Assertions, and Request Chaining

Aura combines Bruno-style plain-text `[assert]` rules, async JavaScript scripting powered by `deno_core`, and Postman/Bruno-compatible Collection Runner chaining (`aura.setNextRequest`).

---

## 1. Declarative / No-Code Assertions (`[assert]`)

Declarative assertions let you validate HTTP responses without writing JavaScript.

### Using the UI

1. Open any request and switch to the **Tests** tab.
2. In the **NO-CODE ASSERTIONS (`[assert]`)** section, click **Add Rule** or choose a preset chip (`Status 200`, `body.id isNumber`, `Time < 500ms`).
3. Enter the target expression (e.g., `res.body.user.email`), click the operator pill to cycle operators, and type the expected value (unary operators like `isNumber` hide the expected input).
4. Click **Run Tests** or **Send** — declarative rules run automatically and appear alongside JS BDD `it(...)` checks in the **Tests** response tab.

### `.aura` File Syntax

```toml
[assert]
res.status = "== 200"
res.duration_ms = "< 1500"
res.headers.content-type = "contains application/json"
res.body.id = "isNumber"
res.body.tags = "isArray"
~res.body.debug_flag = "== true"
```

Prefixing a rule with `~` disables it without deleting it.

### Supported Expressions & Operators

- **Targets**: `res.status`, `res.duration_ms`, `res.size_bytes`, `res.body.<nested.path[0]>`, `res.headers.<header-name>`
- **Comparisons**: `==` (`eq`), `!=` (`neq`), `>` (`gt`), `>=` (`gte`), `<` (`lt`), `<=` (`lte`)
- **Strings / Collections / Regex**: `contains`, `notContains`, `startsWith`, `endsWith`, `matches` (regex), `in` (`a, b, c`), `notIn`, `length`
- **Unary Checks**: `isDefined`, `isUndefined`, `isNull`, `isNotNull`, `isEmpty`, `isNotEmpty`, `isNumber`, `isString`, `isBoolean`, `isArray`, `isObject`

---

## 2. Async `fetch` and `aura.sendRequest` in Scripts

Both **Pre-request** and **Post-response** / **Test** scripts support async HTTP/HTTPS requests using standard `fetch` or `aura.sendRequest` (and Postman's `pm.sendRequest`).

### Example: Pre-request Token Fetch

```javascript
const tokenRes = await fetch("https://auth.example.com/oauth/token", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ grant_type: "client_credentials" })
});

const payload = await tokenRes.json();
if (payload.access_token) {
    aura.variables.set("access_token", payload.access_token);
}
```

### Example: Auxiliary Call with `aura.sendRequest`

```javascript
// Works with async/await OR a Node-style callback: (err, res) => { ... }
const check = await aura.sendRequest({
    url: "https://postman-echo.com/post",
    method: "POST",
    headers: { "content-type": "application/json" },
    body: { source: "post-response-hook", status: response.status }
});

console.log("Audit webhook status:", check.status);
```

---

## 3. Request Chaining (`aura.setNextRequest`)

Inside the **Collection Runner**, requests normally run in sequential order from top to bottom. Calling `aura.setNextRequest(...)` in a post-response or test script overrides which request executes next within the current iteration.

```
┌─────────────────┐  200 OK   ┌──────────────────────┐  done
│  01_login.aura  ├──────────►│ 02_fetch_profile.aura├──────► Stop (null)
└────────┬────────┘           └──────────────────────┘
         │ non-200 / missing token
         ▼
┌──────────────────────┐      Stop (null)
│03_fallback_error.aura├─────────────────► Done
└──────────────────────┘
```

### API Reference

| Call | Behavior |
|---|---|
| `aura.setNextRequest("02 Fetch User Profile")` | Jumps to the request whose `name` or `id` matches (case-insensitive). |
| `aura.setNextRequest(null)` | Stops the rest of the current Collection Runner iteration cleanly. |
| `aura.execution.setNextRequest(...)` | Official alias matching Postman's `pm.execution.setNextRequest(...)`. |
| `pm.setNextRequest(...)` | Postman compatibility alias. |

### Shared State Across Chained Steps

1. **Variables**: Any `aura.variables.set("token", ...)` executed during step N is merged into the Collection Runner's iteration context and is immediately available via `{{token}}` or `aura.variables.get("token")` in step N+1.
2. **Cookies**: Cookies returned via `Set-Cookie` headers are carried forward in the runner's per-iteration Cookie Jar across chained steps.
3. **Loop Safety**: Collection runs enforce a maximum chained step limit per iteration (`max(requests.len() * 10, 50)`) so accidental infinite `A -> B -> A` loops terminate gracefully with a clear log entry instead of hanging.

### Sample Chaining Collection

See the ready-to-use `.aura` sample files in [examples/chaining/](examples/chaining/):

1. [01_login.aura](examples/chaining/01_login.aura) — Pre-request `fetch(...)` health check + Login POST + conditional `aura.setNextRequest(...)` + `aura.variables.set("auth_token", ...)`
2. [02_fetch_profile.aura](examples/chaining/02_fetch_profile.aura) — Uses `{{auth_token}}`, runs `[assert]` + `aura.sendRequest(...)`, and halts cleanly with `aura.setNextRequest(null)`
3. [03_fallback_error.aura](examples/chaining/03_fallback_error.aura) — Error-handling step reached only when Login branches to `"03 Fallback Auth Error"`
