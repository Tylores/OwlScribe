use crate::mcp::protocol::{McpTool, McpToolCallResult};
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
}

pub fn tool_definition() -> McpTool {
    McpTool {
        name: "parse_pdf_to_terms".to_string(),
        description: "Extracts raw text and standard sections from specification PDFs (ISO, IEEE, W3C, NIST, RFCs) and identifies candidate domain terms and definitions based on McGuinness 7-Step Ontology Development (Steps 1-3: Domain/Scope, Reusability, Term Enumeration).".to_string(),
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

    match TermExtractor::parse_pdf(&path, args.spec_type, args.min_confidence) {
        Ok(result) => match serde_json::to_string_pretty(&result) {
            Ok(json_str) => McpToolCallResult::text(json_str),
            Err(e) => McpToolCallResult::error(format!("Failed to serialize result: {}", e)),
        },
        Err(e) => McpToolCallResult::error(format!("PDF parsing failed: {:#}", e)),
    }
}
