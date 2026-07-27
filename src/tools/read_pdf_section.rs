use crate::mcp::protocol::{McpTool, McpToolCallResult};
use crate::ontology::base_ontology::BaseOntologyLoader;
use crate::parser::spec_profile::SpecType;
use crate::parser::term_extractor::TermExtractor;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use crate::ontology::base_ontology::BaseOntologySeed;

#[derive(Debug, Deserialize)]
pub struct ReadPdfSectionArgs {
    pub pdf_path: String,
    pub section_id: Option<String>,
    pub section_title: Option<String>,
    pub page_start: Option<usize>,
    pub page_end: Option<usize>,
    pub spec_type: Option<SpecType>,
    pub min_confidence: Option<f64>,
    pub base_ontology_path: Option<String>,
    pub base_ontology_seed: Option<BaseOntologySeed>,
}

pub fn tool_definition() -> McpTool {
    McpTool {
        name: "read_pdf_section".to_string(),
        description: "Phase 2: Retrieves targeted section text and section candidate terms from standard specification PDFs by section_id, section_title, or page_start/page_end rather than dumping the raw document text.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "pdf_path": {
                    "type": "string",
                    "description": "Absolute or relative file path to the specification PDF."
                },
                "section_id": {
                    "type": "string",
                    "description": "Optional section ID (e.g. 'sec_3' or '3') as returned by get_pdf_sections."
                },
                "section_title": {
                    "type": "string",
                    "description": "Optional section title query (e.g. '3. Terms and Definitions' or 'Architecture')."
                },
                "page_start": {
                    "type": "integer",
                    "description": "Optional starting page number (1-based)."
                },
                "page_end": {
                    "type": "integer",
                    "description": "Optional ending page number (1-based)."
                },
                "spec_type": {
                    "type": "string",
                    "enum": ["iso", "ieee", "w3c", "nist", "rfc", "auto"],
                    "description": "Optional specification profile override."
                },
                "min_confidence": {
                    "type": "number",
                    "description": "Minimum confidence for candidate term extractions (0.0 to 1.0). Defaults to 0.3."
                },
                "base_ontology_path": {
                    "type": "string",
                    "description": "Optional path to base domain ontology to align candidate terms."
                },
                "base_ontology_seed": {
                    "type": "object",
                    "description": "Optional base ontology seed object to align candidate terms."
                }
            },
            "required": ["pdf_path"]
        }),
    }
}

pub fn execute(args: ReadPdfSectionArgs) -> McpToolCallResult {
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

    match TermExtractor::read_pdf_section(
        &path,
        args.section_id.as_deref(),
        args.section_title.as_deref(),
        args.page_start,
        args.page_end,
        args.spec_type,
        args.min_confidence,
        loaded_seed.as_ref(),
    ) {
        Ok(result) => match serde_json::to_string_pretty(&result) {
            Ok(json_str) => McpToolCallResult::text(json_str),
            Err(e) => McpToolCallResult::error(format!("Failed to serialize section result: {}", e)),
        },
        Err(e) => McpToolCallResult::error(format!("Failed to read PDF section: {:#}", e)),
    }
}
