use crate::{lexer, parser, validation, Interpreter, RuntimeError, Value};

fn run_apex(source: &str) -> Result<Value, RuntimeError> {
    let tokens = lexer::lex(source).expect("lexing should succeed");
    let report = parser::parse_tokens_with_diagnostics(source, tokens);
    assert!(
        report.errors.is_empty(),
        "unexpected parse errors: {report:?}"
    );
    let validation_errors = validation::validate_file(&report.file);
    assert!(
        validation_errors.is_empty(),
        "unexpected validation errors: {validation_errors:?}"
    );
    let mut interp = Interpreter::new();
    interp.register_file(&report.file)?;
    interp.call_function_by_name("apex", Vec::new())
}

#[test]
fn function_arity_and_type_mismatch() {
    let arity_err = run_apex(
        r#"
fun foo(a:: i32) -> i32 { return a; }
fun apex() { return foo(); }
"#,
    )
    .unwrap_err();
    assert!(
        arity_err.message().contains("expects 1 arguments"),
        "got {}",
        arity_err.message()
    );

    let type_err = run_apex(
        r#"
fun foo(a:: i32) -> i32 { return a; }
fun apex() { return foo("hi"); }
"#,
    )
    .unwrap_err();
    assert!(
        type_err.message().contains("function argument"),
        "unexpected msg {}",
        type_err.message()
    );
}

#[test]
fn struct_init_field_and_method() {
    let val = run_apex(
        r#"
struct User { name:: str, age:: i32 }
impl User { fun greet(self:: User) -> str { return self.name; } }

fun apex() {
    let u = User { name: "Ada", age: 30 };
    return (u.age, u.greet());
}
"#,
    )
    .expect("apex should succeed");
    match val {
        Value::Tuple(items) => {
            assert!(matches!(items[0], Value::Int(30)));
            match &items[1] {
                Value::String(s) => assert_eq!(s, "Ada"),
                other => panic!("expected greet string, got {other:?}"),
            }
        }
        other => panic!("expected tuple result, got {other:?}"),
    }
}

#[test]
fn struct_missing_field_errors() {
    let err = run_apex(
        r#"
struct User { name:: str, age:: i32 }
fun apex() {
    let u = User { name: "Bob" };
    return u.age;
}
"#,
    )
    .unwrap_err();
    assert!(
        err.message().contains("Missing field `age`"),
        "unexpected msg {}",
        err.message()
    );
}

#[test]
fn enum_constructor_and_switch_binding() {
    let val = run_apex(
        r#"
enum Status { Ok, Error(str) }
fun apex() {
    let s = Status::Error("oops");
    var out = "other";
    switch s {
        Error(msg) -> out = msg,
        Ok -> out = "ok",
        _ -> out = "other",
    }
    return out;
}
"#,
    )
    .expect("apex should succeed");
    assert!(matches!(val, Value::String(ref s) if s == "oops"));
}

#[test]
fn question_mark_on_result_and_option() {
    let ok_val = run_apex(
        r#"
fun test() -> i32 {
    let r = result.ok(5);
    let v = r?;
    return v + 1;
}
fun apex() { return test(); }
"#,
    )
    .expect("ok path");
    assert!(matches!(ok_val, Value::Int(6)));

    let opt_val = run_apex(
        r#"
fun maybe() -> option<i32> {
    let v = option.none();
    let x = v?;
    return option.some(x);
}
fun apex() { return maybe(); }
"#,
    )
    .expect("option none should propagate");
    assert!(matches!(
        opt_val,
        Value::Option(crate::runtime::OptionValue::None { .. })
    ));

    let wrong = run_apex(
        r#"
fun apex() {
    let x = 1;
    return x?;
}
"#,
    )
    .unwrap_err();
    assert!(wrong
        .message()
        .contains("`?` expects result<T,E> or option<T>"));
}

#[test]
fn try_catch_catches_throw() {
    let val = run_apex(
        r#"
fun apex() {
    try {
        forge.error.throw("boom");
    } catch(e) {
        return e;
    }
    return "nope";
}
"#,
    )
    .expect("throw should be caught");
    assert!(matches!(val, Value::String(ref s) if s == "boom"));
}

#[test]
fn error_new_and_wrap_formatting() {
    let val = run_apex(
        r#"
fun apex() {
    let e = forge.error.new("E1", "failed");
    let w = forge.error.wrap(e, "ctx");
    let f = forge.error.fail("E2", "oops");
    return (e, w, f);
}
"#,
    )
    .expect("expected tuple");
    match val {
        Value::Tuple(items) => {
            assert!(matches!(items[0], Value::String(ref s) if s == "[E1] failed"));
            assert!(matches!(items[1], Value::String(ref s) if s == "ctx: [E1] failed"));
            match &items[2] {
                Value::Result(res) => match res {
                    crate::runtime::ResultValue::Err { value, .. } => assert!(matches!(
                        &**value,
                        Value::String(ref s) if s == "[E2] oops"
                    )),
                    other => panic!("expected err result, got {other:?}"),
                },
                other => panic!("expected result, got {other:?}"),
            }
        }
        other => panic!("expected tuple, got {other:?}"),
    }
}

#[test]
fn trait_dispatch_and_missing_impl_error() {
    let ok = run_apex(
        r#"
trait Display { fun to_string(self:: User) -> str; }
struct User { name:: str }
impl Display for User { fun to_string(self:: User) -> str { return self.name; } }

fun apex() {
    let u = User { name: "Neo" };
    return Display::to_string(u);
}
"#,
    )
    .expect("expected display impl to work");
    assert!(matches!(ok, Value::String(ref s) if s == "Neo"));

    let err = run_apex(
        r#"
trait Display { fun to_string(self:: Point) -> str; }
struct Point { x:: i32, y:: i32 }
fun apex() {
    let p = Point { x: 1, y: 2 };
    return Display::to_string(p);
}
"#,
    )
    .unwrap_err();
    assert!(
        err.message().contains("trait bound not satisfied"),
        "unexpected msg {}",
        err.message()
    );
}
