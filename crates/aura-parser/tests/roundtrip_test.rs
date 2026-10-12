use aura_parser::{parse, serialize};

#[test]
fn test_roundtrip_spec_file() {
    let spec_content = include_str!("../../../docs/file_format.md");
    let cst = parse(spec_content).expect("Failed to parse spec file");
    let serialized = serialize(&cst);
    assert_eq!(serialized, spec_content, "Round-trip invariant failed on spec file");

    let req = cst.to_request().expect("Failed to map CST to model");
    assert_eq!(req.version, "1.0");
    assert_eq!(req.name, "Create User Account");
    assert_eq!(req.method, "POST");
    assert_eq!(req.url, "{{base_url}}/api/v1/users");
    assert_eq!(req.settings.inherit_auth, true);
    assert_eq!(req.headers.get("Content-Type").unwrap(), "application/json");
    assert!(req.scripts.pre_request.is_some());
    assert!(req.scripts.post_response.is_some());
}

#[test]
fn test_roundtrip_crlf_and_lf() {
    let lf_input = "version = \"1.0\"\nname = \"Test\"\n\n[headers]\nX-Key = \"Val\"\n";
    let cst_lf = parse(lf_input).expect("Parse LF");
    assert_eq!(serialize(&cst_lf), lf_input);

    let crlf_input = "version = \"1.0\"\r\nname = \"Test\"\r\n\r\n[headers]\r\nX-Key = \"Val\"\r\n";
    let cst_crlf = parse(crlf_input).expect("Parse CRLF");
    assert_eq!(serialize(&cst_crlf), crlf_input);
}

#[test]
fn test_conflict_marker_roundtrip() {
    let conflict_input = "version = \"1.0\"\n<<<<<<< HEAD\nname = \"Local Branch\"\n=======\nname = \"Remote Branch\"\n>>>>>>> main\n\n[headers]\nX-Key = \"Val\"\n";
    let cst = parse(conflict_input).expect("Parse conflict block");
    assert!(cst.has_conflicts(), "Should detect git conflict markers");
    assert_eq!(serialize(&cst), conflict_input, "Roundtrip must preserve conflict markers");
}

#[test]
fn test_roundtrip_flow_with_pacing_and_jitter() {
    let flow_input = "version = \"1.0\"\nname = \"Pacing Test\"\nmethod = \"GET\"\nurl = \"http://api.test/items\"\n\n[flow]\nonce_per_run = true\nretry_until = \"response.status == 200\"\nretry_interval_ms = 500\nmax_retries = 3\ndelay_ms = 250\njitter_min_ms = 50\njitter_max_ms = 150\n";
    let cst = parse(flow_input).expect("Parse flow");
    assert_eq!(serialize(&cst), flow_input, "Round-trip must match flow input exactly");

    let req = cst.to_request().expect("To request");
    let flow = req.flow.expect("Flow present");
    assert_eq!(flow.once_per_run, true);
    assert_eq!(flow.retry_until.as_deref(), Some("response.status == 200"));
    assert_eq!(flow.retry_interval_ms, 500);
    assert_eq!(flow.max_retries, 3);
    assert_eq!(flow.delay_ms, Some(250));
    assert_eq!(flow.jitter_min_ms, Some(50));
    assert_eq!(flow.jitter_max_ms, Some(150));
}

#[test]
fn test_roundtrip_variables_assert_tests_settings() {
    let input = "version = \"1.0\"\nname = \"Complete Request\"\nmethod = \"POST\"\nurl = \"https://api.example.com/items\"\n\n[settings]\ninherit_auth = true\nverify_ssl = true\nfollow_redirects = true\nmax_redirects = 10\ntimeout_ms = 0\n\n[variables]\nitem_id = \"item_999\"\napi_mode = \"strict\"\n\n[assert]\n```javascript\nresponse.status === 200\nresponse.json().success === true\n```\n\n[tests]\n```javascript\ndescribe(\"Item Verification\", () => {\n  it(\"matches expectation\", () => {\n    expect(response.status).toBe(200);\n  });\n});\n```\n";
    let cst = parse(input).expect("Parse full request");
    assert_eq!(serialize(&cst), input, "Roundtrip invariant must hold for variables, assert, tests, settings");

    let req = cst.to_request().expect("To request");
    assert_eq!(req.settings.max_redirects, 10);
    assert_eq!(req.settings.timeout_ms, 0);
    assert_eq!(req.variables.get("item_id").unwrap(), "item_999");
    assert_eq!(req.variables.get("api_mode").unwrap(), "strict");
    assert!(req.assert.unwrap().contains("response.status === 200"));
    assert!(req.tests.unwrap().contains("Item Verification"));
}

#[test]
fn test_inactive_tilde_keys_roundtrip_and_omitted_from_request() {
    let input = "version = \"1.0\"\nname = \"Tilde Test\"\nmethod = \"GET\"\nurl = \"http://api.test/items\"\n\n[headers]\nAccept = \"application/json\"\nAuthorization~ = \"Bearer secret_123\"\n~X-Debug = \"true\"\n\n[params.query]\nlimit = \"10\"\nfilter~ = \"active\"\n\n[variables]\nenv_mode = \"dev\"\nsecret_flag~ = \"true\"\n";
    let cst = parse(input).expect("Parse inactive tilde keys");
    assert_eq!(serialize(&cst), input, "Roundtrip invariant must hold for inactive tilde keys");

    let req = cst.to_request().expect("To request");
    // Active headers present
    assert_eq!(req.headers.get("Accept").unwrap(), "application/json");
    // Inactive headers (ending or starting with ~) omitted
    assert!(!req.headers.contains_key("Authorization~"));
    assert!(!req.headers.contains_key("Authorization"));
    assert!(!req.headers.contains_key("~X-Debug"));
    assert!(!req.headers.contains_key("X-Debug"));

    // Active params present
    assert_eq!(req.query_params.get("limit").unwrap(), "10");
    // Inactive params omitted
    assert!(!req.query_params.contains_key("filter~"));
    assert!(!req.query_params.contains_key("filter"));

    // Active variables present
    assert_eq!(req.variables.get("env_mode").unwrap(), "dev");
    // Inactive variables omitted
    assert!(!req.variables.contains_key("secret_flag~"));
    assert!(!req.variables.contains_key("secret_flag"));
}
#[test]
fn test_roundtrip_all_body_types() {
    use aura_core::model::BodyType;

    // 1. XML
    let xml_input = "version = \"1.0\"\nname = \"XML Test\"\nmethod = \"POST\"\nurl = \"http://api.test/xml\"\n\n[body.xml]\n```xml\n<request>\n  <user id=\"1\">Alice</user>\n</request>\n```\n";
    let cst_xml = parse(xml_input).expect("Parse XML");
    assert_eq!(serialize(&cst_xml), xml_input, "XML round-trip");
    let req_xml = cst_xml.to_request().expect("To request XML");
    assert!(matches!(req_xml.body, BodyType::Xml(s) if s.contains("<user id=\"1\">Alice</user>")));

    // 2. Text
    let text_input = "version = \"1.0\"\nname = \"Text Test\"\nmethod = \"POST\"\nurl = \"http://api.test/text\"\n\n[body.text]\n```text\nHello Plain Text\nLine 2\n```\n";
    let cst_text = parse(text_input).expect("Parse Text");
    assert_eq!(serialize(&cst_text), text_input, "Text round-trip");
    let req_text = cst_text.to_request().expect("To request Text");
    assert!(matches!(req_text.body, BodyType::Text(s) if s.contains("Hello Plain Text")));

    // 3. Form Urlencoded with active and inactive (~) keys
    let form_input = "version = \"1.0\"\nname = \"Form Test\"\nmethod = \"POST\"\nurl = \"http://api.test/form\"\n\n[body.form]\nusername = \"alice\"\ngrant_type = \"password\"\nremember~ = \"false\"\n";
    let cst_form = parse(form_input).expect("Parse Form");
    assert_eq!(serialize(&cst_form), form_input, "Form round-trip");
    let req_form = cst_form.to_request().expect("To request Form");
    if let BodyType::Form(map) = req_form.body {
        assert_eq!(map.get("username").unwrap(), "alice");
        assert_eq!(map.get("grant_type").unwrap(), "password");
        assert!(!map.contains_key("remember~"));
        assert!(!map.contains_key("remember"));
    } else {
        panic!("Expected BodyType::Form");
    }

    // 4. Multipart with text and relative file references
    let multipart_input = "version = \"1.0\"\nname = \"Multipart Test\"\nmethod = \"POST\"\nurl = \"http://api.test/upload\"\n\n[body.multipart]\ndescription = \"Profile Photo\"\navatar = \"@./fixtures/avatar.png\"\ndoc~ = \"@./fixtures/doc.pdf\"\n";
    let cst_mp = parse(multipart_input).expect("Parse Multipart");
    assert_eq!(serialize(&cst_mp), multipart_input, "Multipart round-trip");
    let req_mp = cst_mp.to_request().expect("To request Multipart");
    if let BodyType::Multipart(parts) = req_mp.body {
        assert_eq!(parts.len(), 2, "Inactive doc~ part omitted from execution request");
        let desc = parts.iter().find(|p| p.name == "description").unwrap();
        assert_eq!(desc.value.as_deref(), Some("Profile Photo"));
        assert_eq!(desc.file_path, None);

        let avatar = parts.iter().find(|p| p.name == "avatar").unwrap();
        assert_eq!(avatar.value, None);
        assert_eq!(avatar.file_path.as_deref(), Some("./fixtures/avatar.png"));
    } else {
        panic!("Expected BodyType::Multipart");
    }

    // 5. Binary file
    let binary_input = "version = \"1.0\"\nname = \"Binary Test\"\nmethod = \"POST\"\nurl = \"http://api.test/bin\"\n\n[body.binary]\nfile = \"./fixtures/firmware.bin\"\ncontent_type = \"application/octet-stream\"\n";
    let cst_bin = parse(binary_input).expect("Parse Binary");
    assert_eq!(serialize(&cst_bin), binary_input, "Binary round-trip");
    let req_bin = cst_bin.to_request().expect("To request Binary");
    if let BodyType::Binary { file_path, content_type } = req_bin.body {
        assert_eq!(file_path, "./fixtures/firmware.bin");
        assert_eq!(content_type.as_deref(), Some("application/octet-stream"));
    } else {
        panic!("Expected BodyType::Binary");
    }
}
