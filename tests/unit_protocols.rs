use aura::model::{AuraRequest, BodyType, HttpMethod, RequestProtocol};
use aura_core::graphql::{
    graphql_completions, lint_graphql_query, parse_introspection_response,
};
use aura_core::grpc::{
    decode_protobuf_to_json, encode_json_to_protobuf, frame_grpc_message, parse_proto_file,
    sample_json_for_message, unframe_grpc_messages, RpcCardinality,
};
use aura_core::soap::{
    extract_soap_body, parse_soap_fault, soap_headers, wrap_soap_envelope, SoapVersion,
};
use aura_parser::{parse_request, request_to_spec};
use aura_sync::importers::import_wsdl;

#[test]
fn test_graphql_introspection_lint_completion_and_spec_roundtrip() {
    let introspection_json = r#"{
        "data": {
            "__schema": {
                "queryType": { "name": "Query" },
                "mutationType": null,
                "subscriptionType": null,
                "types": [
                    {
                        "kind": "OBJECT",
                        "name": "Query",
                        "fields": [
                            {
                                "name": "user",
                                "args": [
                                    { "name": "id", "type": { "kind": "NON_NULL", "ofType": { "kind": "SCALAR", "name": "ID" } } }
                                ],
                                "type": { "kind": "OBJECT", "name": "User" }
                            }
                        ]
                    },
                    {
                        "kind": "OBJECT",
                        "name": "User",
                        "fields": [
                            { "name": "id", "args": [], "type": { "kind": "SCALAR", "name": "ID" } },
                            { "name": "email", "args": [], "type": { "kind": "SCALAR", "name": "String" } }
                        ]
                    }
                ]
            }
        }
    }"#;

    let schema = parse_introspection_response(introspection_json).expect("introspection parse");
    assert_eq!(schema.query_type.as_deref(), Some("Query"));
    assert!(schema.types.contains_key("User"));

    let valid_diags = lint_graphql_query("query {\n  user(id: \"1\") { id email }\n}", Some(&schema));
    assert!(valid_diags.is_empty(), "Unexpected diags: {valid_diags:?}");

    let bad_diags = lint_graphql_query("query {\n  nonExistent {\n}", Some(&schema));
    assert!(bad_diags.len() >= 2);

    let comps = graphql_completions("use", Some(&schema));
    assert!(comps.iter().any(|c| c.label == "user"));

    let mut req = AuraRequest::new("Get User", HttpMethod::POST, "https://api.example.com/graphql");
    req.protocol = RequestProtocol::Graphql;
    req.body = BodyType::GraphQL {
        query: "query GetUser($id: ID!) {\n  user(id: $id) { id email }\n}".to_string(),
        variables: "{\"id\": \"42\"}".to_string(),
    };
    let spec = request_to_spec(&req);
    let reparsed = parse_request(&spec).expect("reparse GraphQL spec");
    assert_eq!(reparsed.protocol, RequestProtocol::Graphql);
    match reparsed.body {
        BodyType::GraphQL { query, variables } => {
            assert!(query.contains("user(id: $id)"));
            assert_eq!(variables.trim(), "{\"id\": \"42\"}");
        }
        other => panic!("Expected GraphQL body, got {other:?}"),
    }
}

#[test]
fn test_proto_parser_and_wire_codec_roundtrip() {
    let proto_src = r#"
        syntax = "proto3";
        package demo;

        enum Role {
            MEMBER = 0;
            ADMIN = 1;
        }

        message HelloRequest {
            string name = 1;
            int32 age = 2;
            repeated string tags = 3;
            Role role = 4;
        }

        message HelloReply {
            string message = 1;
            bool ok = 2;
        }

        service Greeter {
            rpc SayHello (HelloRequest) returns (HelloReply);
            rpc StreamReplies (HelloRequest) returns (stream HelloReply);
            rpc Chat (stream HelloRequest) returns (stream HelloReply);
        }
    "#;

    let schema = parse_proto_file(proto_src).expect("parse proto");
    assert_eq!(schema.package.as_deref(), Some("demo"));
    assert_eq!(schema.services.len(), 1);
    assert_eq!(schema.services[0].methods[0].cardinality(), RpcCardinality::Unary);
    assert_eq!(
        schema.services[0].methods[1].cardinality(),
        RpcCardinality::ServerStreaming
    );
    assert_eq!(
        schema.services[0].methods[2].cardinality(),
        RpcCardinality::BidiStreaming
    );

    let sample = sample_json_for_message(&schema, "HelloRequest");
    assert_eq!(sample["name"], "example");

    let input_json = serde_json::json!({
        "name": "Aura",
        "age": 7,
        "tags": ["fast", "rust"],
        "role": "ADMIN"
    });
    let encoded = encode_json_to_protobuf(&schema, "demo.HelloRequest", &input_json)
        .expect("encode protobuf");
    let framed = frame_grpc_message(&encoded);
    let unframed = unframe_grpc_messages(&framed).expect("unframe");
    assert_eq!(unframed.len(), 1);

    let decoded = decode_protobuf_to_json(&schema, "demo.HelloRequest", &unframed[0])
        .expect("decode protobuf");
    assert_eq!(decoded["name"], "Aura");
    assert_eq!(decoded["age"], 7);
    assert_eq!(decoded["role"], "ADMIN");
}

#[test]
fn test_wsdl_import_and_soap_envelope_helpers() {
    let wsdl = r#"<?xml version="1.0" encoding="utf-8"?>
    <definitions xmlns="http://schemas.xmlsoap.org/wsdl/"
                 xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/"
                 xmlns:tns="http://example.com/calc"
                 targetNamespace="http://example.com/calc"
                 name="Calculator">
      <types>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="http://example.com/calc">
          <xs:element name="Add">
            <xs:complexType><xs:sequence>
              <xs:element name="intA" type="xs:int"/>
              <xs:element name="intB" type="xs:int"/>
            </xs:sequence></xs:complexType>
          </xs:element>
        </xs:schema>
      </types>
      <portType name="CalculatorSoap">
        <operation name="Add"/>
      </portType>
      <binding name="CalculatorSoap" type="tns:CalculatorSoap">
        <operation name="Add">
          <soap:operation soapAction="http://example.com/calc/Add"/>
        </operation>
      </binding>
      <service name="CalculatorService">
        <port name="CalculatorSoap" binding="tns:CalculatorSoap">
          <soap:address location="https://example.com/Calculator.asmx"/>
        </port>
      </service>
    </definitions>"#;

    let col = import_wsdl(wsdl).expect("import WSDL");
    assert_eq!(col.name, "CalculatorService");
    assert_eq!(col.requests.len(), 1);
    let req = &col.requests[0];
    assert_eq!(req.name, "Add");
    assert_eq!(req.protocol, RequestProtocol::Soap);
    assert_eq!(req.url, "https://example.com/Calculator.asmx");
    assert!(req
        .headers
        .iter()
        .any(|h| h.key == "SOAPAction" && h.value.contains("http://example.com/calc/Add")));
    assert!(req.body.content_str().contains("<intA>?</intA>"));

    let env12 = wrap_soap_envelope(
        SoapVersion::Soap12,
        "Ping",
        "http://example.com/calc",
        "<Ping/>",
        None,
    );
    assert!(env12.contains("http://www.w3.org/2003/05/soap-envelope"));
    assert_eq!(extract_soap_body(&env12).unwrap(), "<Ping/>");

    let fault_xml = r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
      <soap:Body><soap:Fault><faultcode>soap:Client</faultcode><faultstring>Invalid input</faultstring></soap:Fault></soap:Body>
    </soap:Envelope>"#;
    let fault = parse_soap_fault(fault_xml).expect("parse SOAP fault");
    assert_eq!(fault.code, "soap:Client");
    assert_eq!(fault.reason, "Invalid input");
    assert!(!soap_headers(SoapVersion::Soap11, Some("DoIt")).is_empty());
}
