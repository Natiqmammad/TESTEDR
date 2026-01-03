use nightscript_android::parser::{parse_tokens_with_diagnostics};
use nightscript_android::lexer::lex;
use nightscript_android::validation::validate_file;

#[test]
fn test_await_in_sync_function_fails() {
    let source = r#"
    fun sync_func() {
        await other_async();
    }
    async fun other_async() {}
    "#;
    let tokens = lex(source).expect("Lexing failed");
    let report = parse_tokens_with_diagnostics(source, tokens);
    assert!(report.errors.is_empty(), "Parsing should succeed");
    
    let errors = validate_file(&report.file);
    let has_error = errors.iter().any(|e| e.message.contains("`await` is only allowed inside `async`"));
    // Note: Rust string check is case sensitive, checking my validation message.
    // Message: "`await` is only allowed inside `async` functions or blocks"
    assert!(errors.iter().any(|e| e.message.contains("inside `async`")), "Expected validation error for await in sync function, got: {:?}", errors);
}

#[test]
fn test_await_in_async_function_passes() {
    let source = r#"
    async fun my_async_func() {
        await other_async();
    }
    async fun other_async() {}
    "#;
    let tokens = lex(source).expect("Lexing failed");
    let report = parse_tokens_with_diagnostics(source, tokens);
    assert!(report.errors.is_empty(), "Parsing should succeed");

    let errors = validate_file(&report.file);
    let await_error = errors.iter().find(|e| e.message.contains("inside `async`"));
    assert!(await_error.is_none(), "Unexpected validation error: {:?}", await_error);
}
