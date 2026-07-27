use crate::mcp::protocol::{McpTool, McpToolCallResult};
use crate::parser::spec_profile::SpecType;
use crate::parser::term_extractor::TermExtractor;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct GetPdfSectionsArgs {
    pub pdf_path: String,
    pub spec_type: Option<SpecType>,
}

pub fn tool_definition() -> McpTool {
    McpTool {
        name: "get_pdf_sections".to_string(),
        description: "Phase 1: Returns section titles, section IDs, page ranges, and preview snippets from standard specification PDFs (ISO, IEEE, W3C, NIST, RFCs) so the agent can selectively inspect target standard sections (e.g., Section 3: Definitions & Acronyms, Section 5: Architecture). Alias: get_pdf_toc.".to_string(),
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
                }
            },
            "required": ["pdf_path"]
        }),
    }
}

pub fn execute(args: GetPdfSectionsArgs) -> McpToolCallResult {
    let path = PathBuf::from(&args.pdf_path);
    if !path.exists() {
        return McpToolCallResult::error(format!(
            "PDF file not found at path: '{}'",
            args.pdf_path
        ));
    }

    match TermExtractor::get_pdf_sections(&path, args.spec_type) {
        Ok(result) => match serde_json::to_string_pretty(&result) {
            Ok(json_str) => McpToolCallResult::text(json_str),
            Err(e) => McpToolCallResult::error(format!("Failed to serialize sections result: {}", e)),
        },
        Err(e) => McpToolCallResult::error(format!("Failed to extract PDF sections: {:#}", e)),
    }
}
