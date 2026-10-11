use aura::scripting::AuraScriptRuntime;
use std::collections::HashMap;

#[test]
fn test_deno_core_scripting_offline_and_bdd() {
    let mut initial_vars = HashMap::new();
    initial_vars.insert("token".to_string(), "old_token".to_string());

    let mut runtime = AuraScriptRuntime::new(initial_vars).expect("Failed to initialize deno_core");

    let script = r#"
        console.log("Starting test execution");
        console.warn("Simulated warning");

        aura.variables.set("token", "new_refreshed_token");

        aura.test("check updated token", () => {
            const current = aura.variables.get("token");
            expect(current).toBe("new_refreshed_token");
        });

        aura.test("math check", () => {
            expect(10 + 5).toBe(15);
        });
    "#;

    let res = runtime.run(script).expect("Failed to run script");

    assert_eq!(res.variables.get("token").unwrap(), "new_refreshed_token");

    // Verify console logs captured
    assert_eq!(res.console_logs.len(), 2);
    assert_eq!(res.console_logs[0].level, "INFO");
    assert_eq!(res.console_logs[0].message, "Starting test execution");
    assert_eq!(res.console_logs[1].level, "WARN");
    assert_eq!(res.console_logs[1].message, "Simulated warning");

    // Verify assertions
    assert_eq!(res.test_assertions.len(), 2);
    assert!(res.test_assertions[0].passed);
    assert_eq!(res.test_assertions[0].name, "check updated token");
    assert!(res.test_assertions[1].passed);
    assert_eq!(res.test_assertions[1].name, "math check");
}
