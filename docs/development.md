# Development Guide

This repository targets Linux/macOS development with Rust stable and LLVM installed locally.

## Prerequisites
- Rust toolchain (stable)
- Cargo
- LLVM toolchain (`llvm-config`, `opt`, `llc`)

## Bootstrap

```bash
./scripts/bootstrap.sh
```

## Common Commands

```bash
./scripts/build.sh
./scripts/test.sh
./scripts/lint.sh
```

## Workspace Layout
- `crates/`: compiler components
- `docs/`: specification and technical documentation
- `examples/`: TS-Native example input programs
- `tests/fixtures/`: pass/fail fixture sets for integration testing

## LLVM Integration
Textual LLVM IR emission via `tsn --emit llvm-ir` works in the default build and does not require a local LLVM toolchain.

Host-native artifact emission uses an optional `inkwell` dependency gated by feature `llvm`, plus the host C toolchain for linking.

Enable it with:

```bash
cargo check -p ts-native-codegen --features llvm
cargo test -p ts-native-codegen --features llvm
cargo test -p ts-native-cli --features llvm
cargo run -p ts-native-cli --features llvm -- examples/fibonacci.ts --emit native --output /tmp/fibonacci
```

If `llvm-config` is not installed, points to an unsupported version, or the host linker (`cc`) is unavailable, LLVM-enabled native emission checks will fail.
