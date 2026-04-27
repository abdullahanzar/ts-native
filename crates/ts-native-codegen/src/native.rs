use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use ts_native_ir::IrModule;

use crate::{
    CodegenError, NativeArtifact, NativeArtifactKind, NativeArtifactOptions, llvm_backend,
};

const RUNTIME_SOURCE: &str = "runtime/stdio.c";

pub(crate) fn compile_native_artifact(
    module: &IrModule,
    options: &NativeArtifactOptions,
) -> Result<NativeArtifact, CodegenError> {
    ensure_parent_directory(&options.output_path)?;

    match options.kind {
        NativeArtifactKind::Object => {
            llvm_backend::emit_object_file(module, &options.output_path)?;
            Ok(NativeArtifact {
                output_path: options.output_path.clone(),
                kind: NativeArtifactKind::Object,
                retained_object_path: None,
            })
        }
        NativeArtifactKind::Executable => {
            let object_path = if options.keep_object {
                sibling_object_path(&options.output_path)
            } else {
                temporary_object_path(&options.output_path)
            };
            let runtime_object_path = temporary_runtime_object_path(&options.output_path);

            ensure_parent_directory(&object_path)?;
            llvm_backend::emit_object_file(module, &object_path)?;
            compile_runtime_object(&runtime_object_path, options.linker.as_deref())?;
            link_object_file(
                &[object_path.as_path(), runtime_object_path.as_path()],
                &options.output_path,
                options.linker.as_deref(),
            )?;

            let retained_object_path = if options.keep_object {
                Some(object_path)
            } else {
                let _ = fs::remove_file(&object_path);
                None
            };
            let _ = fs::remove_file(&runtime_object_path);

            Ok(NativeArtifact {
                output_path: options.output_path.clone(),
                kind: NativeArtifactKind::Executable,
                retained_object_path,
            })
        }
    }
}

fn ensure_parent_directory(path: &Path) -> Result<(), CodegenError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|error| {
                CodegenError::message(format!(
                    "failed to create output directory {}: {error}",
                    parent.display()
                ))
            })?;
        }
    }

    Ok(())
}

fn sibling_object_path(output_path: &Path) -> PathBuf {
    output_path.with_extension("o")
}

fn temporary_object_path(output_path: &Path) -> PathBuf {
    let file_stem = output_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("tsn-native");

    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0_u128, |duration| duration.as_nanos());

    std::env::temp_dir().join(format!(
        "{file_stem}.{}.{}.o",
        std::process::id(),
        unique_suffix
    ))
}

fn temporary_runtime_object_path(output_path: &Path) -> PathBuf {
    let mut runtime_path = temporary_object_path(output_path);
    runtime_path.set_file_name(format!(
        "tsn-runtime.{}.{}.o",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0_u128, |duration| duration.as_nanos())
    ));
    runtime_path
}

fn compile_runtime_object(
    output_path: &Path,
    tool_override: Option<&Path>,
) -> Result<(), CodegenError> {
    let tool = tool_override.unwrap_or_else(|| Path::new("cc"));
    let runtime_source = Path::new(env!("CARGO_MANIFEST_DIR")).join(RUNTIME_SOURCE);
    let output = Command::new(tool)
        .arg("-c")
        .arg(&runtime_source)
        .arg("-o")
        .arg(output_path)
        .output()
        .map_err(|error| {
            CodegenError::message(format!(
                "failed to compile runtime source {} with {}: {error}",
                runtime_source.display(),
                tool.display()
            ))
        })?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let details = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };

    Err(CodegenError::message(format!(
        "runtime compilation with {} failed for {} (status {}): {}",
        tool.display(),
        runtime_source.display(),
        output.status,
        details
    )))
}

fn link_object_file(
    object_paths: &[&Path],
    output_path: &Path,
    linker_override: Option<&Path>,
) -> Result<(), CodegenError> {
    let linker = linker_override.unwrap_or_else(|| Path::new("cc"));
    let mut command = Command::new(linker);
    for object_path in object_paths {
        command.arg(object_path);
    }
    let output = command
        .arg("-o")
        .arg(output_path)
        .output()
        .map_err(|error| {
            CodegenError::message(format!(
                "failed to invoke linker {}: {error}",
                linker.display()
            ))
        })?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let details = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };

    let joined_paths = object_paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    Err(CodegenError::message(format!(
        "linker {} failed while producing {} from {} (status {}): {}",
        linker.display(),
        output_path.display(),
        joined_paths,
        output.status,
        details
    )))
}
