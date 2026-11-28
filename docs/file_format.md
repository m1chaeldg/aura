# ==============================================================================
# Aura Lossless Specification (.aura)
# ==============================================================================
version = "1.0"
name = "Create User Account"
method = "POST"
url = "{{base_url}}/api/v1/users"

[settings]
inherit_auth = true
verify_ssl = true
follow_redirects = true
timeout_ms = 15000

[headers]
Content-Type = "application/json"
Accept = "application/json"
X-Request-Id = "{{$uuid}}"
~X-Debug-Trace = "1"

[auth]
type = "bearer"
token = "{{api_bearer_token}}"

[params.query]
dry_run = "false"

[flow]
once_per_run = false
max_duration_ms = 5000

# ------------------------------------------------------------------------------
# Request Body (Multipart file references strictly use relative paths)
# ------------------------------------------------------------------------------
[body.json]
```json
{
  "email": "{{generated_email}}",
  "username": "developer_{{$randomInt}}"
}
```

# ------------------------------------------------------------------------------
# Script Hooks (Executed via deno_core V8 isolate)
# ------------------------------------------------------------------------------
[scripts.pre-request]
```javascript
// Generate dynamic values and store them for {{template}} interpolation
const email = faker.internet.email();
aura.variables.set("generated_email", email);
console.log("Creating user with email:", email);
```

[scripts.post-response]
```javascript
if (response.status === 201 || response.status === 200) {
  const data = response.json();
  if (data && data.id) {
    aura.variables.set("created_user_id", data.id);
  }
}
```

[scripts.tests]
```javascript
describe("Create User Account", () => {
  it("returns 200 or 201", () => {
    expect(response.status).toBeGreaterThanOrEqual(200);
    expect(response.status).toBeLessThan(300);
  });

  it("returns JSON content-type", () => {
    expect(response.headers["content-type"]).toContain("application/json");
  });
});
```

# ------------------------------------------------------------------------------
# No-Code Declarative Assertions & Zero-JS Extractors
# ------------------------------------------------------------------------------
[assert]
res.duration_ms = "< 5000"
res.headers.content-type = "contains application/json"

[extractors]
created_user_id = "jsonpath:$.id"
