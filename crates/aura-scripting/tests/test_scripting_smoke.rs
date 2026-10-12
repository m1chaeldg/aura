use aura_scripting::AuraScriptRuntime;
use std::collections::HashMap;

#[test]
fn test_scripting_smoke() {
    let mut runtime = AuraScriptRuntime::new(HashMap::new()).expect("runtime initialization");
    let result = runtime
        .run("aura.variables.set('token', 'abc123xyz'); console.log('token stored');")
        .expect("execution succeeds");

    assert_eq!(result.variables.get("token"), Some(&"abc123xyz".to_string()));
    assert!(!result.console_logs.is_empty());
}
