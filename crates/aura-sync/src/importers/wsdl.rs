//! WSDL 1.1 / SOAP service importer generating ready-to-run SOAP AuraRequests.

use anyhow::{anyhow, Result};
use aura_core::model::{
    AuraRequest, AuthConfig, BodyType, Collection, HeaderItem, HttpMethod, RequestProtocol,
};
use aura_core::soap::{soap_headers, wrap_soap_envelope, SoapVersion};
use std::collections::HashMap;

pub fn import_wsdl(wsdl_xml: &str) -> Result<Collection> {
    let doc = roxmltree::Document::parse(wsdl_xml)
        .map_err(|e| anyhow!("Invalid WSDL XML specification: {e}"))?;

    let root = doc.root_element();
    let target_ns = root
        .attribute("targetNamespace")
        .unwrap_or("http://tempuri.org/");
    let service_name = root
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "service")
        .and_then(|n| n.attribute("name"))
        .or_else(|| root.attribute("name"))
        .unwrap_or("SOAP Service")
        .to_string();

    // Detect endpoint URL & SOAP version from <port><address location="...">
    let mut endpoint_url = "https://example.com/soap".to_string();
    let mut default_soap_version = SoapVersion::Soap11;
    for node in doc.descendants().filter(|n| n.is_element()) {
        if node.tag_name().name() == "address" {
            if let Some(loc) = node.attribute("location") {
                if !loc.trim().is_empty() {
                    endpoint_url = loc.trim().to_string();
                }
            }
            if let Some(ns) = node.tag_name().namespace() {
                if ns.contains("soap12") {
                    default_soap_version = SoapVersion::Soap12;
                }
            }
            break;
        }
    }

    // Map binding operation name -> (soapAction, SoapVersion)
    let mut op_actions: HashMap<String, (Option<String>, SoapVersion)> = HashMap::new();
    for binding in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "binding")
    {
        for op in binding
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "operation")
        {
            let Some(op_name) = op.attribute("name") else {
                continue;
            };
            let mut action = None;
            let mut ver = default_soap_version;
            for child in op.children().filter(|n| n.is_element()) {
                if child.tag_name().name() == "operation" {
                    if let Some(ns) = child.tag_name().namespace() {
                        if ns.contains("soap12") {
                            ver = SoapVersion::Soap12;
                        }
                    }
                    if let Some(sa) = child.attribute("soapAction") {
                        action = Some(sa.to_string());
                    }
                }
            }
            op_actions.insert(op_name.to_string(), (action, ver));
        }
    }

    // Collect XSD complexType/element child field hints if defined inline in <types>
    let mut element_fields: HashMap<String, Vec<String>> = HashMap::new();
    for elem in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "element")
    {
        if let Some(ename) = elem.attribute("name") {
            let children: Vec<String> = elem
                .descendants()
                .filter(|c| c.is_element() && c.tag_name().name() == "element" && *c != elem)
                .filter_map(|c| c.attribute("name").map(ToString::to_string))
                .collect();
            if !children.is_empty() {
                element_fields.insert(ename.to_string(), children);
            }
        }
    }

    // Collect <message> parts as fallback parameters
    let mut messages: HashMap<String, Vec<String>> = HashMap::new();
    for msg in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "message")
    {
        if let Some(mname) = msg.attribute("name") {
            let parts: Vec<String> = msg
                .children()
                .filter(|c| c.is_element() && c.tag_name().name() == "part")
                .filter_map(|c| {
                    c.attribute("element")
                        .or_else(|| c.attribute("name"))
                        .map(|s| s.rsplit(':').next().unwrap_or(s).to_string())
                })
                .collect();
            messages.insert(mname.to_string(), parts);
        }
    }

    // Build one request per <portType><operation>
    let mut requests = Vec::new();
    let mut seen_ops = std::collections::HashSet::new();
    for pt in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "portType")
    {
        for op in pt
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "operation")
        {
            let Some(op_name) = op.attribute("name") else {
                continue;
            };
            if !seen_ops.insert(op_name.to_string()) {
                continue;
            }

            let input_msg = op
                .children()
                .find(|n| n.is_element() && n.tag_name().name() == "input")
                .and_then(|n| n.attribute("message"))
                .map(|s| s.rsplit(':').next().unwrap_or(s).to_string());

            let fields = element_fields
                .get(op_name)
                .or_else(|| element_fields.get(&format!("{op_name}Request")))
                .cloned()
                .or_else(|| {
                    input_msg.as_ref().and_then(|m| {
                        messages.get(m).and_then(|parts| {
                            if parts.len() == 1 {
                                element_fields.get(&parts[0]).cloned().or_else(|| Some(parts.clone()))
                            } else {
                                Some(parts.clone())
                            }
                        })
                    })
                })
                .unwrap_or_default();

            let inner_xml = if fields.is_empty() {
                String::new()
            } else {
                let child_tags: String = fields
                    .iter()
                    .map(|f| format!("  <{f}>?</{f}>"))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("<{op_name} xmlns=\"{target_ns}\">\n{child_tags}\n</{op_name}>")
            };

            let (soap_action, soap_ver) = op_actions
                .get(op_name)
                .cloned()
                .unwrap_or((None, default_soap_version));
            let envelope = wrap_soap_envelope(soap_ver, op_name, target_ns, &inner_xml, None);

            let mut req = AuraRequest::new(op_name, HttpMethod::POST, &endpoint_url);
            req.protocol = RequestProtocol::Soap;
            req.body = BodyType::Xml(envelope);
            for (hk, hv) in soap_headers(soap_ver, soap_action.as_deref()) {
                req.headers.push(HeaderItem::new(hk, hv));
            }
            requests.push(req);
        }
    }

    if requests.is_empty() {
        return Err(anyhow!("No SOAP operations found in WSDL document"));
    }

    Ok(Collection {
        id: uuid::Uuid::new_v4().to_string(),
        name: service_name,
        auth: AuthConfig::None,
        variables: HashMap::new(),
        requests,
        folders: Vec::new(),
    })
}
