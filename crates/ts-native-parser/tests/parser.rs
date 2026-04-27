use std::{fs, path::PathBuf};

use thiserror as _;
use ts_native_parser::{
    TokenKind,
    ast::{BinaryOperator, Expression, Statement, TypeName},
    parse_source, tokenize,
};

fn workspace_path(relative_path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative_path)
}

fn read_workspace_file(relative_path: &str) -> String {
    fs::read_to_string(workspace_path(relative_path)).expect("fixture should be readable")
}

#[test]
fn tokenize_recognizes_keywords_literals_and_operators() {
    let tokens = tokenize("export {}; function add(a: int, b: int): int { return add(a, b); }")
        .expect("lexer should succeed");

    assert!(matches!(tokens[0].kind, TokenKind::Export));
    assert!(matches!(tokens[1].kind, TokenKind::LeftBrace));
    assert!(matches!(tokens[2].kind, TokenKind::RightBrace));
    assert!(matches!(tokens[3].kind, TokenKind::Semicolon));
    assert!(matches!(tokens[4].kind, TokenKind::Function));
    assert!(matches!(tokens[5].kind, TokenKind::Identifier(ref name) if name == "add"));
    assert!(matches!(tokens[6].kind, TokenKind::LeftParen));
    assert!(matches!(tokens[7].kind, TokenKind::Identifier(ref name) if name == "a"));
    assert!(matches!(tokens[8].kind, TokenKind::Colon));
    assert!(matches!(tokens[9].kind, TokenKind::IntType));
    assert!(matches!(tokens[10].kind, TokenKind::Comma));
    assert!(matches!(tokens[11].kind, TokenKind::Identifier(ref name) if name == "b"));
    assert!(matches!(tokens[12].kind, TokenKind::Colon));
    assert!(matches!(tokens[13].kind, TokenKind::IntType));
    assert!(matches!(tokens[14].kind, TokenKind::RightParen));
    assert!(matches!(tokens[15].kind, TokenKind::Colon));
    assert!(matches!(tokens[16].kind, TokenKind::IntType));
    assert!(matches!(tokens[17].kind, TokenKind::LeftBrace));
    assert!(matches!(tokens[18].kind, TokenKind::Return));
    assert!(matches!(tokens[19].kind, TokenKind::Identifier(ref name) if name == "add"));
    assert!(matches!(tokens[20].kind, TokenKind::LeftParen));
    assert!(matches!(tokens[21].kind, TokenKind::Identifier(ref name) if name == "a"));
    assert!(matches!(tokens[22].kind, TokenKind::Comma));
    assert!(matches!(tokens[23].kind, TokenKind::Identifier(ref name) if name == "b"));
    assert!(matches!(tokens[24].kind, TokenKind::RightParen));
    assert!(matches!(tokens[25].kind, TokenKind::Semicolon));
    assert!(matches!(tokens[26].kind, TokenKind::RightBrace));
    assert!(matches!(tokens[27].kind, TokenKind::EndOfFile));
}

#[test]
fn tokenize_recognizes_console_log_dot_syntax() {
    let tokens = tokenize("export {}; console.log(1);").expect("lexer should succeed");

    assert!(matches!(tokens[4].kind, TokenKind::Identifier(ref name) if name == "console"));
    assert!(matches!(tokens[5].kind, TokenKind::Dot));
    assert!(matches!(tokens[6].kind, TokenKind::Identifier(ref name) if name == "log"));
    assert!(matches!(tokens[7].kind, TokenKind::LeftParen));
}

#[test]
fn parse_respects_expression_precedence() {
    let source = "export {}; const value: int = 1 + 2 * 3;";
    let program = parse_source(source).expect("parser should succeed");

    let Statement::VariableDeclaration(declaration) = &program.statements[0] else {
        panic!("expected a variable declaration");
    };

    assert_eq!(declaration.type_annotation, Some(TypeName::Int));

    let Expression::Binary(addition) = &declaration.initializer else {
        panic!("expected a binary expression");
    };
    assert_eq!(addition.operator, BinaryOperator::Add);
    assert!(matches!(
        addition.left.as_ref(),
        Expression::IntegerLiteral(_)
    ));

    let Expression::Binary(multiplication) = addition.right.as_ref() else {
        panic!("expected multiplication on the right-hand side");
    };
    assert_eq!(multiplication.operator, BinaryOperator::Multiply);
}

#[test]
fn parse_fixtures_in_pass_directory() {
    for relative_path in [
        "tests/fixtures/programs/pass/simple_const.ts",
        "tests/fixtures/programs/pass/arithmetic.ts",
    ] {
        let source = read_workspace_file(relative_path);
        parse_source(&source).unwrap_or_else(|error| {
            panic!("expected {relative_path} to parse successfully: {error}");
        });
    }
}

#[test]
fn parse_examples_cover_blocks_assignments_and_while_loops() {
    for relative_path in [
        "examples/hello.ts",
        "examples/fibonacci.ts",
        "examples/type_inference.ts",
        "examples/functions.ts",
    ] {
        let source = read_workspace_file(relative_path);
        parse_source(&source).unwrap_or_else(|error| {
            panic!("expected {relative_path} to parse successfully: {error}");
        });
    }
}

#[test]
fn parse_function_declarations_and_return_statements() {
    let source = r#"export {};

function add(a: int, b: int): int {
  const total: int = a + b;
  return total;
}

function logValue(): void {
  return;
}
"#;

    let program = parse_source(source).expect("parser should succeed");
    assert_eq!(program.statements.len(), 2);

    let Statement::FunctionDeclaration(function) = &program.statements[0] else {
        panic!("expected a function declaration");
    };
    assert_eq!(function.name.name, "add");
    assert_eq!(function.parameters.len(), 2);
    assert_eq!(function.parameters[0].type_annotation, TypeName::Int);
    assert_eq!(function.return_type, TypeName::Int);
    assert!(matches!(function.body.statements[1], Statement::Return(_)));

    let Statement::FunctionDeclaration(void_function) = &program.statements[1] else {
        panic!("expected a function declaration");
    };
    assert_eq!(void_function.return_type, TypeName::Void);

    let Statement::Return(return_statement) = &void_function.body.statements[0] else {
        panic!("expected a return statement");
    };
    assert!(return_statement.value.is_none());
}

#[test]
fn parse_call_expressions_in_initializers_returns_and_statements() {
    let source = r#"export {};

function add(a: int, b: int): int {
  return a + b;
}

function orchestrate(): int {
  const first: int = add(1, 2);
  add(first, 3);
  return add(first, add(4, 5));
}
"#;

    let program = parse_source(source).expect("parser should succeed");

    let Statement::FunctionDeclaration(function) = &program.statements[1] else {
        panic!("expected a function declaration");
    };

    let Statement::VariableDeclaration(declaration) = &function.body.statements[0] else {
        panic!("expected a variable declaration");
    };
    let Expression::Call(initializer_call) = &declaration.initializer else {
        panic!("expected a call expression in the initializer");
    };
    assert_eq!(initializer_call.arguments.len(), 2);

    let Statement::Expression(expression_statement) = &function.body.statements[1] else {
        panic!("expected an expression statement");
    };
    assert!(matches!(
        expression_statement.expression,
        Expression::Call(_)
    ));

    let Statement::Return(return_statement) = &function.body.statements[2] else {
        panic!("expected a return statement");
    };
    let Some(Expression::Call(return_call)) = &return_statement.value else {
        panic!("expected a call expression in the return statement");
    };
    assert_eq!(return_call.arguments.len(), 2);
    assert!(matches!(return_call.arguments[1], Expression::Call(_)));
}

#[test]
fn parse_console_log_expression_statement() {
    let source = r#"export {};

function run(): void {
  console.log(1);
  return;
}
"#;

    let program = parse_source(source).expect("parser should succeed");
    let Statement::FunctionDeclaration(function) = &program.statements[0] else {
        panic!("expected function declaration");
    };
    let Statement::Expression(expression_statement) = &function.body.statements[0] else {
        panic!("expected expression statement");
    };

    let Expression::ConsoleLog(console_log) = &expression_statement.expression else {
        panic!("expected console.log expression");
    };
    assert_eq!(console_log.arguments.len(), 1);
    assert!(matches!(console_log.arguments[0], Expression::IntegerLiteral(_)));
}

#[test]
fn parse_reports_line_and_column_for_syntax_errors() {
    let error = parse_source("export {};\nconst value: int = 1\n")
        .expect_err("parser should reject a missing semicolon");

    assert_eq!(error.line, 3);
    assert_eq!(error.column, 1);
    assert!(
        error
            .message
            .contains("expected ';' after variable declaration")
    );
}

#[test]
fn parse_reports_parameter_syntax_errors() {
    let error = parse_source("export {};\nfunction add(a int): int { return a; }\n")
        .expect_err("parser should reject a parameter without a colon");

    assert!(error.message.contains("expected ':' after parameter name"));
}

#[test]
fn parse_reports_call_argument_syntax_errors() {
    let error = parse_source("export {};\nfunction f(): void { add(1, ); }\n")
        .expect_err("parser should reject a trailing comma without an argument");

    assert!(error.message.contains("expected expression"));
}

#[test]
fn parse_reports_unsupported_property_access() {
    let error = parse_source("export {};\nfunction run(): void { other.log(1); }\n")
        .expect_err("parser should reject unsupported property access");

    assert!(
        error
            .message
            .contains("only 'console.log(...)' property access is supported in v0")
    );
}
