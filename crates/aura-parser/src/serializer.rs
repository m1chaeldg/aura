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
            AuthConfig::OAuth2 { grant_type, access_token, token_url, client_id, client_secret } => {
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
    } else if let Some(assert) = &req.assert {
        if !assert.is_empty() {
            out.push_str("\n[tests]\n");
            out.push_str("```javascript\n");
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

    out
}

