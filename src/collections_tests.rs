use crate::runtime::{OptionValue, ResultValue};
use crate::{ast, lexer, parser, validation, Interpreter, RuntimeError, Value};

fn parse_ok(source: &str) -> ast::File {
    let tokens = lexer::lex(source).expect("lexing should succeed");
    let report = parser::parse_tokens_with_diagnostics(source, tokens);
    assert!(
        report.errors.is_empty(),
        "unexpected parse errors: {report:?}"
    );
    report.file
}

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
fn parses_array_type_and_literal() {
    let tokens = lexer::lex(
        r#"
fun apex() {
    let a:: [i32; 3] = [1, 2, 3];
}
"#,
    )
    .expect("lexing should succeed");
    let report = parser::parse_tokens_with_diagnostics(
        "fun apex() { let a:: [i32; 3] = [1, 2, 3]; }",
        tokens,
    );
    assert!(
        report.errors.is_empty(),
        "unexpected parse errors: {report:?}"
    );
}

#[test]
fn array_len_and_bounds() {
    let ok = run_apex(
        r#"
fun apex() {
    let a = [1, 2, 3];
    return a.len();
}
"#,
    )
    .expect("expected success");
    assert!(matches!(ok, Value::Int(3)));

    let err = run_apex(
        r#"
fun apex() {
    let a = [1, 2, 3];
    return a[10];
}
"#,
    )
    .unwrap_err();
    assert!(err.message().contains("out of bounds"));
}

#[test]
fn vec_basic_ops() {
    let result = run_apex(
        r#"
fun apex() {
    let v = vec.new();
    vec.push(v, 1);
    vec.push(v, 2);
    let first = vec.get(v, 0);
    let popped = vec.pop(v);
    return (first, popped, vec.len(v));
}
"#,
    )
    .expect("expected success");
    let items = match result {
        Value::Tuple(items) => items,
        other => panic!("expected tuple result, got {other:?}"),
    };
    assert_eq!(items.len(), 3);
    match &items[0] {
        Value::Option(OptionValue::Some { value, .. }) => {
            assert!(
                matches!(**value, Value::Int(1)),
                "expected vec.get to return 1"
            );
        }
        other => panic!("expected vec.get result to be some(1), got {other:?}"),
    }
    match &items[1] {
        Value::Option(OptionValue::Some { value, .. }) => {
            assert!(
                matches!(**value, Value::Int(2)),
                "expected vec.pop to return 2"
            );
        }
        other => panic!("expected vec.pop result to be some(2), got {other:?}"),
    }
    assert!(
        matches!(items[2], Value::Int(1)),
        "expected len after pop to be 1"
    );
}

#[test]
fn vec_set_bounds_error() {
    let value = run_apex(
        r#"
fun apex() {
    let v = vec.new();
    vec.push(v, 1);
    return vec.set(v, 5, 2);
}
"#,
    )
    .expect("execution should succeed and return result");
    match value {
        Value::Result(ResultValue::Err { value, .. }) => match &*value {
            Value::String(msg) => assert!(
                msg.contains("index out of bounds"),
                "expected bounds message, got {msg}"
            ),
            other => panic!("expected error string from vec.set, got {other:?}"),
        },
        other => panic!("expected result err from vec.set, got {other:?}"),
    }
}

#[test]
fn nested_array_indexing() {
    let result = run_apex(
        r#"
fun apex() {
    let grid:: [[i32; 2]; 2] = [[1, 2], [3, 4]];
    return grid[1][0];
}
"#,
    )
    .expect("expected success");
    assert!(matches!(result, Value::Int(3)));
}

#[test]
fn vec_of_vec_type_enforced() {
    let err = run_apex(
        r#"
fun apex() {
    let outer:: vec<vec<i32>> = vec.new();
    let inner = vec.new();
    vec.push(inner, 1);
    vec.push(outer, inner);
    // Wrong: pushing int into vec<vec<i32>>
    vec.push(outer, 5);
}
"#,
    )
    .unwrap_err();
    assert!(err.message().contains("vec"));
}

#[test]
fn map_basic_put_get() {
    let value = run_apex(
        r#"
fun apex() {
    let m = map.new();
    map.put(m, "a", 1);
    map.put(m, "b", 2);
    return map.get(m, "a");
}
"#,
    )
    .expect("expected success");
    match value {
        Value::Option(OptionValue::Some { value, .. }) => assert!(matches!(*value, Value::Int(1))),
        other => panic!("expected option.some, got {other:?}"),
    }
}

#[test]
fn set_mismatched_union_errors() {
    let err = run_apex(
        r#"
fun apex() {
    let a = set.new();
    let b = set.new();
    set.insert(a, 1);
    set.insert(b, "x");
    set.union(a, b);
}
"#,
    )
    .unwrap_err();
    assert!(err.message().contains("set op type mismatch"));
}

#[test]
fn set_unsupported_element_type_errors() {
    let err = run_apex(
        r#"
fun apex() {
    let s = set.new();
    set.insert(s, [1]);
}
"#,
    )
    .unwrap_err();
    assert!(err.message().contains("map/set key type not supported"));
}

#[test]
fn tuple_parses_and_indexes() {
    let file = parse_ok(
        r#"
fun apex() {
    let t:: tuple(str, i32) = ("hi", 42);
    return t[1];
}
"#,
    );
    assert_eq!(file.items.len(), 1);
    let result = run_apex(
        r#"
fun apex() {
    let t = ("hi", 42);
    return t[1];
}
"#,
    )
    .expect("expected success");
    assert!(matches!(result, Value::Int(42)));
}

#[test]
fn tuple_index_oob_errors() {
    let err = run_apex(
        r#"
fun apex() {
    let t = (1, 2);
    return t[5];
}
"#,
    )
    .unwrap_err();
    assert!(err.message().contains("tuple index out of bounds"));
}

#[test]
fn map_rejects_unsupported_key() {
    let err = run_apex(
        r#"
fun apex() {
    let m = map.new();
    map.put(m, [1], "oops");
}
"#,
    )
    .unwrap_err();
    assert!(err.message().contains("map/set key type not supported"));
}

#[test]
fn set_union_and_contains() {
    let value = run_apex(
        r#"
fun apex() {
    let s1 = set.new();
    let s2 = set.new();
    set.insert(s1, "a");
    set.insert(s2, "b");
    let merged = set.union(s1, s2);
    return merged;
}
"#,
    )
    .expect("expected success");
    let merged = match value {
        Value::Result(ResultValue::Ok { value, .. }) => *value,
        other => panic!("expected result ok set, got {other:?}"),
    };
    match merged {
        Value::Set(set_rc) => {
            let set_ref = set_rc.borrow();
            assert_eq!(set_ref.items.len(), 2);
            assert!(set_ref
                .items
                .iter()
                .any(|k| matches!(k, crate::runtime::MapKey::Str(s) if s == "a")));
            assert!(set_ref
                .items
                .iter()
                .any(|k| matches!(k, crate::runtime::MapKey::Str(s) if s == "b")));
        }
        other => panic!("expected set value, got {other:?}"),
    }
}
