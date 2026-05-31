//! `lancer-promptfoo mcp` — expose lancer-promptfoo as a Model Context Protocol
//! server (JSON-RPC 2.0 over stdio, newline-framed) so Claude Code / Cursor /
//! Continue can call `eval`, `redteam_run`, and `assert` as tools.

use crate::core::config::PromptfooConfig;
use crate::core::engine::RunOptions;
use crate::core::types::AssertionConfig;
use anyhow::Result;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const PROTOCOL_VERSION: &str = "2024-11-05";

pub async fn serve_stdio() -> Result<()> {
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();

    while let Some(line) = reader.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                let err = rpc_error(Value::Null, -32700, &format!("parse error: {}", e));
                write_msg(&mut stdout, &err).await?;
                continue;
            }
        };
        if let Some(resp) = handle(&req).await {
            write_msg(&mut stdout, &resp).await?;
        }
    }
    Ok(())
}

async fn write_msg(out: &mut tokio::io::Stdout, msg: &Value) -> Result<()> {
    let s = serde_json::to_string(msg)?;
    out.write_all(s.as_bytes()).await?;
    out.write_all(b"\n").await?;
    out.flush().await?;
    Ok(())
}

fn rpc_result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

async fn handle(req: &Value) -> Option<Value> {
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(json!({}));

    match method {
        "initialize" => Some(rpc_result(
            id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "lancer-promptfoo", "version": env!("CARGO_PKG_VERSION") }
            }),
        )),
        // Notifications (no id) — don't reply.
        "notifications/initialized" | "initialized" => None,
        "ping" => Some(rpc_result(id, json!({}))),
        "tools/list" => Some(rpc_result(id, json!({ "tools": tool_specs() }))),
        "tools/call" => Some(handle_tool_call(id, &params).await),
        _ => Some(rpc_error(id, -32601, &format!("method not found: {}", method))),
    }
}

fn tool_specs() -> Value {
    json!([
        {
            "name": "eval",
            "description": "Run a promptfoo eval from a config file path or inline YAML and return the summary.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "config_path": { "type": "string" },
                    "config_yaml": { "type": "string" }
                }
            }
        },
        {
            "name": "redteam_run",
            "description": "Run a promptfoo red-team scan from a config file path or inline YAML and return findings.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "config_path": { "type": "string" },
                    "config_yaml": { "type": "string" }
                }
            }
        },
        {
            "name": "assert",
            "description": "Evaluate a single deterministic assertion against an output string.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "type": { "type": "string" },
                    "output": { "type": "string" },
                    "value": {}
                },
                "required": ["type", "output"]
            }
        }
    ])
}

async fn handle_tool_call(id: Value, params: &Value) -> Value {
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    let text = match name {
        "eval" => run_eval_tool(&args).await,
        "redteam_run" => run_redteam_tool(&args).await,
        "assert" => run_assert_tool(&args),
        other => Err(format!("unknown tool: {}", other)),
    };

    match text {
        Ok(body) => rpc_result(
            id,
            json!({ "content": [{ "type": "text", "text": body }], "isError": false }),
        ),
        Err(e) => rpc_result(
            id,
            json!({ "content": [{ "type": "text", "text": e }], "isError": true }),
        ),
    }
}

fn load_config_from_args(args: &Value) -> Result<PromptfooConfig, String> {
    if let Some(path) = args.get("config_path").and_then(|p| p.as_str()) {
        return PromptfooConfig::load(std::path::Path::new(path)).map_err(|e| e.to_string());
    }
    if let Some(yaml) = args.get("config_yaml").and_then(|y| y.as_str()) {
        return PromptfooConfig::parse(yaml).map_err(|e| e.to_string());
    }
    Err("provide either config_path or config_yaml".into())
}

async fn run_eval_tool(args: &Value) -> Result<String, String> {
    let cfg = load_config_from_args(args)?;
    let report = crate::core::eval::run(&cfg, RunOptions::default())
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::to_string_pretty(&report.summary).unwrap_or_default())
}

async fn run_redteam_tool(args: &Value) -> Result<String, String> {
    let cfg = load_config_from_args(args)?;
    let report = crate::core::redteam::run(&cfg, RunOptions::default())
        .await
        .map_err(|e| e.to_string())?;
    crate::core::report::render(&report, crate::core::report::OutputFormat::Markdown)
        .map_err(|e| e.to_string())
}

fn run_assert_tool(args: &Value) -> Result<String, String> {
    let kind = args.get("type").and_then(|t| t.as_str()).ok_or("missing type")?;
    let output = args.get("output").and_then(|o| o.as_str()).ok_or("missing output")?;
    let cfg = AssertionConfig {
        kind: kind.to_string(),
        value: args.get("value").cloned(),
        threshold: args.get("threshold").and_then(|t| t.as_f64()),
        ..Default::default()
    };
    let reg = crate::core::assertions::AssertionRegistry::new();
    match reg.evaluate(kind, output, &cfg, 0, 0.0) {
        Some(o) => Ok(serde_json::to_string_pretty(&o).unwrap_or_default()),
        None => Err(format!(
            "assertion '{}' is model-graded/custom and cannot be evaluated without a judge model",
            kind
        )),
    }
}
