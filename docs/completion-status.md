# TS-Native Completion Status

This file tracks what is actually completed in the repository. Update it as features land.

## How To Use This Tracker
- Check an item only when the code, tests, and any required documentation have landed.
- Keep the snapshot table aligned with the checklists below it.
- Update the `Last reviewed` date whenever you make a meaningful status change.

## Snapshot

Last reviewed: 2026-04-27

The repository is still in the bootstrap phase, so progress is tracked by area instead of as a single overall percentage. A blended percentage would overstate compiler functionality because the workspace and documentation scaffolding are much further along than the implementation itself.

| Area | Completed | Current status | Evidence |
| --- | --- | --- | --- |
| Repository foundation | 6/6 | Complete | Cargo workspace, scripts, docs, examples, and fixture layout are present. |
| Specification | 6/6 | Current documented scope complete | Versioned spec structure and the current v0 chapters are written. |
| CLI and tooling | 4/4 | Native-aware compiler driver | CLI reads input files, reports parse and type failures, emits AST, TS-Native IR, textual LLVM IR, and host-native artifacts in LLVM-enabled builds. |
| Parser | 5/5 | Complete for current v0 syntax slice | Lexer, AST, recursive-descent parsing, diagnostics, and parser tests are implemented for declarations, loops, function declarations, return statements, and call expressions. |
| Type checker | 5/5 | Complete for current v0 syntax slice | Semantic analysis, primitive typing, binding analysis, function/call checking, return checking, inference, and tests are implemented for the current parser surface area. |
| IR lowering | 4/4 | Complete for current v0 syntax slice | TS-Native IR data structures, lowering, formatting, and tests are implemented for the current typed syntax surface. |
| Code generation | 4/4 | Complete for current v0 syntax slice | Textual LLVM IR emission, host-native object/executable emission, and backend tests are implemented for the current typed IR surface. |
| Testing and automation | 5/5 | Compiler integration coverage landed | Build, test, lint, CI, CLI smoke tests, and feature-gated native output tests cover AST, IR, LLVM IR, and native flows. |

## Current Baseline

- The Rust workspace builds and `cargo test --workspace` passes.
- The parser stage now tokenizes and parses the current v0 syntax slice into a structured AST with source-located failures, including typed function declarations, return statements, call expressions, and standalone call statements.
- The type checker now validates primitive operations, lexical bindings, assignments, while conditions, returns, and named function calls over the current syntax tree.
- `tsn --emit ast` reads source files, parses them, and prints the resulting AST; `tsn --emit tsn-ir` runs semantic checking and IR lowering; `tsn --emit llvm-ir` prints textual LLVM IR for the current v0 IR surface.
- `tsn --emit native` now emits a host executable in LLVM-enabled builds by lowering typed IR to an object file and linking it with the local host toolchain.
- Automated coverage now includes the CLI smoke tests plus parser, type-checker, IR lowering, textual LLVM IR, and feature-gated native artifact tests.

## Checklist

### Repository Foundation
- [x] Cargo workspace created for parser, types, IR, codegen, and CLI crates
- [x] Shared workspace lint and package configuration added
- [x] Bootstrap, build, test, and lint scripts added
- [x] Architecture and development documentation added
- [x] Example TS-Native source files added
- [x] Test fixture directory structure added

### Specification
- [x] Versioned specification directory structure added
- [x] Specification index added
- [x] v0 specification index added
- [x] v0 introduction chapter written
- [x] v0 primitive types chapter written
- [x] v0 variables and bindings chapter written

### CLI And Tooling
- [x] `tsn` CLI crate created
- [x] CLI argument parsing for input and `--emit` modes added
- [x] CLI invokes parser, type checker, IR lowering, and codegen stages
- [x] CLI reports structured diagnostics for user-facing failures

### Parser
- [x] Lexer or token model implemented
- [x] AST or parse tree model implemented
- [x] `parse_source` returns a structured parse result
- [x] Parser diagnostics and error handling implemented
- [x] Parser unit or fixture tests added

### Type Checker
- [x] Typed program or semantic model implemented
- [x] Primitive type checking implemented
- [x] Variable and binding analysis implemented
- [x] Type inference implemented for supported language forms
- [x] Type checker tests added

### IR Lowering
- [x] TS-Native IR data structures implemented
- [x] Lowering from typed program to TS-Native IR implemented
- [x] IR inspection, formatting, or validation support added
- [x] IR tests added

### Code Generation
- [x] Optional LLVM dependency and feature gate configured
- [x] LLVM IR emission implemented
- [x] Native artifact emission implemented
- [x] Code generation tests added

### Testing And Automation
- [x] Workspace build script added
- [x] Workspace test script added
- [x] CI workflow runs bootstrap, build, test, and lint steps
- [x] Fixture-driven compiler integration tests added
- [x] Golden output tests added for AST, IR, or LLVM output

## Evidence

- Workspace layout and current status: [../README.md](../README.md)
- Architecture overview: [architecture.md](architecture.md)
- Development workflow: [development.md](development.md)
- Specification index: [specification/README.md](specification/README.md)
- v0 specification index: [specification/v0/README.md](specification/v0/README.md)
- CLI entry point: [../crates/ts-native-cli/src/main.rs](../crates/ts-native-cli/src/main.rs)
- Parser entry point: [../crates/ts-native-parser/src/lib.rs](../crates/ts-native-parser/src/lib.rs)
- Parser AST model: [../crates/ts-native-parser/src/ast.rs](../crates/ts-native-parser/src/ast.rs)
- Parser implementation: [../crates/ts-native-parser/src/parser.rs](../crates/ts-native-parser/src/parser.rs)
- Lexer implementation: [../crates/ts-native-parser/src/lexer.rs](../crates/ts-native-parser/src/lexer.rs)
- Type checker entry point: [../crates/ts-native-types/src/lib.rs](../crates/ts-native-types/src/lib.rs)
- Type checker tests: [../crates/ts-native-types/tests/type_checker.rs](../crates/ts-native-types/tests/type_checker.rs)
- IR entry point: [../crates/ts-native-ir/src/lib.rs](../crates/ts-native-ir/src/lib.rs)
- IR tests: [../crates/ts-native-ir/tests/lowering.rs](../crates/ts-native-ir/tests/lowering.rs)
- Codegen entry point: [../crates/ts-native-codegen/src/lib.rs](../crates/ts-native-codegen/src/lib.rs)
- Native codegen backend: [../crates/ts-native-codegen/src/llvm_backend.rs](../crates/ts-native-codegen/src/llvm_backend.rs)
- Native artifact orchestration: [../crates/ts-native-codegen/src/native.rs](../crates/ts-native-codegen/src/native.rs)
- Codegen native tests: [../crates/ts-native-codegen/tests/native_artifact.rs](../crates/ts-native-codegen/tests/native_artifact.rs)
- Smoke test: [../crates/ts-native-cli/tests/integration_smoke.rs](../crates/ts-native-cli/tests/integration_smoke.rs)
- Parser tests: [../crates/ts-native-parser/tests/parser.rs](../crates/ts-native-parser/tests/parser.rs)
- CI workflow: [../.github/workflows/test.yml](../.github/workflows/test.yml)