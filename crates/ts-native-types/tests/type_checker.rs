use std::{fs, path::PathBuf};

use thiserror as _;
use ts_native_parser::{ast::Statement, parse_source};
use ts_native_types::{BuiltinFunction, ResolvedSymbol, type_check};

fn workspace_path(relative_path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative_path)
}

fn read_workspace_file(relative_path: &str) -> String {
    fs::read_to_string(workspace_path(relative_path)).expect("fixture should be readable")
}

fn check_source(source: &str) -> Result<(), String> {
    let program = parse_source(source).map_err(|error| error.to_string())?;
    type_check(&program, source)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[test]
fn type_checks_pass_fixtures_and_examples() {
    for relative_path in [
        "tests/fixtures/programs/pass/simple_const.ts",
        "tests/fixtures/programs/pass/arithmetic.ts",
        "examples/hello.ts",
        "examples/fibonacci.ts",
        "examples/type_inference.ts",
        "examples/functions.ts",
    ] {
        let source = read_workspace_file(relative_path);
        check_source(&source).unwrap_or_else(|error| {
            panic!("expected {relative_path} to type-check successfully: {error}");
        });
    }
}

#[test]
fn type_checks_nested_function_calls_and_widening() {
    let source = r#"export {};

function add(a: int, b: int): int {
  return a + b;
}

function widen(value: int): double {
  return value;
}

function run(): double {
  const base = add(1, 2);
  return widen(add(base, 3));
}
"#;

    check_source(source).expect("source should type-check");
}

#[test]
fn rejects_existing_fail_fixtures() {
    for (relative_path, expected_message) in [
        (
            "tests/fixtures/programs/fail/invalid_reassign.ts",
            "cannot assign to immutable binding",
        ),
        (
            "tests/fixtures/programs/fail/type_mismatch.ts",
            "cannot assign value of type double",
        ),
    ] {
        let source = read_workspace_file(relative_path);
        let error = check_source(&source).expect_err("fixture should fail semantic analysis");
        assert!(
            error.contains(expected_message),
            "unexpected error: {error}"
        );
    }
}

#[test]
fn rejects_bad_call_and_return_semantics() {
    for (source, expected_message) in [
        (
            "export {};\nfunction add(a: int, b: int): int { return a + b; }\nfunction run(): int { return add(1); }\n",
            "expects 2 argument(s), found 1",
        ),
        (
            "export {};\nfunction run(): int { return; }\n",
            "must return a value",
        ),
        (
            "export {};\nfunction run(): void { return 1; }\n",
            "void functions cannot return a value",
        ),
        (
            "export {};\nreturn 1;\n",
            "return statements are only valid inside functions",
        ),
        (
            "export {};\nfunction run(): void { 1 + 2; }\n",
            "only call expressions may appear as standalone statements",
        ),
    ] {
        let error = check_source(source).expect_err("source should fail semantic analysis");
        assert!(
            error.contains(expected_message),
            "unexpected error: {error}"
        );
    }
}

#[test]
fn type_checks_builtin_print_calls() {
    let source = r#"export {};

function run(): void {
  printInt(1);
  printDouble(2);
  printBool(true);
  return;
}
"#;

    let program = parse_source(source).expect("source should parse");
    let typed_program = type_check(&program, source).expect("source should type-check");

    let Statement::FunctionDeclaration(function) = &typed_program.syntax.statements[0] else {
        panic!("expected function declaration");
    };
    let Statement::Expression(print_statement) = &function.body.statements[0] else {
        panic!("expected print expression statement");
    };
    let ts_native_parser::ast::Expression::Call(print_call) = &print_statement.expression else {
        panic!("expected print call expression");
    };
    let ts_native_parser::ast::Expression::Identifier(print_callee) = print_call.callee.as_ref() else {
        panic!("expected builtin callee identifier");
    };

    match typed_program
        .resolved_symbol(print_callee.span)
        .expect("print builtin should resolve")
    {
        ResolvedSymbol::BuiltinFunction(builtin) => {
            assert_eq!(*builtin, BuiltinFunction::PrintInt);
        }
        other => panic!("expected builtin resolution, found {other:?}"),
    }
}

#[test]
fn rejects_invalid_builtin_print_usage() {
    for (source, expected_message) in [
        (
            "export {};\nfunction run(): void { printInt(true); return; }\n",
            "cannot pass value of type bool to parameter of type int",
        ),
        (
            "export {};\nfunction run(): void { printDouble(); return; }\n",
            "function 'printDouble' expects 1 argument(s), found 0",
        ),
        (
            "export {};\nfunction run(printBool: int): void { return; }\n",
            "'printBool' is a reserved builtin function name",
        ),
    ] {
        let error = check_source(source).expect_err("source should fail semantic analysis");
        assert!(
            error.contains(expected_message),
            "unexpected error: {error}"
        );
    }
}

#[test]
fn type_checks_console_log_sugar() {
    let source = r#"export {};

function run(): void {
  console.log(1);
  console.log(2.5);
  console.log(true);
  return;
}
"#;

    check_source(source).expect("source should type-check");
}

#[test]
fn rejects_invalid_console_log_usage() {
    for (source, expected_message) in [
        (
            "export {};\nfunction run(): void { console.log(); return; }\n",
            "console.log expects 1 argument(s), found 0",
        ),
        (
            "export {};\nfunction noop(): void { return; }\nfunction run(): void { console.log(noop()); return; }\n",
            "console.log does not accept values of type void",
        ),
    ] {
        let error = check_source(source).expect_err("source should fail semantic analysis");
        assert!(
            error.contains(expected_message),
            "unexpected error: {error}"
        );
    }
}

#[test]
fn rejects_semantic_fail_fixtures() {
    for (relative_path, expected_message) in [
        (
            "tests/fixtures/programs/fail/bad_call_arity.ts",
            "expects 2 argument(s), found 1",
        ),
        (
            "tests/fixtures/programs/fail/bad_return.ts",
            "must return a value",
        ),
        (
            "tests/fixtures/programs/fail/non_bool_while.ts",
            "while condition must have type bool",
        ),
        (
            "tests/fixtures/programs/fail/unknown_function.ts",
            "use of undeclared identifier 'missing'",
        ),
    ] {
        let source = read_workspace_file(relative_path);
        let error = check_source(&source).expect_err("fixture should fail semantic analysis");
        assert!(
            error.contains(expected_message),
            "unexpected error: {error}"
        );
    }
}

#[test]
fn exposes_semantic_tables_for_lowering() {
    let source = r#"export {};

function run(value: int): int {
  const total = value + 1;
  return total;
}
"#;

    let program = parse_source(source).expect("source should parse");
    let typed_program = type_check(&program, source).expect("source should type-check");

    let Statement::FunctionDeclaration(function) = &typed_program.syntax.statements[0] else {
        panic!("expected function declaration");
    };

    let Statement::VariableDeclaration(total) = &function.body.statements[0] else {
        panic!("expected variable declaration");
    };
    let Statement::Return(return_statement) = &function.body.statements[1] else {
        panic!("expected return statement");
    };
    let return_value = return_statement
        .value
        .as_ref()
        .expect("return should have value");

    let parameter_binding = typed_program
        .binding(function.parameters[0].name.span)
        .expect("parameter binding should exist");
    assert_eq!(
        parameter_binding.type_name,
        ts_native_parser::ast::TypeName::Int
    );
    assert!(parameter_binding.mutable);

    let local_binding = typed_program
        .binding(total.name.span)
        .expect("local binding should exist");
    assert_eq!(
        local_binding.type_name,
        ts_native_parser::ast::TypeName::Int
    );
    assert!(!local_binding.mutable);

    assert_eq!(
        typed_program.expression_type(total.initializer.span()),
        Some(ts_native_parser::ast::TypeName::Int)
    );
    assert_eq!(
        typed_program.expression_type(return_value.span()),
        Some(ts_native_parser::ast::TypeName::Int)
    );

    match typed_program
        .resolved_symbol(return_value.span())
        .expect("return identifier should resolve")
    {
        ResolvedSymbol::Variable(binding) => {
            assert_eq!(binding.type_name, ts_native_parser::ast::TypeName::Int);
            assert!(!binding.mutable);
        }
        ResolvedSymbol::Function(_) => panic!("expected variable resolution"),
        ResolvedSymbol::BuiltinFunction(_) => panic!("expected variable resolution"),
    }
}
