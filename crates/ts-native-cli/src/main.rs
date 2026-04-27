use clap::{Parser, ValueEnum};
use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};
use ts_native_codegen::{
    CodegenError, NativeArtifactOptions, compile_native_artifact, emit_llvm_ir,
};
use ts_native_ir::lower_to_tsn_ir;
use ts_native_parser::parse_source;
use ts_native_types::type_check;

#[derive(Debug, Clone, ValueEnum)]
enum EmitMode {
    Ast,
    TsnIr,
    LlvmIr,
    Native,
}

#[derive(Debug, Parser)]
#[command(name = "tsn", version, about = "TS-Native compiler driver")]
struct Cli {
    #[arg(help = "Input TS-Native source file")]
    input: String,

    #[arg(long, value_enum, default_value_t = EmitMode::LlvmIr)]
    emit: EmitMode,

    #[arg(long, help = "Output path for native artifact emission")]
    output: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let source = match fs::read_to_string(&cli.input) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("failed to read {}: {error}", cli.input);
            return ExitCode::FAILURE;
        }
    };

    let program = match parse_source(&source) {
        Ok(program) => program,
        Err(error) => {
            eprintln!("parse error in {}: {error}", cli.input);
            return ExitCode::FAILURE;
        }
    };

    match cli.emit {
        EmitMode::Ast => {
            println!("{program:#?}");
            ExitCode::SUCCESS
        }
        EmitMode::TsnIr => match type_check_and_lower(&program, &source, &cli.input) {
            Ok(module) => {
                println!("{module}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                report_compile_error(&cli.input, error);
                ExitCode::FAILURE
            }
        },
        EmitMode::LlvmIr => {
            let module = match type_check_and_lower(&program, &source, &cli.input) {
                Ok(module) => module,
                Err(error) => {
                    report_compile_error(&cli.input, error);
                    return ExitCode::FAILURE;
                }
            };

            match emit_llvm_ir(&module) {
                Ok(llvm_ir) => {
                    println!("{llvm_ir}");
                }
                Err(error) => {
                    eprintln!(
                        "LLVM IR emission failed after parsing, type checking, and IR lowering completed successfully: {error}"
                    );
                    return ExitCode::FAILURE;
                }
            }

            ExitCode::SUCCESS
        }
        EmitMode::Native => {
            let module = match type_check_and_lower(&program, &source, &cli.input) {
                Ok(module) => module,
                Err(error) => {
                    report_compile_error(&cli.input, error);
                    return ExitCode::FAILURE;
                }
            };

            let output_path = cli
                .output
                .clone()
                .unwrap_or_else(|| default_native_output_path(Path::new(&cli.input)));
            let options = NativeArtifactOptions::executable(output_path.clone());

            match compile_native_artifact(&module, &options) {
                Ok(artifact) => {
                    println!(
                        "wrote native executable to {}",
                        artifact.output_path.display()
                    );
                    ExitCode::SUCCESS
                }
                Err(CodegenError::BackendUnavailable) => {
                    eprintln!(
                        "native artifact emission requires an LLVM-enabled build; try: cargo run -p ts-native-cli --features llvm -- {} --emit native --output {}",
                        cli.input,
                        output_path.display()
                    );
                    ExitCode::FAILURE
                }
                Err(error) => {
                    eprintln!(
                        "native artifact emission failed after parsing, type checking, and IR lowering completed successfully: {error}"
                    );
                    ExitCode::FAILURE
                }
            }
        }
    }
}

fn type_check_and_lower(
    program: &ts_native_parser::ast::Program,
    source: &str,
    input_path: &str,
) -> Result<ts_native_ir::IrModule, String> {
    let typed_program = type_check(program, source)
        .map_err(|error| format!("type error in {input_path}: {error}"))?;

    lower_to_tsn_ir(&typed_program)
        .map_err(|error| format!("IR lowering error in {input_path}: {error}"))
}

fn report_compile_error(input_path: &str, error: String) {
    if error.contains(input_path) {
        eprintln!("{error}");
    } else {
        eprintln!("compile error in {input_path}: {error}");
    }
}

fn default_native_output_path(input_path: &Path) -> PathBuf {
    let mut output_path = input_path.to_path_buf();
    let file_name = input_path
        .file_stem()
        .or_else(|| input_path.file_name())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("tsn-output"));
    output_path.set_file_name(file_name);
    output_path
}
