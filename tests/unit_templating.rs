use aura::templating::TemplateContext;

#[test]
fn test_templating_cascading_and_os_env() {
    let mut ctx = TemplateContext::new();
    ctx.request_vars
        .insert("req_id".to_string(), "REQ-100".to_string());
    ctx.environment_vars.insert(
        "base_url".to_string(),
        "https://api.staging.com".to_string(),
    );
    ctx.collection_vars
        .insert("api_version".to_string(), "v2".to_string());

    std::env::set_var("AURA_TEST_VAR", "TEST_VAL_123");

    let template = "{{base_url}}/{{api_version}}/orders/{{req_id}}?env={{$AURA_TEST_VAR}}";
    let rendered = ctx.render(template);

    assert_eq!(
        rendered,
        "https://api.staging.com/v2/orders/REQ-100?env=TEST_VAL_123"
    );
}
