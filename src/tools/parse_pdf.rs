use crate::mcp::protocol::{McpTool, McpToolCallResult};
use crate::ontology::base_ontology::{BaseOntologyLoader, BaseOntologySeed};
use crate::parser::spec_profile::SpecType;
use crate::parser::term_extractor::TermExtractor;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct ParsePdfToTermsArgs {
    pub pdf_path: String,
    pub spec_type: Option<SpecType>,
    pub min_confidence: Option<f64>,
    pub base_ontology_path: Option<String>,
    pub base_ontology_seed: Option<BaseOntologySeed>,
}

pub fn tool_definition() -> McpTool {
    McpTool {
        name: "parse_pdf_to_terms".to_string(),
        description: "Extracts raw text and standard sections from specification PDFs (ISO, IEEE, W3C, NIST, RFCs), identifies candidate domain terms, and optionally aligns with a base domain ontology seed (Base-First extraction).".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "pdf_path": {
                    "type": "string",
                    "description": "Absolute or relative file path to the specification PDF."
                },
                "spec_type": {
                    "type": "string",
                    "enum": ["iso", "ieee", "w3c", "nist", "rfc", "auto"],
                    "description": "Optional specification format profile override. Defaults to 'auto'."
                },
                "min_confidence": {
                    "type": "number",
                    "description": "Minimum confidence threshold for term candidate extraction (0.0 to 1.0). Defaults to 0.3."
                },
                "base_ontology_path": {
                    "type": "string",
                    "description": "Optional path to an existing base ontology (.ofn format) to guide term extraction and mapping."
                },
                "base_ontology_seed": {
                    "type": "object",
                    "description": "Optional structured base ontology seed summary (top classes and key properties) for guiding concepts."
                }
            },
            "required": ["pdf_path"]
        }),
    }
}

pub fn execute(args: ParsePdfToTermsArgs) -> McpToolCallResult {
    let path = PathBuf::from(&args.pdf_path);
    if !path.exists() {
        return McpToolCallResult::error(format!(
            "PDF file not found at path: '{}'",
            args.pdf_path
        ));
    }

    let loaded_seed = if let Some(ref base_path) = args.base_ontology_path {
        match BaseOntologyLoader::from_file(base_path) {
            Ok((_ont, seed)) => Some(seed),
            Err(e) => return McpToolCallResult::error(format!("Failed to load base ontology: {:#}", e)),
        }
    } else {

        args.base_ontology_seed
    };

    match TermExtractor::parse_pdf(&path, args.spec_type, args.min_confidence, loaded_seed.as_ref()) {
        Ok(result) => match serde_json::to_string_pretty(&result) {
            Ok(json_str) => McpToolCallResult::text(json_str),
            Err(e) => McpToolCallResult::error(format!("Failed to serialize result: {}", e)),
        },
        Err(e) => McpToolCallResult::error(format!("PDF parsing failed: {:#}", e)),
    }
}
