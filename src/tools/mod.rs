pub mod generate_owl;
pub mod get_pdf_sections;
pub mod propose_ontology_terms;
pub mod read_pdf_section;

use crate::mcp::protocol::{McpTool, McpToolCallResult};
use serde_json::Value;

pub fn list_tools() -> Vec<McpTool> {
    vec![
        get_pdf_sections::tool_definition(),
        read_pdf_section::tool_definition(),
        propose_ontology_terms::tool_definition(),
        generate_owl::tool_definition(),
    ]
}

pub fn call_tool(name: &str, arguments: Option<Value>) -> McpToolCallResult {
    let args_val = arguments.unwrap_or_else(|| Value::Object(serde_json::Map::new()));

    match name {
        "get_pdf_sections" | "get_pdf_toc" => match serde_json::from_value(args_val) {
            Ok(args) => get_pdf_sections::execute(args),
            Err(e) => McpToolCallResult::error(format!("Invalid arguments for get_pdf_sections: {}", e)),
        },
        "read_pdf_section" => match serde_json::from_value(args_val) {
            Ok(args) => read_pdf_section::execute(args),
            Err(e) => McpToolCallResult::error(format!("Invalid arguments for read_pdf_section: {}", e)),
        },
        "propose_ontology_terms" => match serde_json::from_value(args_val) {
            Ok(args) => propose_ontology_terms::execute(args),
            Err(e) => McpToolCallResult::error(format!("Invalid arguments for propose_ontology_terms: {}", e)),
        },
        "generate_owl_ontology" => match serde_json::from_value(args_val) {
            Ok(args) => generate_owl::execute(args),
            Err(e) => McpToolCallResult::error(format!("Invalid arguments for generate_owl_ontology: {}", e)),
        },
        _ => McpToolCallResult::error(format!("Unknown tool: {}", name)),
    }
}
