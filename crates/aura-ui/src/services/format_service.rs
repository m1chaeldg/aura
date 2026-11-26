//! Format Service: Offloads JSON/XML pretty-printing, minification, and language detection.

use crate::stores::ResponseBodyMode;
use anyhow::{anyhow, Result};

/// Detected or overridden language for response body rendering and formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResponseLanguage {
    #[default]
    Json,
    Xml,
    Html,
    Text,
}

impl ResponseLanguage {
    pub fn label(self) -> &'static str {
        match self {
            Self::Json => "JSON",
            Self::Xml => "XML",
            Self::Html => "HTML",
            Self::Text => "Text",
        }
    }

    pub fn tree_sitter_language(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Xml | Self::Html => "html",
            Self::Text => "text",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Json => Self::Xml,
            Self::Xml => Self::Html,
            Self::Html => Self::Text,
            Self::Text => Self::Json,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FormatService;

impl FormatService {
    pub fn new() -> Self {
        Self
    }

    /// Detects response language from the `Content-Type` header first, then sniffs the body.
    pub fn detect_language(body: &str, content_type: Option<&str>) -> ResponseLanguage {
        if let Some(ct) = content_type {
            let ct_lower = ct.to_ascii_lowercase();
            if ct_lower.contains("json") {
                return ResponseLanguage::Json;
            }
            if ct_lower.contains("html") {
                return ResponseLanguage::Html;
            }
            if ct_lower.contains("xml") {
                return ResponseLanguage::Xml;
            }
        }

        let trimmed = body.trim();
        if trimmed.is_empty() {
            return ResponseLanguage::Text;
        }
        if (trimmed.starts_with('{') || trimmed.starts_with('['))
            && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
        {
            return ResponseLanguage::Json;
        }
        let lower_prefix: String = trimmed.chars().take(32).collect::<String>().to_ascii_lowercase();
        if lower_prefix.starts_with("<!doctype html") || lower_prefix.starts_with("<html") {
            return ResponseLanguage::Html;
        }
        if trimmed.starts_with('<') && trimmed.ends_with('>') {
            return ResponseLanguage::Xml;
        }
        ResponseLanguage::Text
    }

    /// Format JSON string pretty.
    pub fn format_json(raw: &str) -> Result<String> {
        let val: serde_json::Value = serde_json::from_str(raw)?;
        Ok(serde_json::to_string_pretty(&val)?)
    }

    /// Compress (minify) JSON string.
    pub fn compress_json(raw: &str) -> Result<String> {
        let val: serde_json::Value = serde_json::from_str(raw)?;
        Ok(serde_json::to_string(&val)?)
    }

    /// Pretty-print an XML/HTML document with 2-space indentation.
    pub fn format_xml(raw: &str) -> Result<String> {
        let tokens = tokenize_xml(raw)?;
        let mut out = String::with_capacity(raw.len() + 64);
        let mut indent: usize = 0;
        let mut i = 0;

        while i < tokens.len() {
            match &tokens[i] {
                XmlToken::Decl(t) | XmlToken::Comment(t) | XmlToken::CData(t) => {
                    push_indent(&mut out, indent);
                    out.push_str(t);
                    out.push('\n');
                }
                XmlToken::SelfClosing(t) => {
                    push_indent(&mut out, indent);
                    out.push_str(t);
                    out.push('\n');
                }
                XmlToken::Open(t) => {
                    // Keep `<tag>inline text</tag>` on a single line.
                    if let (Some(XmlToken::Text(txt)), Some(XmlToken::Close(close))) =
                        (tokens.get(i + 1), tokens.get(i + 2))
                    {
                        push_indent(&mut out, indent);
                        out.push_str(t);
                        out.push_str(txt);
                        out.push_str(close);
                        out.push('\n');
                        i += 3;
                        continue;
                    }
                    push_indent(&mut out, indent);
                    out.push_str(t);
                    out.push('\n');
                    indent += 1;
                }
                XmlToken::Close(t) => {
                    indent = indent.saturating_sub(1);
                    push_indent(&mut out, indent);
                    out.push_str(t);
                    out.push('\n');
                }
                XmlToken::Text(t) => {
                    push_indent(&mut out, indent);
                    out.push_str(t);
                    out.push('\n');
                }
            }
            i += 1;
        }

        Ok(out.trim_end().to_string())
    }

    /// Compress (minify) an XML/HTML document by stripping inter-tag whitespace.
    pub fn compress_xml(raw: &str) -> Result<String> {
        let tokens = tokenize_xml(raw)?;
        let mut out = String::with_capacity(raw.len());
        for token in tokens {
            match token {
                XmlToken::Decl(t)
                | XmlToken::Comment(t)
                | XmlToken::CData(t)
                | XmlToken::SelfClosing(t)
                | XmlToken::Open(t)
                | XmlToken::Close(t)
                | XmlToken::Text(t) => out.push_str(&t),
            }
        }
        Ok(out)
    }

    /// Formats a response body according to `lang` and `mode`, falling back to `body` on parse error.
    pub fn format_response_body(
        body: &str,
        lang: ResponseLanguage,
        mode: ResponseBodyMode,
    ) -> String {
        match (lang, mode) {
            (_, ResponseBodyMode::Raw) | (ResponseLanguage::Text, _) => body.to_string(),
            (ResponseLanguage::Json, ResponseBodyMode::Pretty) => {
                Self::format_json(body).unwrap_or_else(|_| body.to_string())
            }
            (ResponseLanguage::Json, ResponseBodyMode::Compress) => {
                Self::compress_json(body).unwrap_or_else(|_| body.to_string())
            }
            (ResponseLanguage::Xml | ResponseLanguage::Html, ResponseBodyMode::Pretty) => {
                Self::format_xml(body).unwrap_or_else(|_| body.to_string())
            }
            (ResponseLanguage::Xml | ResponseLanguage::Html, ResponseBodyMode::Compress) => {
                Self::compress_xml(body).unwrap_or_else(|_| body.to_string())
            }
        }
    }
}

#[derive(Debug)]
enum XmlToken {
    Decl(String),
    Comment(String),
    CData(String),
    Open(String),
    Close(String),
    SelfClosing(String),
    Text(String),
}

fn push_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn tokenize_xml(raw: &str) -> Result<Vec<XmlToken>> {
    let trimmed = raw.trim();
    if !trimmed.starts_with('<') {
        return Err(anyhow!("Not valid XML: expected '<'"));
    }

    let mut tokens = Vec::new();
    let mut rest = trimmed;

    while !rest.is_empty() {
        if rest.starts_with("<!--") {
            let end = rest
                .find("-->")
                .ok_or_else(|| anyhow!("Unterminated XML comment"))?;
            tokens.push(XmlToken::Comment(rest[..end + 3].to_string()));
            rest = &rest[end + 3..];
        } else if rest.starts_with("<![CDATA[") {
            let end = rest
                .find("]]>")
                .ok_or_else(|| anyhow!("Unterminated CDATA section"))?;
            tokens.push(XmlToken::CData(rest[..end + 3].to_string()));
            rest = &rest[end + 3..];
        } else if rest.starts_with('<') {
            let end = rest
                .find('>')
                .ok_or_else(|| anyhow!("Unterminated XML tag"))?;
            let tag = &rest[..=end];
            if tag.starts_with("<?") || tag.starts_with("<!") {
                tokens.push(XmlToken::Decl(tag.to_string()));
            } else if tag.starts_with("</") {
                tokens.push(XmlToken::Close(tag.to_string()));
            } else if tag.ends_with("/>") {
                tokens.push(XmlToken::SelfClosing(tag.to_string()));
            } else {
                tokens.push(XmlToken::Open(tag.to_string()));
            }
            rest = &rest[end + 1..];
        } else if let Some(next_tag) = rest.find('<') {
            let text = rest[..next_tag].trim();
            if !text.is_empty() {
                tokens.push(XmlToken::Text(text.to_string()));
            }
            rest = &rest[next_tag..];
        } else {
            let text = rest.trim();
            if !text.is_empty() {
                tokens.push(XmlToken::Text(text.to_string()));
            }
            break;
        }
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stores::ResponseBodyMode;

    #[test]
    fn test_detect_language_from_content_type_and_body() {
        assert_eq!(
            FormatService::detect_language(r#"{"a":1}"#, Some("application/json; charset=utf-8")),
            ResponseLanguage::Json
        );
        assert_eq!(
            FormatService::detect_language("<root><item>1</item></root>", Some("application/xml")),
            ResponseLanguage::Xml
        );
        assert_eq!(
            FormatService::detect_language("<html><body>Hi</body></html>", Some("text/html")),
            ResponseLanguage::Html
        );
        // Sniffing when content-type is absent
        assert_eq!(
            FormatService::detect_language("  [1, 2, 3] ", None),
            ResponseLanguage::Json
        );
        assert_eq!(
            FormatService::detect_language("<?xml version=\"1.0\"?><a/>", None),
            ResponseLanguage::Xml
        );
        assert_eq!(
            FormatService::detect_language("<!DOCTYPE html><html></html>", None),
            ResponseLanguage::Html
        );
        assert_eq!(
            FormatService::detect_language("plain text response", None),
            ResponseLanguage::Text
        );
    }

    #[test]
    fn test_compress_json_minifies_whitespace() {
        let pretty = "{\n  \"name\": \"Aura\",\n  \"items\": [\n    1,\n    2\n  ]\n}";
        let compressed = FormatService::compress_json(pretty).expect("compress valid json");
        assert_eq!(compressed, r#"{"name":"Aura","items":[1,2]}"#);
    }

    #[test]
    fn test_format_and_compress_xml() {
        let compact = "<?xml version=\"1.0\"?><users><user id=\"1\"><name>Ada</name><active/></user></users>";
        let pretty = FormatService::format_xml(compact).expect("format xml");
        assert!(pretty.contains("<?xml version=\"1.0\"?>"));
        assert!(pretty.contains("  <user id=\"1\">"));
        assert!(pretty.contains("    <name>Ada</name>"));
        assert!(pretty.contains("    <active/>"));
        assert!(pretty.contains("</users>"));

        let minified = FormatService::compress_xml(&pretty).expect("compress xml");
        assert_eq!(minified, compact);
    }

    #[test]
    fn test_format_response_body_modes_and_fallback() {
        let raw_json = " { \"ok\" : true } ";
        assert_eq!(
            FormatService::format_response_body(
                raw_json,
                ResponseLanguage::Json,
                ResponseBodyMode::Pretty
            ),
            "{\n  \"ok\": true\n}"
        );
        assert_eq!(
            FormatService::format_response_body(
                raw_json,
                ResponseLanguage::Json,
                ResponseBodyMode::Compress
            ),
            r#"{"ok":true}"#
        );
        assert_eq!(
            FormatService::format_response_body(
                raw_json,
                ResponseLanguage::Json,
                ResponseBodyMode::Raw
            ),
            raw_json
        );
        // Graceful fallback on invalid JSON
        let bad_json = "{not valid json";
        assert_eq!(
            FormatService::format_response_body(
                bad_json,
                ResponseLanguage::Json,
                ResponseBodyMode::Pretty
            ),
            bad_json
        );
    }
}
