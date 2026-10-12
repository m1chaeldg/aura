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

[auth]
type = "bearer"
token = "{{api_bearer_token}}"

[params.query]
dry_run = "false"

[flow]
once_per_run = false
retry_until = "response.status == 201"
retry_interval_ms = 1000
max_retries = 3

# ------------------------------------------------------------------------------
# Request Body (Multipart file references strictly use relative paths)
# ------------------------------------------------------------------------------
[body.json]
```json
{
  "email": "{{$randomEmail}}",
  "username": "developer_{{$randomInt}}",
  "avatar_attachment": "./fixtures/avatar.png"
}
```

# ------------------------------------------------------------------------------
# Script Hooks (Executed via deno_core isolate)
# ------------------------------------------------------------------------------
[scripts.pre-request]
```javascript
const timestamp = Date.now().toString();
const secret = aura.getSecret("signing_key");
const encoder = new TextEncoder();

const key = await crypto.subtle.importKey(
  "raw",
  encoder.encode(secret),
  { name: "HMAC", hash: "SHA-256" },
  false,
  ["sign"]
);

const signatureBuffer = await crypto.subtle.sign(
  "HMAC",
  key,
  encoder.encode(`${timestamp}.${aura.request.getBody()}`)
);

const signature = Array.from(new Uint8Array(signatureBuffer))
  .map(b => b.toString(16).padStart(2, "0"))
  .join("");

aura.setHeader("X-Signature", signature);
aura.setHeader("X-Timestamp", timestamp);
```

[scripts.post-response]
```javascript
aura.test("User created successfully", () => {
  expect(aura.response.status).to.equal(201);
  const data = aura.response.json();
  expect(data.id).to.be.a("string");
  aura.setEnvVar("created_user_id", data.id);
});
```
