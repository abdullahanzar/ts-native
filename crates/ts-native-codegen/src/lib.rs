#[cfg(feature = "llvm")]
mod llvm_backend;
#[cfg(feature = "llvm")]
mod native;
mod text_ir;

use std::path::PathBuf;

use thiserror::Error;
use ts_native_ir::IrModule;

#[cfg(test)]
use ts_native_parser as _;
#[cfg(test)]
use ts_native_types as _;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CodegenError {
    #[error("{message}")]
    Message { message: String },
    #[error(
        "native artifact emission requires an LLVM-enabled build; rebuild with the `llvm` feature enabled"
    )]
    BackendUnavailable,
}

impl CodegenError {
    pub(crate) fn message(message: impl Into<String>) -> Self {
        Self::Message {
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeArtifactKind {
    Object,
    Executable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeArtifactOptions {
    pub output_path: PathBuf,
    pub kind: NativeArtifactKind,
    pub keep_object: bool,
    pub linker: Option<PathBuf>,
}

impl NativeArtifactOptions {
    pub fn object(output_path: PathBuf) -> Self {
        Self {
            output_path,
            kind: NativeArtifactKind::Object,
            keep_object: false,
            linker: None,
        }
    }

    pub fn executable(output_path: PathBuf) -> Self {
        Self {
            output_path,
            kind: NativeArtifactKind::Executable,
            keep_object: false,
            linker: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeArtifact {
    pub output_path: PathBuf,
    pub kind: NativeArtifactKind,
    pub retained_object_path: Option<PathBuf>,
}

/// Code generation entry point for TS-Native IR to LLVM IR.
pub fn emit_llvm_ir(module: &IrModule) -> Result<String, CodegenError> {
    text_ir::emit_llvm_ir(module)
}

/// Native artifact generation entry point for TS-Native IR.
pub fn compile_native_artifact(
    module: &IrModule,
    options: &NativeArtifactOptions,
) -> Result<NativeArtifact, CodegenError> {
    compile_native_artifact_impl(module, options)
}

#[cfg(feature = "llvm")]
pub const LLVM_BACKEND_ENABLED: bool = true;

#[cfg(not(feature = "llvm"))]
pub const LLVM_BACKEND_ENABLED: bool = false;

#[cfg(feature = "llvm")]
fn compile_native_artifact_impl(
    module: &IrModule,
    options: &NativeArtifactOptions,
) -> Result<NativeArtifact, CodegenError> {
    native::compile_native_artifact(module, options)
}

#[cfg(not(feature = "llvm"))]
fn compile_native_artifact_impl(
    _module: &IrModule,
    _options: &NativeArtifactOptions,
) -> Result<NativeArtifact, CodegenError> {
    Err(CodegenError::BackendUnavailable)
}
