use thiserror as _;
use ts_native_ir::lower_to_tsn_ir;
use ts_native_parser::parse_source;
use ts_native_types::type_check;

fn lower_source(source: &str) -> String {
    let program = parse_source(source).expect("source should parse");
    let typed_program = type_check(&program, source).expect("source should type-check");
    let module = lower_to_tsn_ir(&typed_program).expect("source should lower");
    module.to_string()
}

#[test]
fn lowers_simple_const_program() {
    let source = "export {};\nconst x: int = 10;\n";

    let actual = lower_source(source);

    let expected = r#"module {
  const x: int = 10;
}"#;

    assert_eq!(actual, expected);
}

#[test]
fn lowers_explicit_widening_into_casts() {
    let source = r#"export {};

function widen(value: int): double {
  return value;
}

function run(): double {
  const base: double = 1;
  return widen(1) + base;
}
"#;

    let actual = lower_source(source);

    let expected = r#"module {
  function widen(value: int): double {
    return cast<double>(value);
  }

  function run(): double {
    const base: double = cast<double>(1);
    return (widen(1) + base);
  }
}"#;

    assert_eq!(actual, expected);
}

#[test]
fn lowers_builtin_print_calls() {
    let source = r#"export {};

function run(): void {
  printInt(1);
  printDouble(2);
  printBool(true);
  return;
}
"#;

    let actual = lower_source(source);

    let expected = r#"module {
  function run(): void {
    printInt(1);
    printDouble(cast<double>(2));
    printBool(true);
    return;
  }
}"#;

    assert_eq!(actual, expected);
}

#[test]
fn lowers_console_log_sugar_to_builtin_print_calls() {
    let source = r#"export {};

function run(): void {
  console.log(1);
  console.log(2.5);
  console.log(true);
  return;
}
"#;

    let actual = lower_source(source);

    let expected = r#"module {
  function run(): void {
    printInt(1);
    printDouble(2.5);
    printBool(true);
    return;
  }
}"#;

    assert_eq!(actual, expected);
}
