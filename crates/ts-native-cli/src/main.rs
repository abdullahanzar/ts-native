use clap::{Parser, ValueEnum};
use ts_native_codegen as _;
use ts_native_ir as _;
use ts_native_parser as _;
use ts_native_types as _;

#[derive(Debug, Clone, ValueEnum)]
enum EmitMode {
    Ast,
    TsnIr,
    LlvmIr,
}

#[derive(Debug, Parser)]
#[command(name = "tsn", version, about = "TS-Native compiler driver")]
struct Cli {
    #[arg(help = "Input TS-Native source file")]
    input: String,

    #[arg(long, value_enum, default_value_t = EmitMode::LlvmIr)]
    emit: EmitMode,
}

fn main() {
    let cli = Cli::parse();
    println!("tsn bootstrap: input={} emit={:?}", cli.input, cli.emit);
}
