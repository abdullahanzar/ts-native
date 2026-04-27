use thiserror as _;
use ts_native_codegen as _;
use ts_native_ir as _;
use ts_native_parser as _;
use ts_native_types as _;

#[cfg(feature = "llvm")]
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(feature = "llvm")]
use ts_native_codegen::{NativeArtifactKind, NativeArtifactOptions, compile_native_artifact};
#[cfg(feature = "llvm")]
use ts_native_ir::lower_to_tsn_ir;
#[cfg(feature = "llvm")]
use ts_native_parser::parse_source;
#[cfg(feature = "llvm")]
use ts_native_types::type_check;

#[cfg(feature = "llvm")]
fn workspace_path(relative_path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative_path)
}

#[cfg(feature = "llvm")]
fn unique_temp_path(stem: &str, extension: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0_u128, |duration| duration.as_nanos());

    std::env::temp_dir().join(format!(
        "{stem}-{}-{unique_suffix}.{extension}",
        std::process::id()
    ))
}

#[cfg(feature = "llvm")]
fn lower_fixture(relative_path: &str) -> ts_native_ir::IrModule {
    let source_path = workspace_path(relative_path);
    let source = fs::read_to_string(&source_path).expect("fixture should be readable");
    let program = parse_source(&source).expect("fixture should parse");
    let typed_program = type_check(&program, &source).expect("fixture should type-check");
    lower_to_tsn_ir(&typed_program).expect("fixture should lower to TS-Native IR")
}

#[cfg(feature = "llvm")]
fn lower_source(source: &str) -> ts_native_ir::IrModule {
    let program = parse_source(source).expect("source should parse");
    let typed_program = type_check(&program, source).expect("source should type-check");
    lower_to_tsn_ir(&typed_program).expect("source should lower to TS-Native IR")
}

#[cfg(feature = "llvm")]
#[test]
fn emits_object_file_for_top_level_program() {
    let module = lower_fixture("examples/fibonacci.ts");
    let output_path = unique_temp_path("ts-native-native-object", "o");
    let artifact =
        compile_native_artifact(&module, &NativeArtifactOptions::object(output_path.clone()))
            .expect("object emission should succeed with the llvm feature enabled");

    assert_eq!(artifact.kind, NativeArtifactKind::Object);
    assert_eq!(artifact.output_path, output_path);
    let metadata = fs::metadata(&artifact.output_path).expect("object file should exist");
    assert!(metadata.len() > 0, "object file should not be empty");

    let _ = fs::remove_file(&artifact.output_path);
}

#[cfg(feature = "llvm")]
#[test]
fn emits_executable_that_prints_builtin_output() {
    let module = lower_source(
        r#"export {};

function run(): void {
  printInt(42);
  printDouble(2.5);
  printBool(true);
  return;
}

run();
"#,
    );
    let output_path = unique_temp_path("ts-native-native-print", "bin");
    let artifact = compile_native_artifact(&module, &NativeArtifactOptions::executable(output_path.clone()))
        .expect("native executable emission should succeed with builtin runtime support");

    let run_output = Command::new(&artifact.output_path)
        .output()
        .expect("native executable should run");
    assert!(run_output.status.success(), "native executable should exit successfully");

    let stdout = String::from_utf8(run_output.stdout).expect("stdout should be utf-8");
    assert_eq!(stdout, "42\n2.5\ntrue\n");

    let _ = fs::remove_file(&artifact.output_path);
}

#[cfg(feature = "llvm")]
#[test]
fn emits_executable_that_prints_console_log_output() {
    let module = lower_source(
        r#"export {};

function run(): void {
  console.log(42);
  console.log(2.5);
  console.log(true);
  return;
}

run();
"#,
    );
    let output_path = unique_temp_path("ts-native-native-console-log", "bin");
    let artifact = compile_native_artifact(
        &module,
        &NativeArtifactOptions::executable(output_path.clone()),
    )
    .expect("native executable emission should succeed with console.log sugar");

    let run_output = Command::new(&artifact.output_path)
        .output()
        .expect("native executable should run");
    assert!(run_output.status.success(), "native executable should exit successfully");

    let stdout = String::from_utf8(run_output.stdout).expect("stdout should be utf-8");
    assert_eq!(stdout, "42\n2.5\ntrue\n");

    let _ = fs::remove_file(&artifact.output_path);
}
