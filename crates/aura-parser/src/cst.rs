use aura_core::model::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CstDocument {
    pub items: Vec<CstTopLevelItem>,
}

impl CstDocument {
    pub fn has_conflicts(&self) -> bool {
        self.items.iter().any(|item| matches!(item, CstTopLevelItem::ConflictBlock(_)))
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
        self.to_request().map(|req| req.into())
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
                                    req.headers.insert(kv.key.clone(), kv.value.as_str().to_string());
                                }
                            }
                        }
                    }
                    "params.query" => {
                        for sub in &table.items {
                            if let CstTableItem::KeyValue(kv) = sub {
                                if !kv.key.ends_with('~') && !kv.key.starts_with('~') {
                                    req.query_params.insert(kv.key.clone(), kv.value.as_str().to_string());
                                }
                            }
                        }
                    }
                    "auth" => {
                        let mut auth_type = String::new();
                        let mut token = String::new();
                        let mut username = String::new();
                        let mut password = String::new();

                        for sub in &table.items {
                            if let CstTableItem::KeyValue(kv) = sub {
                                match kv.key.as_str() {
                                    "type" => auth_type = kv.value.as_str().to_lowercase(),
                                    "token" => token = kv.value.as_str().to_string(),
                                    "username" => username = kv.value.as_str().to_string(),
                                    "password" => password = kv.value.as_str().to_string(),
                                    _ => {}
                                }
                            }
                        }

                        if auth_type == "bearer" {
                            req.auth = Some(AuthConfig::Bearer { token });
                        } else if auth_type == "basic" {
                            req.auth = Some(AuthConfig::Basic { username, password });
                        }
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
                                        let file_path = val_str.trim_start_matches('@').trim().to_string();
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
                                    "content_type" => content_type = Some(kv.value.as_str().to_string()),
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
                                req.scripts.pre_request = Some(fb.content.clone());
                            }
                        }
                    }
                    "scripts.post-response" => {
                        for sub in &table.items {
                            if let CstTableItem::FencedBlock(fb) = sub {
                                req.scripts.post_response = Some(fb.content.clone());
                            }
                        }
                    }
                    "variables" => {
                        for sub in &table.items {
                            if let CstTableItem::KeyValue(kv) = sub {
                                if !kv.key.ends_with('~') && !kv.key.starts_with('~') {
                                    req.variables.insert(kv.key.clone(), kv.value.as_str().to_string());
                                }
                            }
                        }
                    }
                    "assert" | "asserts" => {
                        for sub in &table.items {
                            if let CstTableItem::FencedBlock(fb) = sub {
                                req.assert = Some(fb.content.clone());
                            }
                        }
                    }
                    "test" | "tests" => {
                        for sub in &table.items {
                            if let CstTableItem::FencedBlock(fb) = sub {
                                req.tests = Some(fb.content.clone());
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
