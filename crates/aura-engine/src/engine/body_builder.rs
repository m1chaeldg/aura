//! Request body rendering and `reqwest::RequestBuilder` attachment.

use super::logging::{MAX_LOG_PAYLOAD_LEN, format_kv_for_log, truncate_for_log};
use aura_core::model::BodyType;
use aura_core::templating::TemplateContext;

fn ensure_content_type(header_pairs: &mut Vec<(String, String)>, ct: &str) {
    if !header_pairs
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
    {
        header_pairs.push(("Content-Type".to_string(), ct.to_string()));
    }
}

pub(super) fn attach_request_body(
    body: &BodyType,
    ctx: &TemplateContext,
    mut request_builder: reqwest::RequestBuilder,
    header_pairs: &mut Vec<(String, String)>,
) -> (reqwest::RequestBuilder, Vec<u8>, String) {
    let mut body_bytes = Vec::new();
    let summary = match body {
        BodyType::Json(s) => attach_text_body(
            "JSON",
            "application/json",
            ctx.render(s),
            &mut request_builder,
            header_pairs,
            &mut body_bytes,
        ),
        BodyType::Xml(s) => attach_text_body(
            "XML",
            "application/xml",
            ctx.render(s),
            &mut request_builder,
            header_pairs,
            &mut body_bytes,
        ),
        BodyType::Text(s) => attach_text_body(
            "Text",
            "text/plain",
            ctx.render(s),
            &mut request_builder,
            header_pairs,
            &mut body_bytes,
        ),
        BodyType::Raw {
            content,
            content_type,
        } => {
            let rendered_body = ctx.render(content);
            let rendered_ct = content_type
                .as_deref()
                .map(|c| ctx.render(c))
                .unwrap_or_else(|| "text/plain".to_string());
            let label = format!("Raw[{rendered_ct}]");
            attach_text_body(
                &label,
                &rendered_ct,
                rendered_body,
                &mut request_builder,
                header_pairs,
                &mut body_bytes,
            )
        }
        BodyType::Form(map) => {
            let form_data: Vec<(String, String)> = map
                .iter()
                .map(|(k, v)| (ctx.render(k), ctx.render(v)))
                .collect();
            attach_form_body(
                "Form",
                form_data,
                &mut request_builder,
                &mut body_bytes,
            )
        }
        BodyType::FormUrlEncoded(items) => {
            let form_data: Vec<(String, String)> = items
                .iter()
                .filter(|item| item.enabled)
                .map(|item| (ctx.render(&item.key), ctx.render(&item.value)))
                .collect();
            attach_form_body(
                "FormUrlEncoded",
                form_data,
                &mut request_builder,
                &mut body_bytes,
            )
        }
        BodyType::Binary {
            file_path,
            content_type,
        } => {
            let rendered_path = ctx.render(file_path);
            let rendered_ct = content_type.as_deref().map(|c| ctx.render(c));
            match std::fs::read(&rendered_path) {
                Ok(bytes) => {
                    let len = bytes.len();
                    if let Some(ct) = rendered_ct {
                        ensure_content_type(header_pairs, &ct);
                    }
                    body_bytes = bytes.clone();
                    request_builder = request_builder.body(bytes);
                    format!("Binary (file={rendered_path}, {len} bytes)")
                }
                Err(err) => {
                    tracing::warn!(
                        target: "aura_engine::request",
                        file_path = %rendered_path,
                        error = %err,
                        "Failed to read binary body file"
                    );
                    format!("Binary (file={rendered_path}, read_error={err})")
                }
            }
        }
        BodyType::Multipart(fields) => {
            let mut form = reqwest::multipart::Form::new();
            let mut field_summaries = Vec::new();
            for f in fields {
                let field_name = ctx.render(&f.name);
                if let Some(v) = &f.value {
                    let rendered = ctx.render(v);
                    field_summaries.push(format!(
                        "{field_name}={}",
                        truncate_for_log(&rendered, 128)
                    ));
                    form = form.text(field_name, rendered);
                } else if let Some(path) = &f.file_path {
                    let rendered_path = ctx.render(path);
                    match std::fs::read(&rendered_path) {
                        Ok(bytes) => {
                            field_summaries.push(format!(
                                "{field_name}=<file:{rendered_path} ({} bytes)>",
                                bytes.len()
                            ));
                            form = form.part(field_name, reqwest::multipart::Part::bytes(bytes));
                        }
                        Err(err) => {
                            tracing::warn!(
                                target: "aura_engine::request",
                                field = %field_name,
                                file_path = %rendered_path,
                                error = %err,
                                "Failed to read multipart file"
                            );
                            field_summaries.push(format!(
                                "{field_name}=<file:{rendered_path} read_error={err}>"
                            ));
                        }
                    }
                }
            }
            request_builder = request_builder.multipart(form);
            format!("Multipart [{}]", field_summaries.join(", "))
        }
        BodyType::GraphQL { query, variables } => {
            let rendered_query = ctx.render(query);
            let rendered_vars = ctx.render(variables);
            let vars_json: serde_json::Value = if rendered_vars.trim().is_empty() {
                serde_json::json!({})
            } else {
                serde_json::from_str(&rendered_vars).unwrap_or_else(|_| serde_json::json!({}))
            };
            let payload = serde_json::json!({
                "query": rendered_query,
                "variables": vars_json,
            })
            .to_string();
            attach_text_body(
                "GraphQL",
                "application/json",
                payload,
                &mut request_builder,
                header_pairs,
                &mut body_bytes,
            )
        }
        BodyType::None => "None".to_string(),
    };

    (request_builder, body_bytes, summary)
}

fn attach_text_body(
    label: &str,
    default_ct: &str,
    rendered_body: String,
    request_builder: &mut reqwest::RequestBuilder,
    header_pairs: &mut Vec<(String, String)>,
    body_bytes: &mut Vec<u8>,
) -> String {
    let summary = format!(
        "{label} ({} bytes): {}",
        rendered_body.len(),
        truncate_for_log(&rendered_body, MAX_LOG_PAYLOAD_LEN)
    );
    ensure_content_type(header_pairs, default_ct);
    *body_bytes = rendered_body.as_bytes().to_vec();
    let rb = std::mem::replace(
        request_builder,
        reqwest::Client::new().get("http://localhost"),
    );
    *request_builder = rb.body(rendered_body);
    summary
}

fn attach_form_body(
    label: &str,
    form_data: Vec<(String, String)>,
    request_builder: &mut reqwest::RequestBuilder,
    body_bytes: &mut Vec<u8>,
) -> String {
    let summary = format!(
        "{label} ({} fields): {}",
        form_data.len(),
        format_kv_for_log(&form_data, false)
    );
    let encoded = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(form_data.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .finish();
    *body_bytes = encoded.as_bytes().to_vec();
    let rb = std::mem::replace(
        request_builder,
        reqwest::Client::new().get("http://localhost"),
    );
    *request_builder = rb.form(&form_data);
    summary
}
