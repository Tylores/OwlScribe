pub mod generate_owl;
pub mod parse_pdf;

use crate::mcp::protocol::{McpTool, McpToolCallResult};
use serde_json::Value;

pub fn list_tools() -> Vec<McpTool> {
    vec![parse_pdf::tool_definition(), generate_owl::tool_definition()]
}

pub fn call_tool(name: &str, arguments: Option<Value>) -> McpToolCallResult {
    let args_val = arguments.unwrap_or_else(|| Value::Object(serde_json::Map::new()));

    match name {
        "parse_pdf_to_terms" => match serde_json::from_value(args_val) {
            Ok(args) => parse_pdf::execute(args),
            Err(e) => McpToolCallResult::error(format!("Invalid arguments for parse_pdf_to_terms: {}", e)),
        },
        "generate_owl_ontology" => match serde_json::from_value(args_val) {
            Ok(args) => generate_owl::execute(args),
            Err(e) => McpToolCallResult::error(format!("Invalid arguments for generate_owl_ontology: {}", e)),
        },
        _ => McpToolCallResult::error(format!("Unknown tool: {}", name)),
    }
}
