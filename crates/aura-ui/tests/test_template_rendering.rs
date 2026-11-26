//! Unit and headless GPUI tests for `{{}}` template resolution, coloring, and hover tooltips.

use std::cell::RefCell;
use std::rc::Rc;

use aura_core::templating::TemplateContext;
use aura_ui::templating_ui::{
    TemplateHoverProvider, TemplateSource, decode_token_id, encode_token_id, parse_template_spans,
    sync_editor_template_decorations, sync_input_template_tokens, tooltip_text_for_span,
};
use gpui::{AppContext as _, Entity};
use gpui_kit::component::input::{
    EditorState, HoverProvider as _, InputState, Rope, TextDecorationCollection,
};

fn sample_context() -> TemplateContext {
    let mut ctx = TemplateContext::default();
    ctx.request_vars
        .insert("base_url".to_string(), "https://api.example.com".to_string());
    ctx.environment_vars
        .insert("token".to_string(), "env-secret-123".to_string());
    ctx.collection_vars
        .insert("version".to_string(), "v2".to_string());
    ctx
}

#[test]
fn test_parse_template_spans_cascading_and_unresolved() {
    let ctx = sample_context();
    let input = "{{base_url}}/{{version}}/users?auth={{missing_var}}&empty={{}}";
    let spans = parse_template_spans(input, &ctx);

    assert_eq!(spans.len(), 4);

    assert_eq!(spans[0].key, "base_url");
    assert_eq!(&input[spans[0].range.clone()], "{{base_url}}");
    let r0 = spans[0].resolution.as_ref().unwrap();
    assert_eq!(r0.value, "https://api.example.com");
    assert_eq!(r0.source, TemplateSource::Request);

    assert_eq!(spans[1].key, "version");
    let r1 = spans[1].resolution.as_ref().unwrap();
    assert_eq!(r1.value, "v2");
    assert_eq!(r1.source, TemplateSource::Collection);

    assert_eq!(spans[2].key, "missing_var");
    assert!(spans[2].resolution.is_none());
    assert!(tooltip_text_for_span(&spans[2]).contains("Unresolved template"));

    assert_eq!(spans[3].key, "");
    assert!(spans[3].resolution.is_none());
    assert!(tooltip_text_for_span(&spans[3]).contains("Empty template"));
}

#[test]
fn test_token_id_roundtrip_preserves_pipes_and_status() {
    let ctx = sample_context();
    let spans = parse_template_spans("{{token}}", &ctx);
    let encoded = encode_token_id(0, &spans[0]);
    let decoded = decode_token_id(&encoded).unwrap();
    assert!(decoded.is_resolved);
    assert_eq!(decoded.key, "token");
    assert_eq!(decoded.value, "env-secret-123");
    assert_eq!(decoded.source_label, "Active Environment");
}

#[gpui_kit::test]
async fn test_input_tokens_and_editor_hover_decorations(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        aura_ui::init_kit(cx);
    });

    let ctx = sample_context();
    let shared_ctx = Rc::new(RefCell::new(ctx.clone()));

    let (window, cx) = cx.add_window_view(|window, cx| {
        let input = cx.new(|cx| {
            InputState::new(window, cx).default_value("{{base_url}}/{{unknown}}")
        });
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("json")
                .default_value("{\n  \"url\": \"{{base_url}}\",\n  \"bad\": \"{{nope}}\"\n}")
        });
        TestHarnessView { input, editor }
    });

    let (input_entity, editor_entity) = window.read_with(cx, |v, _| {
        (v.input.clone(), v.editor.clone())
    });

    cx.update(|_window, cx| {
        sync_input_template_tokens(&input_entity, &ctx, cx);
        let tokens = input_entity.read(cx).tokens();
        assert_eq!(tokens.len(), 2);
        let first_meta = decode_token_id(tokens[0].token().id()).unwrap();
        let second_meta = decode_token_id(tokens[1].token().id()).unwrap();
        assert!(first_meta.is_resolved);
        assert_eq!(first_meta.value, "https://api.example.com");
        assert!(!second_meta.is_resolved);

        let mut col: Option<TextDecorationCollection> = None;
        sync_editor_template_decorations(&editor_entity, &mut col, &ctx, cx);
        let col = col.expect("editor should have template decorations");
        let ranges = col.get_ranges(cx);
        assert_eq!(ranges.len(), 2);
    });

    // Verify HoverProvider resolves and describes both resolved and unresolved spans.
    let hover_provider = TemplateHoverProvider::new(shared_ctx);
    let sample_code = "{\n  \"url\": \"{{base_url}}\",\n  \"bad\": \"{{nope}}\"\n}";
    let rope = Rope::from(sample_code);
    let resolved_offset = sample_code.find("base_url").unwrap();
    let unresolved_offset = sample_code.find("nope").unwrap();

    let resolved_hover = cx
        .update(|window, cx| hover_provider.hover(&rope, resolved_offset, window, cx))
        .await
        .expect("resolved hover task ok")
        .expect("hover returned for resolved template");

    let resolved_text = format!("{:?}", resolved_hover.contents);
    assert!(resolved_text.contains("https://api.example.com"));
    assert!(resolved_text.contains("Resolved Template"));

    let unresolved_hover = cx
        .update(|window, cx| hover_provider.hover(&rope, unresolved_offset, window, cx))
        .await
        .expect("unresolved hover task ok")
        .expect("hover returned for unresolved template");

    let unresolved_text = format!("{:?}", unresolved_hover.contents);
    assert!(unresolved_text.contains("Unresolved Template"));
}

struct TestHarnessView {
    input: Entity<InputState>,
    editor: Entity<EditorState>,
}

impl gpui::Render for TestHarnessView {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        gpui::div()
    }
}
