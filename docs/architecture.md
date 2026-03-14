# Architecture

TS-Native is implemented as a Rust Cargo workspace with explicit stage boundaries that mirror the specification.

## Compilation Pipeline

```text
TypeScript-like Source
        |
        v
Lexer + Parser
        |
        v
Type Checker
        |
        v
TS-Native IR
        |
        v
LLVM IR
        |
        v
Machine Code (via LLVM toolchain)
```

## Workspace Crates
- `ts-native-parser`: lexical analysis, parsing, and AST construction.
- `ts-native-types`: semantic analysis and type checking over parser output.
- `ts-native-ir`: lowering from typed program representation to TS-Native IR.
- `ts-native-codegen`: TS-Native IR to LLVM IR translation.
- `ts-native-cli`: end-user compiler driver and diagnostics orchestration.

## Design Notes
- Crate boundaries are intentionally strict to reduce coupling and simplify testing.
- LLVM integration is feature-gated behind the `llvm` feature in `ts-native-codegen`.
- v0 favors deterministic semantics and explicit lowering over language surface breadth.
