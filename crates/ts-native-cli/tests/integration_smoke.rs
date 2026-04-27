use std::{
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use clap as _;
use ts_native_codegen as _;
use ts_native_ir as _;
use ts_native_parser as _;
use ts_native_types as _;

fn workspace_path(relative_path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative_path)
}

fn unique_temp_path(stem: &str, extension: &str) -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0_u128, |duration| duration.as_nanos());

    std::env::temp_dir().join(format!(
        "{stem}-{}-{unique_suffix}.{extension}",
        std::process::id()
    ))
}

#[test]
fn emits_tsn_ir_for_simple_program() {
    let output = Command::new(env!("CARGO_BIN_EXE_ts-native-cli"))
        .arg(workspace_path(
            "tests/fixtures/programs/pass/simple_const.ts",
        ))
        .arg("--emit")
        .arg("tsn-ir")
        .output()
        .expect("cli should run");

    assert!(
        output.status.success(),
        "expected success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("module {"), "unexpected stdout: {stdout}");
    assert!(
        stdout.contains("const x: int = 10;"),
        "unexpected stdout: {stdout}"
    );
}

#[test]
fn llvm_ir_mode_emits_llvm_ir_after_lowering() {
    let output = Command::new(env!("CARGO_BIN_EXE_ts-native-cli"))
        .arg(workspace_path("examples/functions.ts"))
        .arg("--emit")
        .arg("llvm-ir")
        .output()
        .expect("cli should run");

    assert!(
        output.status.success(),
        "expected LLVM IR emission success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("define i64 @add(i64 %a, i64 %b) {"),
        "unexpected stdout: {stdout}"
    );
    assert!(
        stdout.contains("call i64 @add(i64 1, i64 2)"),
        "unexpected stdout: {stdout}"
    );
}

#[cfg(not(feature = "llvm"))]
#[test]
fn native_mode_reports_missing_llvm_backend_without_feature() {
    let output_path = unique_temp_path("ts-native-native-missing-backend", "bin");
    let output = Command::new(env!("CARGO_BIN_EXE_ts-native-cli"))
        .arg(workspace_path(
            "tests/fixtures/programs/pass/simple_const.ts",
        ))
        .arg("--emit")
        .arg("native")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("cli should run");

    assert!(
        !output.status.success(),
        "expected native mode to fail without llvm feature"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(
        stderr.contains("native artifact emission requires an LLVM-enabled build"),
        "unexpected stderr: {stderr}"
    );
}

#[cfg(feature = "llvm")]
#[test]
fn native_mode_emits_host_executable() {
    let output_path = unique_temp_path("ts-native-native-executable", "bin");
    let output = Command::new(env!("CARGO_BIN_EXE_ts-native-cli"))
        .arg(workspace_path("examples/functions.ts"))
        .arg("--emit")
        .arg("native")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("cli should run");

    assert!(
        output.status.success(),
        "expected native artifact emission success, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output_path.exists(),
        "native executable should exist at {}",
        output_path.display()
    );

    let run_output = Command::new(&output_path)
        .output()
        .expect("native executable should run");
    assert!(
        run_output.status.success(),
        "native executable should exit successfully"
    );

    let _ = std::fs::remove_file(output_path);
}
