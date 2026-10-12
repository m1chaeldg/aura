pub mod cst;
pub mod nom_parser;
pub mod serializer;

pub use cst::*;
pub use nom_parser::parse_document as parse;
pub use serializer::serialize;

/// Parses a `.aura` format string directly into the runtime `AuraRequest` model.
pub fn parse_request(input: &str) -> Result<aura_core::model::AuraRequest, String> {
    parse(input).and_then(|cst| cst.to_aura_request())
}

/// Parses a `.aura` format string directly into the lossless file `AuraFileRequest` model.
pub fn parse_file_request(input: &str) -> Result<aura_core::model::AuraFileRequest, String> {
    parse(input).and_then(|cst| cst.to_request())
}
