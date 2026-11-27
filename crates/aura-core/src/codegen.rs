//! Request code snippet exporter for cURL, fetch, axios, Python requests,
//! Rust reqwest, Go net/http, and raw HTTP/1.1.

use crate::model::{AuraRequest, BodyType};
use crate::security::apply_auth;
use crate::templating::TemplateContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenLanguage {
    Curl,
    JsFetch,
    JsAxios,
    PythonRequests,
    RustReqwest,
    GoHttp,
    HttpRaw,
}

impl CodegenLanguage {
    pub const ALL: [Self; 7] = [
        Self::Curl,
        Self::JsFetch,
        Self::JsAxios,
        Self::PythonRequests,
        Self::RustReqwest,
        Self::GoHttp,
        Self::HttpRaw,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Curl => "cURL",
            Self::JsFetch => "JavaScript fetch",
            Self::JsAxios => "JavaScript axios",
            Self::PythonRequests => "Python requests",
            Self::RustReqwest => "Rust reqwest",
            Self::GoHttp => "Go net/http",
            Self::HttpRaw => "HTTP Raw",
        }
    }

    pub fn editor_language(self) -> &'static str {
        match self {
            Self::Curl | Self::HttpRaw => "text",
            Self::JsFetch | Self::JsAxios => "javascript",
            Self::PythonRequests | Self::RustReqwest | Self::GoHttp => "text",
        }
    }
}

struct PreparedRequest {
    method: String,
    full_url: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
    form_fields: Vec<(String, String)>,
}

fn prepare_request(req: &AuraRequest, ctx: Option<&TemplateContext>) -> PreparedRequest {
    let render = |s: &str| match ctx {
        Some(c) => c.render(s),
        None => s.to_string(),
    };

    let raw_url = render(&req.url);
    let mut url_params: Vec<(String, String)> = req
        .params
        .iter()
        .filter(|p| p.enabled)
        .map(|p| (render(&p.key), render(&p.value)))
        .collect();

    let mut headers: Vec<(String, String)> = req
        .headers
        .iter()
        .filter(|h| h.enabled)
        .map(|h| (render(&h.key), render(&h.value)))
        .collect();

    apply_auth(&req.auth, &mut headers, &mut url_params);
    if ctx.is_some() {
        for (k, v) in &mut headers {
            *k = render(k);
            *v = render(v);
        }
        for (k, v) in &mut url_params {
            *k = render(k);
            *v = render(v);
        }
    }

    let full_url = if url_params.is_empty() {
        raw_url
    } else if let Ok(mut parsed) = url::Url::parse(&raw_url) {
        {
            let mut qp = parsed.query_pairs_mut();
            for (k, v) in &url_params {
                qp.append_pair(k, v);
            }
        }
        parsed.to_string()
    } else {
        let sep = if raw_url.contains('?') { '&' } else { '?' };
        let q: Vec<String> = url_params
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        format!("{raw_url}{sep}{}", q.join("&"))
    };

    let mut form_fields = Vec::new();
    let body = match &req.body {
        BodyType::None => None,
        BodyType::Json(s) => {
            ensure_ct(&mut headers, "application/json");
            Some(render(s))
        }
        BodyType::Xml(s) => {
            ensure_ct(&mut headers, "application/xml");
            Some(render(s))
        }
        BodyType::Text(s) => {
            ensure_ct(&mut headers, "text/plain");
            Some(render(s))
        }
        BodyType::Raw { content, content_type } => {
            if let Some(ct) = content_type {
                ensure_ct(&mut headers, ct);
            }
            Some(render(content))
        }
        BodyType::GraphQL { query, variables } => {
            ensure_ct(&mut headers, "application/json");
            let vars_val: serde_json::Value =
                serde_json::from_str(&render(variables)).unwrap_or_else(|_| serde_json::json!({}));
            Some(
                serde_json::json!({
                    "query": render(query),
                    "variables": vars_val
                })
                .to_string(),
            )
        }
        BodyType::FormUrlEncoded(items) => {
            ensure_ct(&mut headers, "application/x-www-form-urlencoded");
            for it in items.iter().filter(|i| i.enabled) {
                form_fields.push((render(&it.key), render(&it.value)));
            }
            let encoded = url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(form_fields.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .finish();
            Some(encoded)
        }
        BodyType::Form(map) => {
            ensure_ct(&mut headers, "application/x-www-form-urlencoded");
            for (k, v) in map {
                form_fields.push((render(k), render(v)));
            }
            let encoded = url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(form_fields.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .finish();
            Some(encoded)
        }
        BodyType::Multipart(fields) => {
            for f in fields {
                let val = f
                    .value
                    .as_deref()
                    .map(&render)
                    .or_else(|| f.file_path.as_deref().map(|p| format!("@{}", render(p))))
                    .unwrap_or_default();
                form_fields.push((render(&f.name), val));
            }
            None
        }
        BodyType::Binary {
            file_path,
            content_type,
        } => {
            if let Some(ct) = content_type {
                ensure_ct(&mut headers, ct);
            }
            Some(format!("@{}", render(file_path)))
        }
    };

    PreparedRequest {
        method: req.method.as_str().to_string(),
        full_url,
        headers,
        body,
        form_fields,
    }
}

fn ensure_ct(headers: &mut Vec<(String, String)>, ct: &str) {
    if !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type")) {
        headers.push(("Content-Type".to_string(), ct.to_string()));
    }
}

fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn dq_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

pub fn generate_snippet(
    req: &AuraRequest,
    ctx: Option<&TemplateContext>,
    lang: CodegenLanguage,
) -> String {
    let prep = prepare_request(req, ctx);
    match lang {
        CodegenLanguage::Curl => gen_curl(req, &prep),
        CodegenLanguage::JsFetch => gen_js_fetch(&prep),
        CodegenLanguage::JsAxios => gen_js_axios(&prep),
        CodegenLanguage::PythonRequests => gen_python(&prep),
        CodegenLanguage::RustReqwest => gen_rust(&prep),
        CodegenLanguage::GoHttp => gen_go(&prep),
        CodegenLanguage::HttpRaw => gen_http_raw(&prep),
    }
}

fn gen_curl(req: &AuraRequest, prep: &PreparedRequest) -> String {
    let mut lines = vec![format!("curl --request {} \\", prep.method)];
    lines.push(format!("  --url {} \\", sh_quote(&prep.full_url)));
    if !req.settings.verify_ssl {
        lines.push("  --insecure \\".to_string());
    }
    if let Some(ref proxy) = req.settings.proxy {
        if proxy.enabled {
            if let Some(pu) = proxy.effective_url() {
                lines.push(format!("  --proxy {} \\", sh_quote(&pu)));
            }
        }
    }
    for (k, v) in &prep.headers {
        lines.push(format!("  --header {} \\", sh_quote(&format!("{k}: {v}"))));
    }
    if matches!(req.body, BodyType::Multipart(_)) {
        for (k, v) in &prep.form_fields {
            lines.push(format!("  --form {} \\", sh_quote(&format!("{k}={v}"))));
        }
    } else if let Some(ref b) = prep.body {
        lines.push(format!("  --data {} \\", sh_quote(b)));
    }
    if let Some(last) = lines.last_mut() {
        if last.ends_with(" \\") {
            last.truncate(last.len() - 2);
        }
    }
    lines.join("\n")
}

fn gen_js_fetch(prep: &PreparedRequest) -> String {
    let mut out = String::from("const headers = {\n");
    for (k, v) in &prep.headers {
        out.push_str(&format!("  \"{}\": \"{}\",\n", dq_escape(k), dq_escape(v)));
    }
    out.push_str("};\n\n");
    out.push_str("const options = {\n");
    out.push_str(&format!("  method: \"{}\",\n", prep.method));
    out.push_str("  headers,\n");
    if let Some(ref b) = prep.body {
        out.push_str(&format!("  body: \"{}\",\n", dq_escape(b)));
    }
    out.push_str("};\n\n");
    out.push_str(&format!(
        "const response = await fetch(\"{}\", options);\nconst data = await response.text();\nconsole.log(response.status, data);",
        dq_escape(&prep.full_url)
    ));
    out
}

fn gen_js_axios(prep: &PreparedRequest) -> String {
    let mut out = String::from("import axios from \"axios\";\n\nconst response = await axios({\n");
    out.push_str(&format!("  method: \"{}\",\n", prep.method.to_ascii_lowercase()));
    out.push_str(&format!("  url: \"{}\",\n", dq_escape(&prep.full_url)));
    if !prep.headers.is_empty() {
        out.push_str("  headers: {\n");
        for (k, v) in &prep.headers {
            out.push_str(&format!("    \"{}\": \"{}\",\n", dq_escape(k), dq_escape(v)));
        }
        out.push_str("  },\n");
    }
    if let Some(ref b) = prep.body {
        out.push_str(&format!("  data: \"{}\",\n", dq_escape(b)));
    }
    out.push_str("});\nconsole.log(response.status, response.data);");
    out
}

fn gen_python(prep: &PreparedRequest) -> String {
    let mut out = String::from("import requests\n\n");
    out.push_str(&format!("url = \"{}\"\n", dq_escape(&prep.full_url)));
    out.push_str("headers = {\n");
    for (k, v) in &prep.headers {
        out.push_str(&format!("    \"{}\": \"{}\",\n", dq_escape(k), dq_escape(v)));
    }
    out.push_str("}\n");
    if let Some(ref b) = prep.body {
        out.push_str(&format!("payload = \"{}\"\n", dq_escape(b)));
        out.push_str(&format!(
            "response = requests.request(\"{}\", url, headers=headers, data=payload)\n",
            prep.method
        ));
    } else {
        out.push_str(&format!(
            "response = requests.request(\"{}\", url, headers=headers)\n",
            prep.method
        ));
    }
    out.push_str("print(response.status_code, response.text)");
    out
}

fn gen_rust(prep: &PreparedRequest) -> String {
    let mut out = String::from(
        "#[tokio::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {\n    let client = reqwest::Client::new();\n",
    );
    out.push_str(&format!(
        "    let res = client\n        .request(reqwest::Method::{}, \"{}\")\n",
        prep.method,
        dq_escape(&prep.full_url)
    ));
    for (k, v) in &prep.headers {
        out.push_str(&format!(
            "        .header(\"{}\", \"{}\")\n",
            dq_escape(k),
            dq_escape(v)
        ));
    }
    if let Some(ref b) = prep.body {
        out.push_str(&format!("        .body(\"{}\")\n", dq_escape(b)));
    }
    out.push_str(
        "        .send()\n        .await?;\n    println!(\"{} {}\", res.status(), res.text().await?);\n    Ok(())\n}",
    );
    out
}

fn gen_go(prep: &PreparedRequest) -> String {
    let body_expr = match &prep.body {
        Some(b) => format!("strings.NewReader(\"{}\")", dq_escape(b)),
        None => "nil".to_string(),
    };
    let mut imports = vec!["\"fmt\"", "\"io\"", "\"net/http\""];
    if prep.body.is_some() {
        imports.push("\"strings\"");
    }
    let mut out = format!("package main\n\nimport (\n    {}\n)\n\nfunc main() {{\n", imports.join("\n    "));
    out.push_str(&format!(
        "    req, err := http.NewRequest(\"{}\", \"{}\", {body_expr})\n    if err != nil {{ panic(err) }}\n",
        prep.method,
        dq_escape(&prep.full_url)
    ));
    for (k, v) in &prep.headers {
        out.push_str(&format!(
            "    req.Header.Add(\"{}\", \"{}\")\n",
            dq_escape(k),
            dq_escape(v)
        ));
    }
    out.push_str(
        "    resp, err := http.DefaultClient.Do(req)\n    if err != nil { panic(err) }\n    defer resp.Body.Close()\n    body, _ := io.ReadAll(resp.Body)\n    fmt.Println(resp.StatusCode, string(body))\n}",
    );
    out
}

fn gen_http_raw(prep: &PreparedRequest) -> String {
    let parsed = url::Url::parse(&prep.full_url).ok();
    let path_q = parsed
        .as_ref()
        .map(|u| match u.query() {
            Some(q) => format!("{}?{q}", u.path()),
            None => u.path().to_string(),
        })
        .unwrap_or_else(|| "/".to_string());
    let host = parsed
        .as_ref()
        .and_then(|u| {
            u.host_str().map(|h| match u.port() {
                Some(p) => format!("{h}:{p}"),
                None => h.to_string(),
            })
        })
        .unwrap_or_else(|| "localhost".to_string());

    let mut lines = vec![format!("{} {} HTTP/1.1", prep.method, path_q)];
    if !prep.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("host")) {
        lines.push(format!("Host: {host}"));
    }
    for (k, v) in &prep.headers {
        lines.push(format!("{k}: {v}"));
    }
    lines.push(String::new());
    if let Some(ref b) = prep.body {
        lines.push(b.clone());
    }
    lines.join("\n")
}
