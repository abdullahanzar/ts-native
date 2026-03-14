use thiserror as _;

/// Parser crate entry point for TS-Native source parsing.
pub fn parse_source(_source: &str) -> Result<(), &'static str> {
    Err("parser not implemented")
}
