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
The `ts-native-codegen` crate uses an optional `inkwell` dependency gated by feature `llvm`.

Enable it with:

```bash
cargo check -p ts-native-codegen --features llvm
```

If `llvm-config` is not installed or points to an unsupported version, LLVM-enabled checks will fail.
