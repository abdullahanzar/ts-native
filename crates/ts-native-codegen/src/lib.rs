use thiserror as _;
use ts_native_ir as _;

/// Code generation entry point for TS-Native IR to LLVM IR.
pub fn emit_llvm_ir() -> Result<(), &'static str> {
    Err("codegen not implemented")
}

#[cfg(feature = "llvm")]
pub const LLVM_BACKEND_ENABLED: bool = true;
