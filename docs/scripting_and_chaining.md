# Scripting, Declarative Assertions, and Request Chaining

Aura combines Bruno-style plain-text `[assert]` rules, async JavaScript scripting powered by `deno_core` (V8), and Postman/Bruno-compatible Collection Runner chaining (`aura.setNextRequest`).

---

## 1. Pre-Request Scripts

Pre-request scripts run **before** the HTTP request is dispatched. Any variable set with `aura.variables.set(key, value)` is immediately available as `{{key}}` in the request URL, headers, query parameters, auth fields, and body.

### Example: Dynamic Fake Data + Async Token Fetch

```javascript
// Generate realistic test data using built-in offline faker
aura.variables.set("request_id", faker.datatype.uuid());
aura.variables.set("fake_email", faker.internet.email());
aura.variables.set("fake_name", faker.person.fullName());

// Fetch an auth token asynchronously before sending the main request
const tokenRes = await fetch("https://postman-echo.com/get?token=demo-token");
if (tokenRes.ok) {
    const payload = await tokenRes.json();
    aura.variables.set("access_token", payload.args.token);
}

console.log("Pre-request ready for:", aura.variables.get("fake_email"));
```

---

## 2. Post-Response Scripts

Post-response scripts run **after** the HTTP response arrives. Inspect `response` (also aliased as `aura.response` and `pm.response`), save values for downstream requests, or trigger auxiliary HTTP calls.

### `response` Object Properties & Methods

| Property / Method | Description |
|---|---|
| `response.status` / `response.code` | HTTP status code (e.g., `200`) |
| `response.duration_ms` / `response.responseTime` | Total response time in milliseconds |
| `response.headers` | Object map of response headers |
| `response.body` | Raw response body string |
| `response.text()` | Returns raw response body string |
| `response.json()` | Parses response body as JSON |

### Example: Extract Values & Send Auxiliary Webhook

```javascript
const data = response.json();
if (response.status === 200 && data?.json?.email) {
    aura.variables.set("created_email", data.json.email);
}

// Works with async/await OR a Node-style callback: (err, res) => { ... }
const audit = await aura.sendRequest({
    url: "https://postman-echo.com/post",
    method: "POST",
    headers: { "content-type": "application/json" },
    body: { source: "post-response-hook", status: response.status, duration_ms: response.duration_ms }
});

console.log("Audit webhook status:", audit.status);
```

---

## 3. Creating Your Own Tests

### Option A: Declarative / No-Code Assertions (`[assert]`)

Declarative assertions let you validate HTTP responses without writing JavaScript.

#### Using the UI

1. Open any request and switch to the **Tests** tab.
2. In the **NO-CODE ASSERTIONS (`[assert]`)** section, click **Add Rule** or choose a preset chip (`Status 200`, `body.id isNumber`, `Time < 500ms`).
3. Enter the target expression (e.g., `res.body.user.email`), click the operator pill to cycle operators, and type the expected value (unary operators like `isNumber` hide the expected input).
4. Click **Run Tests** or **Send** — declarative rules run automatically and appear alongside JS BDD `it(...)` checks in the **Tests** response tab.

#### `.aura` File Syntax

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

#### Supported Expressions & Operators

- **Targets**: `res.status`, `res.duration_ms`, `res.size_bytes`, `res.body.<nested.path[0]>`, `res.headers.<header-name>`
- **Comparisons**: `==` (`eq`), `!=` (`neq`), `>` (`gt`), `>=` (`gte`), `<` (`lt`), `<=` (`lte`)
- **Strings / Collections / Regex**: `contains`, `notContains`, `startsWith`, `endsWith`, `matches` (regex), `in` (`a, b, c`), `notIn`, `length`
- **Unary Checks**: `isDefined`, `isUndefined`, `isNull`, `isNotNull`, `isEmpty`, `isNotEmpty`, `isNumber`, `isString`, `isBoolean`, `isArray`, `isObject`

---

### Option B: JavaScript BDD Tests (`describe` / `it` / `test` / `aura.test` / `expect`)

Write expressive test suites in the **Tests** tab (`[scripts.tests]` in `.aura`). Both Jest-style and Chai-style `expect(...)` assertions are built in:

```javascript
describe("User API", () => {
    it("returns 200 OK within 2000ms", () => {
        expect(response.status).toBe(200);
        expect(response.duration_ms).toBeLessThan(2000);
    });

    it("returns a valid JSON payload", () => {
        const body = response.json();
        expect(body).to.have.property("json");
        expect(body.json.email).toContain("@");
        expect(body.json.active).to.be.true;
    });
});

// Postman-compatible syntax is also supported:
pm.test("Content-Type is JSON", () => {
    pm.expect(pm.response.headers["content-type"]).toContain("application/json");
});
```

#### `expect(actual)` Matcher Reference

- **Jest-style**:
  - `.toBe(expected)`, `.toEqual(expected)`
  - `.toBeDefined()`, `.toBeUndefined()`, `.toBeNull()`, `.toBeTruthy()`, `.toBeFalsy()`
  - `.toBeGreaterThan(n)`, `.toBeLessThan(n)`, `.toBeGreaterThanOrEqual(n)`, `.toBeLessThanOrEqual(n)`
  - `.toContain(item)`, `.toMatch(pattern)`, `.toThrow()`
  - Negation: `.not.<matcher>` (e.g. `expect(body.error).not.toBeDefined()`)
- **Chai-style**:
  - `.to.equal(expected)`, `.to.deep.equal(expected)`
  - `.to.be.a(type)`, `.to.be.true`, `.to.be.false`
  - `.to.have.property(prop)`

---

## 4. Built-in `faker` & Postman Compatibility (`pm.*`)

Aura provides a zero-dependency `faker` global in all scripts:

- `faker.person`: `firstName()`, `lastName()`, `fullName()`, `jobTitle()`
- `faker.internet`: `email()`, `exampleEmail()`, `userName()`, `password(len)`, `url()`, `ip()`, `ipv6()`, `color()`
- `faker.number`: `int({ min, max })`, `float({ min, max, precision })`
- `faker.datatype` / `faker.string` / `faker.random`: `uuid()`, `boolean()`, `alphanumeric(len)`, `word()`, `words(count)`
- `faker.location` / `faker.company` / `faker.commerce` / `faker.lorem` / `faker.date`

Postman aliases: `pm.variables`, `pm.environment`, `pm.sendRequest`, `pm.setNextRequest`, `pm.execution.setNextRequest`, `pm.test`, `pm.expect`, `pm.response`.

---

## 5. Request Chaining (`aura.setNextRequest`)

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

### Sample Collections

- **Chaining**: [examples/chaining/](examples/chaining/) (`01_login.aura`, `02_fetch_profile.aura`, `03_fallback_error.aura`)
- **Complete Scripts & Tests**: [examples/rest_crud/02_complete_scripts_and_tests.aura](examples/rest_crud/02_complete_scripts_and_tests.aura)
