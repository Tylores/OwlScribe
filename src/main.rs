pub mod mcp;
pub mod ontology;
pub mod parser;
pub mod tools;

use crate::mcp::protocol::{JsonRpcRequest, JsonRpcResponse, McpToolListResponse};
use anyhow::Result;
use serde_json::json;
use std::io::{self, BufRead, Write};

#[tokio::main]
async fn main() -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();

    let mut line_buffer = String::new();

    while reader.read_line(&mut line_buffer)? > 0 {
        let trimmed = line_buffer.trim();
        if trimmed.is_empty() {
            line_buffer.clear();
            continue;
        }

        if let Ok(request) = serde_json::from_str::<JsonRpcRequest>(trimmed) {
            if let Some(response) = handle_request(request) {
                let response_json = serde_json::to_string(&response)?;
                writeln!(writer, "{}", response_json)?;
                writer.flush()?;
            }
        }

        line_buffer.clear();
    }

    Ok(())
}

fn handle_request(req: JsonRpcRequest) -> Option<JsonRpcResponse> {
    let id = req.id.unwrap_or(serde_json::Value::Null);

    match req.method.as_str() {
        "initialize" => Some(JsonRpcResponse::success(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "owlscribe",
                    "version": "0.1.0"
                }
            }),
        )),
        "notifications/initialized" => None,
        "tools/list" => {
            let tools = tools::list_tools();
            let tool_list = McpToolListResponse { tools };
            Some(JsonRpcResponse::success(id, serde_json::to_value(tool_list).unwrap_or(Value::Null)))
        }
        "tools/call" => {
            let params = req.params.unwrap_or(json!({}));
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = params.get("arguments").cloned();

            let tool_result = tools::call_tool(tool_name, arguments);
            Some(JsonRpcResponse::success(id, serde_json::to_value(tool_result).unwrap_or(Value::Null)))
        }
        _ => Some(JsonRpcResponse::error(id, -32601, format!("Method not found: {}", req.method))),
    }
}
use serde_json::Value;
