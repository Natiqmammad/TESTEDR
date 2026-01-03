use crate::{lexer, parser, validation, Interpreter, RuntimeError, Value};

fn run(source: &str) -> Result<Value, RuntimeError> {
    let tokens = lexer::lex(source).expect("lexing should succeed");
    let report = parser::parse_tokens_with_diagnostics(source, tokens);
    assert!(
        report.errors.is_empty(),
        "unexpected parse errors: {report:?}"
    );
    let errors = validation::validate_file(&report.file);
    assert!(
        errors.is_empty(),
        "unexpected validation errors: {errors:?}"
    );
    let mut interp = Interpreter::new();
    interp.register_file(&report.file)?;
    interp.call_function_by_name("apex", Vec::new())
}

fn validation_errors(source: &str) -> Vec<validation::ValidationError> {
    let tokens = lexer::lex(source).expect("lexing should succeed");
    let report = parser::parse_tokens_with_diagnostics(source, tokens);
    assert!(
        report.errors.is_empty(),
        "unexpected parse errors: {report:?}"
    );
    validation::validate_file(&report.file)
}

#[test]
fn allows_shadowing_in_inner_scope() {
    let val = run(r#"
fun apex() {
    let x = 1;
    {
        let x = 2;
        return x;
    }
}
"#)
    .expect("apex should run");
    assert!(matches!(val, Value::Int(2)));
}

#[test]
fn duplicate_binding_in_same_scope_is_error() {
    let errs = validation_errors(
        r#"
fun apex() {
    let x = 1;
    let x = 2;
}
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.message.contains("Duplicate binding `x`")),
        "expected duplicate binding error, got {errs:?}"
    );
}

#[test]
fn self_mut_requires_mutable_binding() {
    let err = run(r#"
struct Counter { n:: i32 }
impl Counter {
    fun bump(self_mut:: Counter) -> i32 { return self_mut.n + 1; }
}

fun apex() {
    let c = Counter { n: 1 };
    return c.bump();
}
"#)
    .unwrap_err();
    assert!(err
        .message()
        .contains("cannot borrow immutable value as mutable (method requires self_mut)"));
}
