use aura_core::assertions::{
    AssertOperator, DeclarativeAssertion, PerformanceAssertionConfig, evaluate_assertions,
    evaluate_flow_condition, evaluate_performance_assertions, parse_assert_block,
    parse_assertion_kv, parse_assertion_line, serialize_assertion_kv,
};
use aura_core::grpc::{
    ProtoMethod, RpcCardinality, decode_protobuf_to_json, encode_json_to_protobuf,
    encode_server_reflection_request, frame_grpc_message, parse_proto_file,
    sample_json_for_message, unframe_grpc_messages, ServerReflectionRequestKind,
};
use aura_core::model::AuraResponse;
use chrono::Utc;
use std::collections::HashMap;

fn sample_response(status: u16, body: &str, duration_ms: u64) -> AuraResponse {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    AuraResponse {
        status,
        status_text: "OK".to_string(),
        duration_ms,
        size_bytes: body.len(),
        headers,
        body: body.to_string(),
        timestamp: Utc::now(),
        assertions: Vec::new(),
        console_logs: Vec::new(),
        script_errors: Vec::new(),
        sse_events: Vec::new(),
        stream_messages: Vec::new(),
        timing: None,
        captured_cookies: Vec::new(),
        next_request: None,
        stop_chain: false,
        exported_variables: HashMap::new(),
    }
}

#[test]
fn test_parse_and_serialize_assertions() {
    let a = parse_assertion_line("res.status == 200").expect("parse eq");
    assert!(a.enabled);
    assert_eq!(a.expression, "res.status");
    assert_eq!(a.operator, AssertOperator::Eq);
    assert_eq!(a.expected, "200");

    let disabled = parse_assertion_line("~res.body.id isNumber").expect("parse disabled");
    assert!(!disabled.enabled);
    assert_eq!(disabled.operator, AssertOperator::IsNumber);

    let (k, v) = serialize_assertion_kv(&disabled);
    assert_eq!(k, "~res.body.id");
    assert_eq!(v, "isNumber");

    let kv = parse_assertion_kv("res.headers.content-type", "contains \"application/json\"")
        .expect("parse kv");
    assert_eq!(kv.operator, AssertOperator::Contains);
    assert_eq!(kv.expected, "application/json");

    let block = parse_assert_block("# comment\nres.status==200\nres.duration_ms<500\n");
    assert_eq!(block.len(), 2);
}

#[test]
fn test_evaluate_assertions_and_flow_conditions() {
    let resp = sample_response(200, r#"{"id":42,"name":"aura","tags":["fast","rust"]}"#, 120);
    let rules = vec![
        DeclarativeAssertion::new("res.status", AssertOperator::Eq, "200"),
        DeclarativeAssertion::new("res.body.id", AssertOperator::IsNumber, ""),
        DeclarativeAssertion::new("res.body.name", AssertOperator::Contains, "aur"),
        DeclarativeAssertion::new("res.body.tags[0]", AssertOperator::StartsWith, "fa"),
        DeclarativeAssertion::new("res.headers.content-type", AssertOperator::Contains, "json"),
        DeclarativeAssertion::new("res.duration_ms", AssertOperator::Lt, "500"),
    ];
    let results = evaluate_assertions(&rules, &resp);
    assert_eq!(results.len(), 6);
    assert!(results.iter().all(|r| r.passed));

    let mut vars = HashMap::new();
    vars.insert("has_next".to_string(), "true".to_string());
    assert!(evaluate_flow_condition("vars.has_next", Some(&resp), &vars));
    assert!(!evaluate_flow_condition("!vars.has_next", Some(&resp), &vars));
    assert!(evaluate_flow_condition("prev.status == 200", Some(&resp), &vars));
}

#[test]
fn test_performance_assertions() {
    let resp = sample_response(200, "hello", 250);
    let cfg = PerformanceAssertionConfig {
        max_duration_ms: Some(100),
        min_size_bytes: Some(10),
        max_size_bytes: Some(100),
    };
    let results = evaluate_performance_assertions(&cfg, &resp);
    assert_eq!(results.len(), 3);
    assert!(!results[0].passed); // duration 250 > 100
    assert!(!results[1].passed); // size 5 < 10
    assert!(results[2].passed);  // size 5 <= 100
}

#[test]
fn test_grpc_proto_parse_frame_and_codec() {
    let proto = r#"
        syntax = "proto3";
        package demo;
        service Greeter {
            rpc SayHello (HelloRequest) returns (HelloReply);
        }
        message HelloRequest {
            string name = 1;
            int32 count = 2;
        }
        message HelloReply {
            string message = 1;
        }
    "#;
    let schema = parse_proto_file(proto).expect("parse proto");
    assert_eq!(schema.package.as_deref(), Some("demo"));
    assert_eq!(schema.services.len(), 1);
    assert_eq!(schema.services[0].methods[0].cardinality(), RpcCardinality::Unary);

    let sample = sample_json_for_message(&schema, "HelloRequest");
    assert!(sample.get("name").is_some());

    let encoded = encode_json_to_protobuf(
        &schema,
        "HelloRequest",
        &serde_json::json!({"name": "Aura", "count": 3}),
    )
    .expect("encode protobuf");
    let framed = frame_grpc_message(&encoded);
    let unframed = unframe_grpc_messages(&framed).expect("unframe");
    assert_eq!(unframed.len(), 1);
    assert_eq!(unframed[0], encoded);

    let decoded = decode_protobuf_to_json(&schema, "HelloRequest", &unframed[0]).expect("decode");
    assert_eq!(decoded["name"], "Aura");
    assert_eq!(decoded["count"], 3);

    let refl = encode_server_reflection_request(ServerReflectionRequestKind::ListServices);
    assert!(!refl.is_empty());

    let bidi = ProtoMethod {
        name: "Chat".into(),
        input_type: "Req".into(),
        output_type: "Res".into(),
        client_streaming: true,
        server_streaming: true,
        full_path: "/demo.Svc/Chat".into(),
    };
    assert_eq!(bidi.cardinality(), RpcCardinality::BidiStreaming);
}
