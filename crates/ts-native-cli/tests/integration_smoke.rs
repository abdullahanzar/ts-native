use clap as _;
use ts_native_codegen as _;
use ts_native_ir as _;
use ts_native_parser as _;
use ts_native_types as _;

#[test]
fn cli_smoke() {
    let version = env!("CARGO_PKG_VERSION");
    assert!(!version.is_empty());
}
