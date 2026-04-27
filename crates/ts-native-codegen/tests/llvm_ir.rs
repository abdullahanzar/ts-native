use std::{fs, path::PathBuf};

use thiserror as _;
use ts_native_codegen::emit_llvm_ir;
use ts_native_ir::lower_to_tsn_ir;
use ts_native_parser::parse_source;
use ts_native_types::type_check;

fn workspace_path(relative_path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative_path)
}

fn compile_to_llvm_ir(relative_path: &str) -> String {
    let source_path = workspace_path(relative_path);
    let source = fs::read_to_string(&source_path).expect("fixture should be readable");
    let program = parse_source(&source).expect("fixture should parse");
    let typed_program = type_check(&program, &source).expect("fixture should type-check");
    let module = lower_to_tsn_ir(&typed_program).expect("fixture should lower to TS-Native IR");

    emit_llvm_ir(&module).expect("fixture should emit LLVM IR")
}

fn compile_source_to_llvm_ir(source: &str) -> String {
    let program = parse_source(source).expect("source should parse");
    let typed_program = type_check(&program, source).expect("source should type-check");
    let module = lower_to_tsn_ir(&typed_program).expect("source should lower to TS-Native IR");

    emit_llvm_ir(&module).expect("source should emit LLVM IR")
}

#[test]
fn emits_function_definitions_and_calls() {
    let llvm_ir = compile_to_llvm_ir("examples/functions.ts");

    assert!(
        llvm_ir.contains("define i64 @add(i64 %a, i64 %b) {"),
        "unexpected LLVM IR: {llvm_ir}"
    );
    assert!(
        llvm_ir.contains("define i64 @run() {"),
        "unexpected LLVM IR: {llvm_ir}"
    );
    assert!(
        llvm_ir.contains("call i64 @add(i64 1, i64 2)"),
        "unexpected LLVM IR: {llvm_ir}"
    );
    assert!(
        llvm_ir.contains("define void @finish() {"),
        "unexpected LLVM IR: {llvm_ir}"
    );
}

#[test]
fn wraps_top_level_statements_in_entry_function() {
    let llvm_ir = compile_to_llvm_ir("examples/fibonacci.ts");

    assert!(
        llvm_ir.contains("define void @__tsn_entry() {"),
        "unexpected LLVM IR: {llvm_ir}"
    );
    assert!(
        llvm_ir.contains("while.cond."),
        "unexpected LLVM IR: {llvm_ir}"
    );
    assert!(llvm_ir.contains("br i1 %"), "unexpected LLVM IR: {llvm_ir}");
    assert!(
        llvm_ir.contains("store i64 0, i64* %a.addr."),
        "unexpected LLVM IR: {llvm_ir}"
    );
}

#[test]
fn emits_builtin_print_runtime_declarations_and_calls() {
    let llvm_ir = compile_source_to_llvm_ir(
        "export {};\nfunction run(): void { printInt(42); printBool(true); return; }\n",
    );

    assert!(
        llvm_ir.contains("declare void @__tsn_print_int(i64)"),
        "unexpected LLVM IR: {llvm_ir}"
    );
    assert!(
        llvm_ir.contains("define void @printInt(i64 %value) {"),
        "unexpected LLVM IR: {llvm_ir}"
    );
    assert!(
        llvm_ir.contains("call void @__tsn_print_int(i64 %value)"),
        "unexpected LLVM IR: {llvm_ir}"
    );
    assert!(
        llvm_ir.contains("define void @printBool(i1 %value) {"),
        "unexpected LLVM IR: {llvm_ir}"
    );
    assert!(
        llvm_ir.contains("zext i1 %value to i64"),
        "unexpected LLVM IR: {llvm_ir}"
    );
}
