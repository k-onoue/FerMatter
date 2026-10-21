use super::{ApiError, ApiResult};
use serde_json::{json, Value};
use std::{
    io::Read,
    sync::{atomic::AtomicBool, Arc},
};

/// CLI/MCP exit before constructing a Tauri runtime, window, tray, or Storage.
pub fn entry() -> bool {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !matches!(args.first().map(String::as_str), Some("cli" | "mcp")) {
        return false;
    }
    let identity = serde_json::from_str::<Value>(include_str!("../../tauri.conf.json")).unwrap()
        ["identifier"]
        .as_str()
        .unwrap()
        .to_owned();
    let result = if args[0] == "mcp" {
        if args.len() != 1 {
            Err(ApiError::new("invalid_arguments", "Usage: FerMatter mcp"))
        } else {
            super::mcp::run(identity).map(|_| Value::Null)
        }
    } else {
        execute(&identity, &args[1..])
    };
    match result {
        Ok(value) => {
            if args[0] != "mcp" {
                println!("{}", json!({"result":value}));
            }
        }
        Err(error) => {
            if args[0] == "mcp" {
                eprintln!("{}: {}", error.code, error.message);
            } else {
                println!("{}", json!({"error":error}));
            }
            std::process::exit(1);
        }
    }
    true
}
fn execute(identity: &str, args: &[String]) -> ApiResult {
    if args.is_empty() || matches!(args[0].as_str(), "--help" | "help" | "tools") {
        return Ok(
            json!({"apiVersion":1,"usage":"FerMatter cli TOOL [--input FILE|-]","tools":super::tools()}),
        );
    }
    if !super::tools().iter().any(|t| t.name == args[0]) {
        return Err(ApiError::new(
            "unknown_operation",
            "Use `FerMatter cli tools` to list operations and JSON schemas.",
        ));
    }
    let arguments = match args.len() {
        1 => json!({}),
        3 if args[1] == "--input" => {
            let input: Box<dyn Read> = if args[2] == "-" {
                Box::new(std::io::stdin())
            } else {
                Box::new(
                    std::fs::File::open(&args[2])
                        .map_err(|e| ApiError::new("invalid_arguments", e.to_string()))?,
                )
            };
            let mut bytes = vec![];
            input
                .take(1_048_577)
                .read_to_end(&mut bytes)
                .map_err(|e| ApiError::new("invalid_arguments", e.to_string()))?;
            if bytes.len() > 1_048_576 {
                return Err(ApiError::new("invalid_arguments", "Input exceeds 1 MiB."));
            }
            serde_json::from_slice(&bytes)
                .map_err(|e| ApiError::new("invalid_arguments", e.to_string()))?
        }
        _ => {
            return Err(ApiError::new(
                "invalid_arguments",
                "Usage: FerMatter cli TOOL [--input FILE|-]",
            ))
        }
    };
    super::transport::call(
        identity,
        &args[0],
        arguments,
        Arc::new(AtomicBool::new(false)),
        &mut |v| eprintln!("{}", json!({"progress":v})),
    )
}
