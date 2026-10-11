use aura::importers::{
    import_insomnia, import_openapi, import_postman_collection, parse_curl_command,
};
use aura::model::HttpMethod;

#[test]
fn test_curl_importer() {
    let curl_cmd = r#"curl -X POST "https://postman-echo.com/post?foo=bar" -H "Content-Type: application/json" -d '{"hello":"world"}'"#;
    let req = parse_curl_command(curl_cmd).expect("Failed to parse cURL");

    assert_eq!(req.method, HttpMethod::POST);
    assert!(req.url.contains("postman-echo.com/post"));
    assert_eq!(req.headers.len(), 1);
    assert_eq!(req.headers[0].key, "Content-Type");
    assert_eq!(req.headers[0].value, "application/json");
}

#[test]
fn test_postman_importer() {
    let json = r#"{
        "info": { "name": "Echo Service" },
        "item": [
            {
                "name": "Echo GET",
                "request": {
                    "method": "GET",
                    "url": { "raw": "https://postman-echo.com/get?a=1" }
                }
            }
        ]
    }"#;
    let col = import_postman_collection(json).expect("Failed to import Postman");
    assert_eq!(col.name, "Echo Service");
    assert_eq!(col.requests.len(), 1);
    assert_eq!(col.requests[0].method, HttpMethod::GET);
}

#[test]
fn test_insomnia_importer() {
    let json = r#"{
        "resources": [
            { "_type": "workspace", "name": "Insomnia Workspace" },
            { "_type": "request", "name": "Get Status", "method": "GET", "url": "https://api.com/status" }
        ]
    }"#;
    let col = import_insomnia(json).expect("Failed to import Insomnia");
    assert_eq!(col.name, "Insomnia Workspace");
    assert_eq!(col.requests.len(), 1);
}

#[test]
fn test_openapi_importer() {
    let json = r#"{
        "openapi": "3.0.0",
        "info": { "title": "Petstore" },
        "servers": [{ "url": "https://petstore.swagger.io/v2" }],
        "paths": {
            "/pets": {
                "get": { "operationId": "listPets" }
            }
        }
    }"#;
    let col = import_openapi(json).expect("Failed to import OpenAPI");
    assert_eq!(col.name, "Petstore");
    assert_eq!(col.requests.len(), 1);
    assert_eq!(col.requests[0].url, "https://petstore.swagger.io/v2/pets");
}
