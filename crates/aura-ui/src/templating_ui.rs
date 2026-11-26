//! UI-layer `{{...}}` template resolution, color coding, inline tokens, and hover tooltips.

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use aura_core::templating::TemplateContext;
use gpui::{
    AnyElement, App, Div, ElementId, HighlightStyle, Hsla, InteractiveElement as _,
    IntoElement, ParentElement as _, SharedString, Stateful, StatefulInteractiveElement as _,
    Styled as _, Task, UnderlineStyle, Window, div, px,
};
use gpui_kit::component::input::{
    HoverProvider, InlineToken, InlineTokenContext, InputContent, InputState, Rope, RopeExt as _,
    TextDecoration, TextDecorationCollection,
};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{ActiveTheme as _, h_flex};

use crate::theme::{template_resolved_color, template_unresolved_color};

/// Origin tier where a template variable was resolved from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateSource {
    Request,
    Environment,
    Collection,
    SystemEnv,
}

impl TemplateSource {
    /// Human-readable label shown in hover tooltips.
    pub fn label(self) -> &'static str {
        match self {
            Self::Request => "Request Variable",
            Self::Environment => "Active Environment",
            Self::Collection => "Collection Variable",
            Self::SystemEnv => "OS Environment",
        }
    }
}

/// Resolved template variable value together with its resolution tier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTemplate {
    pub value: String,
    pub source: TemplateSource,
}

/// Parsed `{{...}}` span within a UI string or buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateSpan {
    /// UTF-8 byte range covering `{{...}}`.
    pub range: Range<usize>,
    /// Exact original text slice matching `range`.
    pub raw: String,
    /// Trimmed variable identifier inside braces.
    pub key: String,
    /// Resolved value + source tier if resolvable, or `None` if unresolved.
    pub resolution: Option<ResolvedTemplate>,
}

/// Resolves a variable name using Aura's cascade order while reporting the source tier.
pub fn resolve_template_with_source(
    key: &str,
    ctx: &TemplateContext,
) -> Option<ResolvedTemplate> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(val) = ctx.request_vars.get(trimmed) {
        return Some(ResolvedTemplate {
            value: val.clone(),
            source: TemplateSource::Request,
        });
    }
    if let Some(val) = ctx.environment_vars.get(trimmed) {
        return Some(ResolvedTemplate {
            value: val.clone(),
            source: TemplateSource::Environment,
        });
    }
    if let Some(val) = ctx.collection_vars.get(trimmed) {
        return Some(ResolvedTemplate {
            value: val.clone(),
            source: TemplateSource::Collection,
        });
    }
    if let Ok(val) = std::env::var(trimmed) {
        return Some(ResolvedTemplate {
            value: val,
            source: TemplateSource::SystemEnv,
        });
    }
    None
}

/// Scans `text` for non-overlapping single-line `{{...}}` template expressions.
pub fn parse_template_spans(text: &str, ctx: &TemplateContext) -> Vec<TemplateSpan> {
    let mut spans = Vec::new();
    let mut cursor = 0;

    while let Some(rel_start) = text[cursor..].find("{{") {
        let start = cursor + rel_start;
        let after_open = start + 2;

        // Find the closing `}}` before any newline so malformed multi-line braces don't eat whole files.
        let rest = &text[after_open..];
        let newline_limit = rest.find(['\n', '\r']).unwrap_or(rest.len());
        let search_slice = &rest[..newline_limit];

        if let Some(rel_end) = search_slice.find("}}") {
            let end = after_open + rel_end + 2;
            let raw = text[start..end].to_string();
            let key = search_slice[..rel_end].trim().to_string();
            let resolution = resolve_template_with_source(&key, ctx);
            spans.push(TemplateSpan {
                range: start..end,
                raw,
                key,
                resolution,
            });
            cursor = end;
        } else {
            cursor = after_open + newline_limit;
        }
    }

    spans
}

fn truncate_tooltip_value(val: &str, max_chars: usize) -> String {
    let mut chars = val.chars();
    let prefix: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}

/// Formats a concise tooltip string for a resolved or unresolved template span.
pub fn tooltip_text_for_span(span: &TemplateSpan) -> String {
    match &span.resolution {
        Some(res) => {
            let display_val = if res.value.is_empty() {
                "<empty string>".to_string()
            } else {
                truncate_tooltip_value(&res.value, 240)
            };
            format!("{} = {} ({})", span.raw, display_val, res.source.label())
        }
        None => {
            if span.key.is_empty() {
                "Empty template expression {{}}".to_string()
            } else {
                format!(
                    "Unresolved template: {} (not set in Request, Environment, Collection, or OS env)",
                    span.raw
                )
            }
        }
    }
}

/// Encodes span metadata in an `InlineToken` ID so custom token renderers stay self-contained.
pub fn encode_token_id(idx: usize, span: &TemplateSpan) -> String {
    match &span.resolution {
        Some(res) => format!(
            "1|{idx}|{}|{}|{}",
            res.source.label(),
            span.key,
            res.value
        ),
        None => format!("0|{idx}||{}|", span.key),
    }
}

/// Decoded token state extracted from `encode_token_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedTokenMeta {
    pub is_resolved: bool,
    pub source_label: String,
    pub key: String,
    pub value: String,
}

/// Decodes token metadata produced by `encode_token_id`.
pub fn decode_token_id(id: &str) -> Option<DecodedTokenMeta> {
    let mut parts = id.splitn(5, '|');
    let flag = parts.next()?;
    let _idx = parts.next()?;
    let source_label = parts.next()?.to_string();
    let key = parts.next()?.to_string();
    let value = parts.next()?.to_string();
    Some(DecodedTokenMeta {
        is_resolved: flag == "1",
        source_label,
        key,
        value,
    })
}

/// Synchronizes `InputState` inline tokens to reflect live `{{...}}` resolution.
pub fn sync_input_template_tokens(
    state: &Entity<InputState>,
    ctx: &TemplateContext,
    cx: &mut App,
) {
    let text = state.read(cx).value().to_string();
    let spans = parse_template_spans(&text, ctx);

    let desired: Vec<(Range<usize>, String)> = spans
        .iter()
        .enumerate()
        .map(|(idx, s)| (s.range.clone(), encode_token_id(idx, s)))
        .collect();

    let unchanged = {
        let existing = state.read(cx).tokens();
        existing.len() == desired.len()
            && existing
                .iter()
                .zip(desired.iter())
                .all(|(cur, (range, id))| cur.range() == *range && cur.token().id() == id.as_str())
    };
    if unchanged {
        return;
    }

    let mut content = InputContent::new(text);
    for (idx, span) in spans.into_iter().enumerate() {
        let id = encode_token_id(idx, &span);
        if let Ok(next) = content.with_token(
            span.range,
            InlineToken::new(id, span.raw.clone()).with_label(span.raw),
        ) {
            content = next;
        } else {
            return;
        }
    }

    // `SplitsSelection` can occur transiently while the caret moves across or types inside braces;
    // ignore cleanly and re-sync on subsequent selection/input changes.
    let _ = state.update(cx, |s, cx| s.set_tokens(content, cx));
}

/// Renders an `InlineToken` inside a GPUI Kit `Input` using green/red color coding and a hover tooltip.
pub fn render_template_token(
    token_ctx: &InlineTokenContext,
    _window: &mut Window,
    cx: &mut App,
) -> Stateful<Div> {
    let meta = decode_token_id(token_ctx.token().id());
    let is_resolved = meta.as_ref().is_some_and(|m| m.is_resolved);
    let accent = if is_resolved {
        template_resolved_color(cx)
    } else {
        template_unresolved_color(cx)
    };

    let raw_text = token_ctx.token().text().to_string();
    let tip: SharedString = match meta {
        Some(m) if m.is_resolved => {
            let display_val = if m.value.is_empty() {
                "<empty string>".to_string()
            } else {
                truncate_tooltip_value(&m.value, 240)
            };
            format!("{raw_text} = {display_val} ({})", m.source_label).into()
        }
        Some(m) if m.key.is_empty() => "Empty template expression {{}}".into(),
        _ => format!(
            "Unresolved template: {raw_text} (not set in Request, Environment, Collection, or OS env)"
        )
        .into(),
    };

    div()
        .id(SharedString::from(format!(
            "tpl-tok-{}",
            token_ctx.token().id()
        )))
        .flex()
        .items_center()
        .px(px(2.))
        .rounded(px(3.))
        .bg(accent.opacity(0.14))
        .border_1()
        .border_color(accent.opacity(0.38))
        .text_color(accent)
        .font_family(cx.theme().mono_font_family.clone())
        .cursor_pointer()
        .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
        .child(raw_text)
}

/// Updates text decorations on an `EditorState` so all `{{...}}` expressions appear green or red.
pub fn sync_editor_template_decorations(
    state: &Entity<EditorState>,
    collection: &mut Option<TextDecorationCollection>,
    ctx: &TemplateContext,
    cx: &mut App,
) {
    let text = state.read(cx).value().to_string();
    let spans = parse_template_spans(&text, ctx);

    if spans.is_empty() {
        if let Some(col) = collection.as_ref() {
            col.clear(cx);
        }
        return;
    }

    let resolved_color = template_resolved_color(cx);
    let unresolved_color = template_unresolved_color(cx);

    let decorations: Vec<TextDecoration> = spans
        .into_iter()
        .map(|span| {
            let is_resolved = span.resolution.is_some();
            let color = if is_resolved {
                resolved_color
            } else {
                unresolved_color
            };
            TextDecoration::new(
                span.range,
                HighlightStyle {
                    color: Some(color),
                    underline: Some(UnderlineStyle {
                        thickness: px(1.),
                        color: Some(color),
                        wavy: !is_resolved,
                    }),
                    ..Default::default()
                },
            )
        })
        .collect();

    if let Some(col) = collection.as_ref() {
        col.set(decorations, cx);
    } else {
        let col = state.update(cx, |editor, cx| {
            editor.create_decorations_collection(decorations, cx)
        });
        *collection = Some(col);
    }
}

/// Renders read-only / display text with green/red inline highlighting and hover tooltips for `{{...}}`.
pub fn render_templated_text(
    id: impl Into<ElementId>,
    text: &str,
    ctx: &TemplateContext,
    base_color: Hsla,
    cx: &App,
) -> AnyElement {
    let spans = parse_template_spans(text, ctx);
    if spans.is_empty() {
        return div()
            .min_w_0()
            .text_color(base_color)
            .child(text.to_string())
            .into_any_element();
    }

    let base_id = id.into();
    let mut row = h_flex().flex_wrap().items_center().min_w_0();
    let mut last_end = 0;

    for (idx, span) in spans.into_iter().enumerate() {
        if span.range.start > last_end {
            row = row.child(
                div()
                    .text_color(base_color)
                    .child(text[last_end..span.range.start].to_string()),
            );
        }

        let is_resolved = span.resolution.is_some();
        let accent = if is_resolved {
            template_resolved_color(cx)
        } else {
            template_unresolved_color(cx)
        };
        let tip: SharedString = tooltip_text_for_span(&span).into();

        row = row.child(
            div()
                .id(ElementId::Name(format!("{base_id:?}-tpl-{idx}").into()))
                .px(px(2.))
                .rounded(px(3.))
                .bg(accent.opacity(0.14))
                .border_1()
                .border_color(accent.opacity(0.35))
                .text_color(accent)
                .cursor_default()
                .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
                .child(span.raw),
        );
        last_end = span.range.end;
    }

    if last_end < text.len() {
        row = row.child(div().text_color(base_color).child(text[last_end..].to_string()));
    }

    row.into_any_element()
}

/// LSP-style hover provider that displays resolved or unresolved `{{...}}` template info in code/body editors.
pub struct TemplateHoverProvider {
    context: Rc<RefCell<TemplateContext>>,
}

impl TemplateHoverProvider {
    /// Creates a hover provider sharing live template variable state.
    pub fn new(context: Rc<RefCell<TemplateContext>>) -> Self {
        Self { context }
    }
}

impl HoverProvider for TemplateHoverProvider {
    fn hover(
        &self,
        text: &Rope,
        offset: usize,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<anyhow::Result<Option<lsp_types::Hover>>> {
        let text_len = text.len();
        if offset > text_len {
            return Task::ready(Ok(None));
        }

        let point = text.offset_to_point(offset);
        let line_start = text.line_start_offset(point.row);
        let line = text.slice_line(point.row).to_string();
        let local_offset = offset.saturating_sub(line_start);

        let ctx = self.context.borrow();
        let spans = parse_template_spans(&line, &ctx);

        let Some(hit) = spans
            .into_iter()
            .find(|s| s.range.start <= local_offset && local_offset <= s.range.end)
        else {
            return Task::ready(Ok(None));
        };

        let start_pos = text.offset_to_position(line_start + hit.range.start);
        let end_pos = text.offset_to_position(line_start + hit.range.end);

        let markdown = match &hit.resolution {
            Some(res) => {
                let display_val = if res.value.is_empty() {
                    "<empty string>".to_string()
                } else {
                    truncate_tooltip_value(&res.value, 500)
                };
                format!(
                    "**Resolved Template** `{}` ({})\n\n```text\n{}\n```",
                    hit.raw,
                    res.source.label(),
                    display_val
                )
            }
            None => {
                if hit.key.is_empty() {
                    "**Empty Template** `{{}}`\n\nProvide a variable name between braces.".to_string()
                } else {
                    format!(
                        "**Unresolved Template** `{}`\n\nNo value found in active Request, Environment, Collection, or OS environment.",
                        hit.raw
                    )
                }
            }
        };

        Task::ready(Ok(Some(lsp_types::Hover {
            contents: lsp_types::HoverContents::Markup(lsp_types::MarkupContent {
                kind: lsp_types::MarkupKind::Markdown,
                value: markdown,
            }),
            range: Some(lsp_types::Range::new(start_pos, end_pos)),
        })))
    }
}

use gpui::Entity;
use gpui_kit::component::input::EditorState;
