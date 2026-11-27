use aura_core::model::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CstDocument {
    pub items: Vec<CstTopLevelItem>,
}

impl CstDocument {
    pub fn has_conflicts(&self) -> bool {
        self.items
            .iter()
            .any(|item| matches!(item, CstTopLevelItem::ConflictBlock(_)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CstTopLevelItem {
    Trivia(String),
    KeyValue(CstKeyValue),
    Table(CstTable),
    ConflictBlock(CstConflictBlock),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CstConflictBlock {
    pub raw: String,
    pub ours_label: String,
    pub ours_content: String,
    pub base_content: Option<String>,
    pub theirs_label: String,
    pub theirs_content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CstKeyValue {
    pub raw: String,
    pub key: String,
    pub value: CstValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CstValue {
    String(String),
    Boolean(bool),
    Integer(i64),
    Raw(String),
}

impl CstValue {
    pub fn as_str(&self) -> &str {
        match self {
            CstValue::String(s) | CstValue::Raw(s) => s.as_str(),
            _ => "",
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            CstValue::Boolean(b) => Some(*b),
            CstValue::String(s) | CstValue::Raw(s) => match s.as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            CstValue::Integer(i) => Some(*i),
            CstValue::String(s) | CstValue::Raw(s) => s.parse().ok(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CstTable {
    pub header_raw: String,
    pub name: String,
    pub items: Vec<CstTableItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CstTableItem {
    Trivia(String),
    KeyValue(CstKeyValue),
    FencedBlock(CstFencedBlock),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CstFencedBlock {
    pub raw: String,
    pub lang: String,
    pub content: String,
}

impl CstDocument {
    /// Converts the Concrete Syntax Tree to the typed AuraRequest runtime model
    pub fn to_aura_request(&self) -> Result<AuraRequest, String> {
        let mut req: AuraRequest = self.to_request()?.into();

        // Preserve disabled headers and query params (ending or starting with ~)
        for item in &self.items {
            if let CstTopLevelItem::Table(table) = item {
                match table.name.as_str() {
                    "headers" => {
                        for sub in &table.items {
                            if let CstTableItem::KeyValue(kv) = sub {
                                if kv.key.ends_with('~') || kv.key.starts_with('~') {
                                    let clean_key = kv.key.trim_matches('~').to_string();
                                    req.headers.push(HeaderItem::disabled(clean_key, kv.value.as_str()));
                                }
                            }
                        }
                    }
                    "params.query" => {
                        for sub in &table.items {
                            if let CstTableItem::KeyValue(kv) = sub {
                                if kv.key.ends_with('~') || kv.key.starts_with('~') {
                                    let clean_key = kv.key.trim_matches('~').to_string();
                                    req.params.push(ParamItem::disabled(clean_key, kv.value.as_str()));
                                }
                            }
                        }
                    }
                    "variables" => {
                        for sub in &table.items {
                            if let CstTableItem::KeyValue(kv) = sub {
                                if kv.key.ends_with('~') || kv.key.starts_with('~') {
                                    let clean_key = kv.key.trim_matches('~').to_string();
                                    req.disabled_variables
                                        .insert(clean_key, kv.value.as_str().to_string());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(req)
    }

    /// Converts the Concrete Syntax Tree to the typed file AuraFileRequest model
    pub fn to_request(&self) -> Result<AuraFileRequest, String> {
        let mut req = AuraFileRequest::default();

        // 1. Process top-level key-values
        for item in &self.items {
            if let CstTopLevelItem::KeyValue(kv) = item {
                match kv.key.as_str() {
                    "version" => req.version = kv.value.as_str().to_string(),
                    "name" => req.name = kv.value.as_str().to_string(),
                    "method" => req.method = kv.value.as_str().to_uppercase(),
                    "url" => req.url = kv.value.as_str().to_string(),
                    "protocol" => {
                        req.protocol = Some(RequestProtocol::from_str_lenient(kv.value.as_str()));
                    }
                    _ => {}
                }
            }
        }

        // 2. Process tables
        for item in &self.items {
            if let CstTopLevelItem::Table(table) = item {
                match table.name.as_str() {
                    "settings" => {
                        for sub in &table.items {
                            if let CstTableItem::KeyValue(kv) = sub {
                                match kv.key.as_str() {
                                    "inherit_auth" => {
                                        if let Some(b) = kv.value.as_bool() {
                                            req.settings.inherit_auth = b;
                                        }
                                    }
                                    "verify_ssl" => {
                                        if let Some(b) = kv.value.as_bool() {
                                            req.settings.verify_ssl = b;
                                        }
                                    }
                                    "follow_redirects" => {
                                        if let Some(b) = kv.value.as_bool() {
                                            req.settings.follow_redirects = b;
                                        }
                                    }
                                    "max_redirects" => {
                                        if let Some(i) = kv.value.as_i64() {
                                            req.settings.max_redirects = i as u32;
                                        }
                                    }
                                    "timeout_ms" => {
                                        if let Some(i) = kv.value.as_i64() {
                                            req.settings.timeout_ms = i as u64;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    "headers" => {
                        for sub in &table.items {
                            if let CstTableItem::KeyValue(kv) = sub {
                                if !kv.key.ends_with('~') && !kv.key.starts_with('~') {
                                    req.headers
                                        .insert(kv.key.clone(), kv.value.as_str().to_string());
                                }
                            }
                        }
                    }
                    "params.query" => {
                        for sub in &table.items {
                            if let CstTableItem::KeyValue(kv) = sub {
                                if !kv.key.ends_with('~') && !kv.key.starts_with('~') {
                                    req.query_params
                                        .insert(kv.key.clone(), kv.value.as_str().to_string());
                                }
                            }
                        }
                    }
                    "auth" => {
                        let mut kv_map: std::collections::HashMap<String, String> =
                            std::collections::HashMap::new();
                        for sub in &table.items {
                            if let CstTableItem::KeyValue(kv) = sub {
                                let v = if let Some(b) = kv.value.as_bool() {
                                    b.to_string()
                                } else if let Some(i) = kv.value.as_i64() {
                                    i.to_string()
                                } else {
                                    kv.value.as_str().to_string()
                                };
                                kv_map.insert(kv.key.clone(), v);
                            }
                        }
                        let auth_type = kv_map
                            .get("type")
                            .map(|s| s.to_lowercase())
                            .unwrap_or_default();
                        let get_s = |k: &str| kv_map.get(k).cloned().unwrap_or_default();
                        let get_opt = |k: &str| kv_map.get(k).cloned().filter(|v| !v.is_empty());
                        let get_bool = |k: &str, default: bool| {
                            kv_map
                                .get(k)
                                .and_then(|v| v.parse::<bool>().ok())
                                .unwrap_or(default)
                        };

                        req.auth = match auth_type.as_str() {
                            "bearer" => Some(AuthConfig::Bearer {
                                token: get_s("token"),
                            }),
                            "basic" => Some(AuthConfig::Basic {
                                username: get_s("username"),
                                password: get_s("password"),
                            }),
                            "apikey" | "api_key" => Some(AuthConfig::ApiKey {
                                key: get_s("key"),
                                value: get_s("value"),
                                in_header: get_bool("in_header", true),
                            }),
                            "oauth2" => Some(AuthConfig::OAuth2 {
                                grant_type: get_s("grant_type"),
                                access_token: get_s("access_token"),
                                token_url: get_opt("token_url"),
                                client_id: get_opt("client_id"),
                                client_secret: get_opt("client_secret"),
                                auth_url: get_opt("auth_url"),
                                redirect_uri: get_opt("redirect_uri"),
                                scope: get_opt("scope"),
                                username: get_opt("username"),
                                password: get_opt("password"),
                                refresh_token: get_opt("refresh_token"),
                                expires_at: kv_map
                                    .get("expires_at")
                                    .and_then(|v| v.parse::<i64>().ok()),
                                use_pkce: get_bool("use_pkce", true),
                                auto_refresh: get_bool("auto_refresh", true),
                            }),
                            "digest" => Some(AuthConfig::Digest {
                                username: get_s("username"),
                                password: get_s("password"),
                                realm: get_opt("realm"),
                                nonce: get_opt("nonce"),
                                qop: get_opt("qop"),
                                algorithm: get_opt("algorithm"),
                            }),
                            "ntlm" => Some(AuthConfig::Ntlm {
                                username: get_s("username"),
                                password: get_s("password"),
                                domain: get_s("domain"),
                                workstation: get_s("workstation"),
                            }),
                            "oauth1" => Some(AuthConfig::OAuth1 {
                                consumer_key: get_s("consumer_key"),
                                consumer_secret: get_s("consumer_secret"),
                                token: get_s("token"),
                                token_secret: get_s("token_secret"),
                                signature_method: get_s("signature_method"),
                                callback_url: get_opt("callback_url"),
                                realm: get_opt("realm"),
                                add_to_query: get_bool("add_to_query", false),
                            }),
                            "hawk" => Some(AuthConfig::Hawk {
                                id: get_s("id"),
                                key: get_s("key"),
                                algorithm: get_s("algorithm"),
                                ext: get_opt("ext"),
                                app: get_opt("app"),
                                dlg: get_opt("dlg"),
                            }),
                            "edgegrid" | "akamai_edgegrid" => Some(AuthConfig::AkamaiEdgeGrid {
                                client_token: get_s("client_token"),
                                client_secret: get_s("client_secret"),
                                access_token: get_s("access_token"),
                                max_body: kv_map
                                    .get("max_body")
                                    .and_then(|v| v.parse::<usize>().ok())
                                    .unwrap_or(131072),
                            }),
                            "wsse" => Some(AuthConfig::Wsse {
                                username: get_s("username"),
                                password: get_s("password"),
                                digest_password: get_bool("digest_password", true),
                            }),
                            _ => None,
                        };
                    }
                    "flow" => {
                        let mut flow = FlowControl::default();
                        for sub in &table.items {
                            if let CstTableItem::KeyValue(kv) = sub {
                                match kv.key.as_str() {
                                    "once_per_run" => {
                                        if let Some(b) = kv.value.as_bool() {
                                            flow.once_per_run = b;
                                        }
                                    }
                                    "retry_until" => {
                                        flow.retry_until = Some(kv.value.as_str().to_string());
                                    }
                                    "retry_interval_ms" => {
                                        if let Some(i) = kv.value.as_i64() {
                                            flow.retry_interval_ms = i as u64;
                                        }
                                    }
                                    "max_retries" => {
                                        if let Some(i) = kv.value.as_i64() {
                                            flow.max_retries = i as u32;
                                        }
                                    }
                                    "delay_ms" => {
                                        if let Some(i) = kv.value.as_i64() {
                                            flow.delay_ms = Some(i as u64);
                                        }
                                    }
                                    "jitter_min_ms" => {
                                        if let Some(i) = kv.value.as_i64() {
                                            flow.jitter_min_ms = Some(i as u64);
                                        }
                                    }
                                    "jitter_max_ms" => {
                                        if let Some(i) = kv.value.as_i64() {
                                            flow.jitter_max_ms = Some(i as u64);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        req.flow = Some(flow);
                    }
                    "body.json" => {
                        for sub in &table.items {
                            if let CstTableItem::FencedBlock(fb) = sub {
                                req.body = BodyType::Json(fb.content.clone());
                            }
                        }
                    }
                    "body.xml" => {
                        for sub in &table.items {
                            if let CstTableItem::FencedBlock(fb) = sub {
                                req.body = BodyType::Xml(fb.content.clone());
                            }
                        }
                    }
                    "body.text" => {
                        for sub in &table.items {
                            if let CstTableItem::FencedBlock(fb) = sub {
                                req.body = BodyType::Text(fb.content.clone());
                            }
                        }
                    }
                    "body.raw" => {
                        for sub in &table.items {
                            if let CstTableItem::FencedBlock(fb) = sub {
                                req.body = BodyType::Raw {
                                    content: fb.content.clone(),
                                    content_type: None,
                                };
                            }
                        }
                    }
                    "body.graphql" => {
                        let mut query = String::new();
                        let mut variables = String::new();
                        for sub in &table.items {
                            match sub {
                                CstTableItem::FencedBlock(fb) => {
                                    if fb.lang.eq_ignore_ascii_case("json")
                                        || (!query.is_empty() && variables.is_empty())
                                    {
                                        variables =
                                            fb.content.trim_end_matches(['\r', '\n']).to_string();
                                    } else {
                                        query =
                                            fb.content.trim_end_matches(['\r', '\n']).to_string();
                                    }
                                }
                                CstTableItem::KeyValue(kv) if kv.key == "variables" => {
                                    variables = kv.value.as_str().to_string();
                                }
                                _ => {}
                            }
                        }
                        req.body = BodyType::GraphQL { query, variables };
                    }
                    "grpc" => {
                        let mut cfg = GrpcConfig::default();
                        for sub in &table.items {
                            match sub {
                                CstTableItem::KeyValue(kv) => match kv.key.as_str() {
                                    "proto_path" => {
                                        cfg.proto_path = Some(kv.value.as_str().to_string())
                                    }
                                    "service" => cfg.service = kv.value.as_str().to_string(),
                                    "method" => cfg.method = kv.value.as_str().to_string(),
                                    "use_reflection" => {
                                        cfg.use_reflection = kv.value.as_bool().unwrap_or(false)
                                    }
                                    "payload" => cfg.payload = kv.value.as_str().to_string(),
                                    _ => {}
                                },
                                CstTableItem::FencedBlock(fb) => {
                                    cfg.payload =
                                        fb.content.trim_end_matches(['\r', '\n']).to_string();
                                }
                                _ => {}
                            }
                        }
                        req.grpc = Some(cfg);
                    }
                    "body.form" | "body.form-urlencoded" => {
                        let mut form = indexmap::IndexMap::new();
                        for sub in &table.items {
                            if let CstTableItem::KeyValue(kv) = sub {
                                if !kv.key.ends_with('~') && !kv.key.starts_with('~') {
                                    form.insert(kv.key.clone(), kv.value.as_str().to_string());
                                }
                            }
                        }
                        req.body = BodyType::Form(form);
                    }
                    "body.multipart" => {
                        let mut parts = Vec::new();
                        for sub in &table.items {
                            if let CstTableItem::KeyValue(kv) = sub {
                                if !kv.key.ends_with('~') && !kv.key.starts_with('~') {
                                    let val_str = kv.value.as_str();
                                    if val_str.starts_with('@') {
                                        let file_path =
                                            val_str.trim_start_matches('@').trim().to_string();
                                        parts.push(MultipartPart {
                                            name: kv.key.clone(),
                                            value: None,
                                            file_path: Some(file_path),
                                            content_type: None,
                                        });
                                    } else {
                                        parts.push(MultipartPart {
                                            name: kv.key.clone(),
                                            value: Some(val_str.to_string()),
                                            file_path: None,
                                            content_type: None,
                                        });
                                    }
                                }
                            }
                        }
                        req.body = BodyType::Multipart(parts);
                    }
                    "body.binary" => {
                        let mut file_path = String::new();
                        let mut content_type = None;
                        for sub in &table.items {
                            if let CstTableItem::KeyValue(kv) = sub {
                                match kv.key.as_str() {
                                    "file" | "path" => file_path = kv.value.as_str().to_string(),
                                    "content_type" => {
                                        content_type = Some(kv.value.as_str().to_string())
                                    }
                                    _ => {}
                                }
                            }
                        }
                        req.body = BodyType::Binary {
                            file_path,
                            content_type,
                        };
                    }
                    "scripts.pre-request" => {
                        for sub in &table.items {
                            if let CstTableItem::FencedBlock(fb) = sub {
                                req.scripts.pre_request =
                                    Some(fb.content.trim_end_matches(['\r', '\n']).to_string());
                            }
                        }
                    }
                    "scripts.post-response" => {
                        for sub in &table.items {
                            if let CstTableItem::FencedBlock(fb) = sub {
                                req.scripts.post_response =
                                    Some(fb.content.trim_end_matches(['\r', '\n']).to_string());
                            }
                        }
                    }
                    "variables" => {
                        for sub in &table.items {
                            if let CstTableItem::KeyValue(kv) = sub {
                                if !kv.key.ends_with('~') && !kv.key.starts_with('~') {
                                    req.variables
                                        .insert(kv.key.clone(), kv.value.as_str().to_string());
                                }
                            }
                        }
                    }
                    "assert" | "asserts" => {
                        let mut lines: Vec<String> = Vec::new();
                        for sub in &table.items {
                            match sub {
                                CstTableItem::FencedBlock(fb) => {
                                    let txt = fb.content.trim_end_matches(['\r', '\n']);
                                    if !txt.trim().is_empty() {
                                        lines.push(txt.to_string());
                                    }
                                }
                                CstTableItem::KeyValue(kv) => {
                                    lines.push(format!("{} {}", kv.key.trim(), kv.value.as_str().trim()));
                                }
                                _ => {}
                            }
                        }
                        if !lines.is_empty() {
                            req.assert = Some(lines.join("\n"));
                        }
                    }
                    "test" | "tests" => {
                        for sub in &table.items {
                            if let CstTableItem::FencedBlock(fb) = sub {
                                req.tests =
                                    Some(fb.content.trim_end_matches(['\r', '\n']).to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(req)
    }
}
