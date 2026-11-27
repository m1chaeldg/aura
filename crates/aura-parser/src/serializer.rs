use crate::cst::*;
use aura_core::model::{AuraFileRequest, AuraRequest, AuthConfig, BodyType};

/// Losslessly serializes a CstDocument back to a String.
/// Invariant: `serialize(&parse(input).unwrap()) == input`
pub fn serialize(doc: &CstDocument) -> String {
    let mut out = String::new();

    for item in &doc.items {
        match item {
            CstTopLevelItem::Trivia(s) => out.push_str(s),
            CstTopLevelItem::KeyValue(kv) => out.push_str(&kv.raw),
            CstTopLevelItem::Table(table) => {
                out.push_str(&table.header_raw);
                for sub in &table.items {
                    match sub {
                        CstTableItem::Trivia(s) => out.push_str(s),
                        CstTableItem::KeyValue(kv) => out.push_str(&kv.raw),
                        CstTableItem::FencedBlock(fb) => out.push_str(&fb.raw),
                    }
                }
            }
            CstTopLevelItem::ConflictBlock(cb) => out.push_str(&cb.raw),
        }
    }

    out
}

/// Serializes an AuraRequest to the canonical `.aura` specification format.
pub fn request_to_spec(req: &AuraRequest) -> String {
    let file_req = AuraFileRequest::from(req.clone());
    file_request_to_spec(&file_req)
}

/// Serializes an AuraFileRequest to the canonical `.aura` specification format.
pub fn file_request_to_spec(req: &AuraFileRequest) -> String {
    let mut out = String::new();

    out.push_str("version = \"1.0\"\n");
    out.push_str(&format!("name = \"{}\"\n", req.name.replace('"', "\\\"")));
    out.push_str(&format!("method = \"{}\"\n", req.method));
    out.push_str(&format!("url = \"{}\"\n", req.url.replace('"', "\\\"")));
    if let Some(proto) = req.protocol {
        if proto != aura_core::model::RequestProtocol::Http {
            out.push_str(&format!(
                "protocol = \"{}\"\n",
                proto.as_str().to_lowercase()
            ));
        }
    }

    out.push_str("\n[settings]\n");
    out.push_str(&format!("inherit_auth = {}\n", req.settings.inherit_auth));
    out.push_str(&format!("verify_ssl = {}\n", req.settings.verify_ssl));
    out.push_str(&format!("follow_redirects = {}\n", req.settings.follow_redirects));
    out.push_str(&format!("max_redirects = {}\n", req.settings.max_redirects));
    out.push_str(&format!("timeout_ms = {}\n", req.settings.timeout_ms));

    if !req.headers.is_empty() {
        out.push_str("\n[headers]\n");
        for (k, v) in &req.headers {
            out.push_str(&format!("{} = \"{}\"\n", k, v.replace('"', "\\\"")));
        }
    }

    if !req.query_params.is_empty() {
        out.push_str("\n[params.query]\n");
        for (k, v) in &req.query_params {
            out.push_str(&format!("{} = \"{}\"\n", k, v.replace('"', "\\\"")));
        }
    }

    if let Some(auth) = &req.auth {
        match auth {
            AuthConfig::Bearer { token } => {
                out.push_str("\n[auth]\n");
                out.push_str("type = \"bearer\"\n");
                out.push_str(&format!("token = \"{}\"\n", token.replace('"', "\\\"")));
            }
            AuthConfig::Basic { username, password } => {
                out.push_str("\n[auth]\n");
                out.push_str("type = \"basic\"\n");
                out.push_str(&format!("username = \"{}\"\n", username.replace('"', "\\\"")));
                out.push_str(&format!("password = \"{}\"\n", password.replace('"', "\\\"")));
            }
            AuthConfig::ApiKey { key, value, in_header } => {
                out.push_str("\n[auth]\n");
                out.push_str("type = \"apikey\"\n");
                out.push_str(&format!("key = \"{}\"\n", key.replace('"', "\\\"")));
                out.push_str(&format!("value = \"{}\"\n", value.replace('"', "\\\"")));
                out.push_str(&format!("in_header = {}\n", in_header));
            }
            AuthConfig::AwsSigV4 {
                use_profile,
                profile,
                credentials_path,
                access_key,
                secret_key,
                region,
                service,
                session_token,
            } => {
                out.push_str("\n[auth]\n");
                out.push_str("type = \"awssigv4\"\n");
                out.push_str(&format!("use_profile = {}\n", use_profile));
                out.push_str(&format!("profile = \"{}\"\n", profile.replace('"', "\\\"")));
                if let Some(cp) = credentials_path {
                    out.push_str(&format!("credentials_path = \"{}\"\n", cp.replace('"', "\\\"")));
                }
                out.push_str(&format!("access_key = \"{}\"\n", access_key.replace('"', "\\\"")));
                out.push_str(&format!("secret_key = \"{}\"\n", secret_key.replace('"', "\\\"")));
                out.push_str(&format!("region = \"{}\"\n", region.replace('"', "\\\"")));
                out.push_str(&format!("service = \"{}\"\n", service.replace('"', "\\\"")));
                if let Some(st) = session_token {
                    out.push_str(&format!("session_token = \"{}\"\n", st.replace('"', "\\\"")));
                }
            }
            AuthConfig::OAuth2 {
                grant_type,
                access_token,
                token_url,
                client_id,
                client_secret,
                auth_url,
                redirect_uri,
                scope,
                username,
                password,
                refresh_token,
                expires_at,
                use_pkce,
                auto_refresh,
            } => {
                out.push_str("\n[auth]\n");
                out.push_str("type = \"oauth2\"\n");
                out.push_str(&format!("grant_type = \"{}\"\n", grant_type.replace('"', "\\\"")));
                out.push_str(&format!("access_token = \"{}\"\n", access_token.replace('"', "\\\"")));
                if let Some(tu) = token_url {
                    out.push_str(&format!("token_url = \"{}\"\n", tu.replace('"', "\\\"")));
                }
                if let Some(ci) = client_id {
                    out.push_str(&format!("client_id = \"{}\"\n", ci.replace('"', "\\\"")));
                }
                if let Some(cs) = client_secret {
                    out.push_str(&format!("client_secret = \"{}\"\n", cs.replace('"', "\\\"")));
                }
                if let Some(au) = auth_url {
                    out.push_str(&format!("auth_url = \"{}\"\n", au.replace('"', "\\\"")));
                }
                if let Some(ru) = redirect_uri {
                    out.push_str(&format!("redirect_uri = \"{}\"\n", ru.replace('"', "\\\"")));
                }
                if let Some(sc) = scope {
                    out.push_str(&format!("scope = \"{}\"\n", sc.replace('"', "\\\"")));
                }
                if let Some(u) = username {
                    out.push_str(&format!("username = \"{}\"\n", u.replace('"', "\\\"")));
                }
                if let Some(p) = password {
                    out.push_str(&format!("password = \"{}\"\n", p.replace('"', "\\\"")));
                }
                if let Some(rt) = refresh_token {
                    out.push_str(&format!("refresh_token = \"{}\"\n", rt.replace('"', "\\\"")));
                }
                if let Some(exp) = expires_at {
                    out.push_str(&format!("expires_at = {}\n", exp));
                }
                out.push_str(&format!("use_pkce = {}\n", use_pkce));
                out.push_str(&format!("auto_refresh = {}\n", auto_refresh));
            }
            AuthConfig::Digest {
                username,
                password,
                realm,
                nonce,
                qop,
                algorithm,
            } => {
                out.push_str("\n[auth]\n");
                out.push_str("type = \"digest\"\n");
                out.push_str(&format!("username = \"{}\"\n", username.replace('"', "\\\"")));
                out.push_str(&format!("password = \"{}\"\n", password.replace('"', "\\\"")));
                if let Some(r) = realm {
                    out.push_str(&format!("realm = \"{}\"\n", r.replace('"', "\\\"")));
                }
                if let Some(n) = nonce {
                    out.push_str(&format!("nonce = \"{}\"\n", n.replace('"', "\\\"")));
                }
                if let Some(q) = qop {
                    out.push_str(&format!("qop = \"{}\"\n", q.replace('"', "\\\"")));
                }
                if let Some(a) = algorithm {
                    out.push_str(&format!("algorithm = \"{}\"\n", a.replace('"', "\\\"")));
                }
            }
            AuthConfig::Ntlm {
                username,
                password,
                domain,
                workstation,
            } => {
                out.push_str("\n[auth]\n");
                out.push_str("type = \"ntlm\"\n");
                out.push_str(&format!("username = \"{}\"\n", username.replace('"', "\\\"")));
                out.push_str(&format!("password = \"{}\"\n", password.replace('"', "\\\"")));
                out.push_str(&format!("domain = \"{}\"\n", domain.replace('"', "\\\"")));
                out.push_str(&format!("workstation = \"{}\"\n", workstation.replace('"', "\\\"")));
            }
            AuthConfig::OAuth1 {
                consumer_key,
                consumer_secret,
                token,
                token_secret,
                signature_method,
                callback_url,
                realm,
                add_to_query,
            } => {
                out.push_str("\n[auth]\n");
                out.push_str("type = \"oauth1\"\n");
                out.push_str(&format!("consumer_key = \"{}\"\n", consumer_key.replace('"', "\\\"")));
                out.push_str(&format!("consumer_secret = \"{}\"\n", consumer_secret.replace('"', "\\\"")));
                out.push_str(&format!("token = \"{}\"\n", token.replace('"', "\\\"")));
                out.push_str(&format!("token_secret = \"{}\"\n", token_secret.replace('"', "\\\"")));
                out.push_str(&format!("signature_method = \"{}\"\n", signature_method.replace('"', "\\\"")));
                if let Some(cb) = callback_url {
                    out.push_str(&format!("callback_url = \"{}\"\n", cb.replace('"', "\\\"")));
                }
                if let Some(r) = realm {
                    out.push_str(&format!("realm = \"{}\"\n", r.replace('"', "\\\"")));
                }
                out.push_str(&format!("add_to_query = {}\n", add_to_query));
            }
            AuthConfig::Hawk {
                id,
                key,
                algorithm,
                ext,
                app,
                dlg,
            } => {
                out.push_str("\n[auth]\n");
                out.push_str("type = \"hawk\"\n");
                out.push_str(&format!("id = \"{}\"\n", id.replace('"', "\\\"")));
                out.push_str(&format!("key = \"{}\"\n", key.replace('"', "\\\"")));
                out.push_str(&format!("algorithm = \"{}\"\n", algorithm.replace('"', "\\\"")));
                if let Some(e) = ext {
                    out.push_str(&format!("ext = \"{}\"\n", e.replace('"', "\\\"")));
                }
                if let Some(a) = app {
                    out.push_str(&format!("app = \"{}\"\n", a.replace('"', "\\\"")));
                }
                if let Some(d) = dlg {
                    out.push_str(&format!("dlg = \"{}\"\n", d.replace('"', "\\\"")));
                }
            }
            AuthConfig::AkamaiEdgeGrid {
                client_token,
                client_secret,
                access_token,
                max_body,
            } => {
                out.push_str("\n[auth]\n");
                out.push_str("type = \"edgegrid\"\n");
                out.push_str(&format!("client_token = \"{}\"\n", client_token.replace('"', "\\\"")));
                out.push_str(&format!("client_secret = \"{}\"\n", client_secret.replace('"', "\\\"")));
                out.push_str(&format!("access_token = \"{}\"\n", access_token.replace('"', "\\\"")));
                out.push_str(&format!("max_body = {}\n", max_body));
            }
            AuthConfig::Wsse {
                username,
                password,
                digest_password,
            } => {
                out.push_str("\n[auth]\n");
                out.push_str("type = \"wsse\"\n");
                out.push_str(&format!("username = \"{}\"\n", username.replace('"', "\\\"")));
                out.push_str(&format!("password = \"{}\"\n", password.replace('"', "\\\"")));
                out.push_str(&format!("digest_password = {}\n", digest_password));
            }
            AuthConfig::Inherit | AuthConfig::None => {}
        }
    }

    if let Some(flow) = &req.flow {
        out.push_str("\n[flow]\n");
        out.push_str(&format!("once_per_run = {}\n", flow.once_per_run));
        if let Some(retry) = &flow.retry_until {
            out.push_str(&format!("retry_until = \"{}\"\n", retry.replace('"', "\\\"")));
        }
        out.push_str(&format!("retry_interval_ms = {}\n", flow.retry_interval_ms));
        out.push_str(&format!("max_retries = {}\n", flow.max_retries));
        if let Some(delay) = flow.delay_ms {
            out.push_str(&format!("delay_ms = {}\n", delay));
        }
        if let Some(j_min) = flow.jitter_min_ms {
            out.push_str(&format!("jitter_min_ms = {}\n", j_min));
        }
        if let Some(j_max) = flow.jitter_max_ms {
            out.push_str(&format!("jitter_max_ms = {}\n", j_max));
        }
        if let Some(gm) = flow.gaussian_mean_ms {
            out.push_str(&format!("gaussian_mean_ms = {}\n", gm));
        }
        if let Some(gs) = flow.gaussian_stddev_ms {
            out.push_str(&format!("gaussian_stddev_ms = {}\n", gs));
        }
        if let Some(w) = flow.weight_percent {
            out.push_str(&format!("weight_percent = {}\n", w));
        }
        if let Some(lc) = flow.loop_count {
            out.push_str(&format!("loop_count = {}\n", lc));
        }
        if let Some(cond) = &flow.condition_if {
            out.push_str(&format!("condition_if = \"{}\"\n", cond.replace('"', "\\\"")));
        }
        if let Some(lw) = &flow.loop_while {
            out.push_str(&format!("loop_while = \"{}\"\n", lw.replace('"', "\\\"")));
        }
        if flow.max_loops != 10 {
            out.push_str(&format!("max_loops = {}\n", flow.max_loops));
        }
        if let Some(md) = flow.max_duration_ms {
            out.push_str(&format!("max_duration_ms = {}\n", md));
        }
        if let Some(min_b) = flow.min_size_bytes {
            out.push_str(&format!("min_size_bytes = {}\n", min_b));
        }
        if let Some(max_b) = flow.max_size_bytes {
            out.push_str(&format!("max_size_bytes = {}\n", max_b));
        }
    }

    match &req.body {
        BodyType::None => {}
        BodyType::Json(s) => {
            out.push_str("\n[body.json]\n");
            out.push_str("```json\n");
            out.push_str(s);
            if !s.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n");
        }
        BodyType::Xml(s) => {
            out.push_str("\n[body.xml]\n");
            out.push_str("```xml\n");
            out.push_str(s);
            if !s.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n");
        }
        BodyType::Text(s) => {
            out.push_str("\n[body.text]\n");
            out.push_str("```text\n");
            out.push_str(s);
            if !s.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n");
        }
        BodyType::Raw { content, .. } => {
            out.push_str("\n[body.raw]\n");
            out.push_str("```raw\n");
            out.push_str(content);
            if !content.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n");
        }
        BodyType::Form(form) => {
            out.push_str("\n[body.form]\n");
            for (k, v) in form {
                out.push_str(&format!("{} = \"{}\"\n", k, v.replace('"', "\\\"")));
            }
        }
        BodyType::FormUrlEncoded(items) => {
            out.push_str("\n[body.form-urlencoded]\n");
            for item in items {
                let key = if item.enabled {
                    item.key.clone()
                } else {
                    format!("{}~", item.key)
                };
                out.push_str(&format!("{} = \"{}\"\n", key, item.value.replace('"', "\\\"")));
            }
        }
        BodyType::Multipart(parts) => {
            out.push_str("\n[body.multipart]\n");
            for p in parts {
                if let Some(fpath) = &p.file_path {
                    out.push_str(&format!("{} = \"@{}\"\n", p.name, fpath.replace('"', "\\\"")));
                } else if let Some(val) = &p.value {
                    out.push_str(&format!("{} = \"{}\"\n", p.name, val.replace('"', "\\\"")));
                }
            }
        }
        BodyType::Binary { file_path, content_type } => {
            out.push_str("\n[body.binary]\n");
            out.push_str(&format!("file = \"{}\"\n", file_path.replace('"', "\\\"")));
            if let Some(ct) = content_type {
                out.push_str(&format!("content_type = \"{}\"\n", ct.replace('"', "\\\"")));
            }
        }
        BodyType::GraphQL { query, variables } => {
            out.push_str("\n[body.graphql]\n");
            out.push_str("```graphql\n");
            out.push_str(query);
            if !query.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n");
            if !variables.trim().is_empty() {
                out.push_str("```json\n");
                out.push_str(variables);
                if !variables.ends_with('\n') {
                    out.push('\n');
                }
                out.push_str("```\n");
            }
        }
    }

    if let Some(grpc) = &req.grpc {
        out.push_str("\n[grpc]\n");
        if let Some(pp) = &grpc.proto_path {
            out.push_str(&format!("proto_path = \"{}\"\n", pp.replace('"', "\\\"")));
        }
        out.push_str(&format!("service = \"{}\"\n", grpc.service.replace('"', "\\\"")));
        out.push_str(&format!("method = \"{}\"\n", grpc.method.replace('"', "\\\"")));
        out.push_str(&format!("use_reflection = {}\n", grpc.use_reflection));
        if !grpc.payload.trim().is_empty() {
            out.push_str("```json\n");
            out.push_str(&grpc.payload);
            if !grpc.payload.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n");
        }
    }

    if let Some(pre) = &req.scripts.pre_request {
        if !pre.is_empty() {
            out.push_str("\n[scripts.pre-request]\n");
            out.push_str("```javascript\n");
            out.push_str(pre);
            if !pre.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n");
        }
    }

    if let Some(post) = &req.scripts.post_response {
        if !post.is_empty() {
            out.push_str("\n[scripts.post-response]\n");
            out.push_str("```javascript\n");
            out.push_str(post);
            if !post.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n");
        }
    }

    if let Some(tests) = &req.tests {
        if !tests.is_empty() {
            out.push_str("\n[tests]\n");
            out.push_str("```javascript\n");
            out.push_str(tests);
            if !tests.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n");
        }
    }
    if let Some(assert) = &req.assert {
        if !assert.is_empty() {
            out.push_str("\n[assert]\n");
            out.push_str("```assert\n");
            out.push_str(assert);
            if !assert.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n");
        }
    }

    if !req.variables.is_empty() {
        out.push_str("\n[variables]\n");
        for (k, v) in &req.variables {
            out.push_str(&format!("{} = \"{}\"\n", k, v.replace('"', "\\\"")));
        }
    }

    if !req.extractors.is_empty() {
        out.push_str("\n[extractors]\n");
        for ext in &req.extractors {
            let (k, v) = aura_core::extractors::serialize_extractor_kv(ext);
            out.push_str(&format!("{} = \"{}\"\n", k, v.replace('"', "\\\"")));
        }
    }

    out
}

