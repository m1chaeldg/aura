use aura_scripting::AuraScriptRuntime;
use std::collections::HashMap;

#[test]
fn test_prelude_faker_generation() {
    let mut rt = AuraScriptRuntime::new(HashMap::new()).expect("create runtime");
    let script = r#"
        const name = faker.person.fullName();
        const email = faker.internet.email();
        const uuid = faker.datatype.uuid();
        aura.variables.set("name", name);
        aura.variables.set("email", email);
        aura.variables.set("uuid", uuid);
    "#;
    let res = rt.run(script).expect("run script");
    assert!(!res.variables.get("name").unwrap().is_empty());
    assert!(res.variables.get("email").unwrap().contains('@'));
    assert_eq!(res.variables.get("uuid").unwrap().len(), 36);
}

#[test]
fn test_prelude_jasmine_chai_bdd_assertions() {
    let mut rt = AuraScriptRuntime::new(HashMap::new()).expect("create runtime");
    let mut headers = HashMap::new();
    headers.insert("content-type".to_string(), "application/json".to_string());
    rt.inject_response_context(200, r#"{"users":[{"id":1,"name":"Alice"}]}"#, &headers).expect("inject response");

    let script = r#"
        console.log("Starting test suite");
        describe("User API", () => {
            it("returns 200 status", () => {
                expect(response.status).toBe(200);
            });

            it("returns json data with users", () => {
                const data = response.json();
                expect(data).to.have.property("users");
                expect(data.users[0].name).toBe("Alice");
            });

            it("handles intentional failure cleanly", () => {
                expect(response.status).toBe(404);
            });
        });
    "#;
    let res = rt.run(script).expect("run script");
    assert_eq!(res.console_logs.len(), 1);
    assert_eq!(res.console_logs[0].message, "Starting test suite");
    assert_eq!(res.test_assertions.len(), 3);
    assert!(res.test_assertions[0].passed);
    assert!(res.test_assertions[1].passed);
    assert!(!res.test_assertions[2].passed);
    assert!(res.test_assertions[2].message.is_some());
}

#[test]
fn test_async_tests_drain_properly() {
    let mut rt = AuraScriptRuntime::new(HashMap::new()).expect("create runtime");
    let script = r#"
        describe("Async Suite", () => {
            it("resolves async test", async () => {
                const val = await Promise.resolve(2);
                expect(1 + 1).toBe(val);
            });
        });
    "#;
    let res = rt.run(script).expect("run script");
    assert_eq!(res.test_assertions.len(), 1);
    assert!(res.test_assertions[0].passed);
}

#[test]
fn test_malformed_script_returns_err_gracefully() {
    let mut rt = AuraScriptRuntime::new(HashMap::new()).expect("create runtime");
    let script = "const foo = ;"; // Syntax error
    let res = rt.run(script);
    assert!(res.is_err());
}
