#!/usr/bin/env bash
set -euo pipefail

if ! command -v rustc >/dev/null 2>&1; then
  echo "rustc not found. Install Rust from https://rustup.rs"
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found. Install Rust from https://rustup.rs"
  exit 1
fi

if ! command -v llvm-config >/dev/null 2>&1; then
  echo "llvm-config not found. Install LLVM for LLVM-enabled codegen checks."
  echo "Continuing without LLVM tooling."
  exit 0
fi

LLVM_VERSION="$(llvm-config --version || true)"
echo "Found llvm-config version: ${LLVM_VERSION}"
