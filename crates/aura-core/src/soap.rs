//! SOAP 1.1 / 1.2 envelope helpers and fault inspection.

use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoapVersion {
    Soap11,
    Soap12,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoapFault {
    pub code: String,
    pub reason: String,
    pub detail: Option<String>,
}

pub fn wrap_soap_envelope(
    version: SoapVersion,
    operation: &str,
    target_namespace: &str,
    body_inner_xml: &str,
    header_xml: Option<&str>,
) -> String {
    let envelope_ns = match version {
        SoapVersion::Soap11 => "http://schemas.xmlsoap.org/soap/envelope/",
        SoapVersion::Soap12 => "http://www.w3.org/2003/05/soap-envelope",
    };

    let header_block = match header_xml.map(str::trim).filter(|s| !s.is_empty()) {
        Some(h) => format!("  <soap:Header>\n    {h}\n  </soap:Header>\n"),
        None => "  <soap:Header/>\n".to_string(),
    };

    let inner_trimmed = body_inner_xml.trim();
    let body_content = if inner_trimmed.is_empty() && !operation.trim().is_empty() {
        if target_namespace.trim().is_empty() {
            format!("    <{operation}/>")
        } else {
            format!("    <{operation} xmlns=\"{target_namespace}\"/>")
        }
    } else if !operation.trim().is_empty()
        && !inner_trimmed.starts_with('<')
    {
        if target_namespace.trim().is_empty() {
            format!("    <{operation}>{inner_trimmed}</{operation}>")
        } else {
            format!("    <{operation} xmlns=\"{target_namespace}\">{inner_trimmed}</{operation}>")
        }
    } else {
        inner_trimmed
            .lines()
            .map(|l| format!("    {l}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<soap:Envelope xmlns:soap=\"{envelope_ns}\">\n{header_block}  <soap:Body>\n{body_content}\n  </soap:Body>\n</soap:Envelope>"
    )
}

pub fn soap_headers(version: SoapVersion, soap_action: Option<&str>) -> Vec<(String, String)> {
    match version {
        SoapVersion::Soap11 => {
            let mut h = vec![(
                "Content-Type".to_string(),
                "text/xml; charset=utf-8".to_string(),
            )];
            if let Some(action) = soap_action {
                let quoted = if action.starts_with('"') {
                    action.to_string()
                } else {
                    format!("\"{action}\"")
                };
                h.push(("SOAPAction".to_string(), quoted));
            }
            h
        }
        SoapVersion::Soap12 => {
            let ct = match soap_action.filter(|s| !s.trim().is_empty()) {
                Some(action) => format!(
                    "application/soap+xml; charset=utf-8; action=\"{}\"",
                    action.trim_matches('"')
                ),
                None => "application/soap+xml; charset=utf-8".to_string(),
            };
            vec![("Content-Type".to_string(), ct)]
        }
    }
}

pub fn is_soap_envelope(xml: &str) -> bool {
    let trimmed = xml.trim();
    trimmed.contains(":Envelope") || trimmed.contains("<Envelope")
}

pub fn extract_soap_body(xml: &str) -> Result<String> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| anyhow!("Invalid SOAP XML document: {e}"))?;
    let body_node = doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "Body")
        .ok_or_else(|| anyhow!("No SOAP <Body> element found"))?;

    let range = body_node.range();
    let snippet = &xml[range];
    if let (Some(open_end), Some(close_start)) = (snippet.find('>'), snippet.rfind("</")) {
        Ok(snippet[open_end + 1..close_start].trim().to_string())
    } else {
        Ok(String::new())
    }
}

pub fn parse_soap_fault(xml: &str) -> Option<SoapFault> {
    let doc = roxmltree::Document::parse(xml).ok()?;
    let fault = doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "Fault")?;

    let find_text = |names: &[&str]| -> Option<String> {
        fault
            .descendants()
            .find(|n| n.is_element() && names.contains(&n.tag_name().name()))
            .and_then(|n| {
                n.descendants()
                    .filter(|d| d.is_text())
                    .map(|d| d.text().unwrap_or(""))
                    .collect::<String>()
                    .trim()
                    .to_string()
                    .into()
            })
            .filter(|s: &String| !s.is_empty())
    };

    let code = find_text(&["faultcode", "Code", "Value"]).unwrap_or_else(|| "Fault".to_string());
    let reason =
        find_text(&["faultstring", "Reason", "Text"]).unwrap_or_else(|| "SOAP Fault".to_string());
    let detail = find_text(&["detail", "Detail"]);

    Some(SoapFault {
        code,
        reason,
        detail,
    })
}
