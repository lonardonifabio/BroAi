use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{io::{self, Read}, process::Command};

#[derive(Debug, Deserialize)]
struct PluginRequest { action: String, payload: Value }
#[derive(Debug, Serialize)]
struct PluginResponse { success: bool, result: Value, error: Option<String> }

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or(0);
    let req = serde_json::from_str::<PluginRequest>(&input).unwrap_or(PluginRequest{action:"update-check".into(), payload: json!({})});
    println!("{}", serde_json::to_string(&handle(req)).unwrap());
}

fn handle(req: PluginRequest) -> PluginResponse {
    let command = req.payload.get("command").and_then(|v| v.as_str()).unwrap_or(req.action.as_str());
    match command {
        "update-check" => ok(json!({
            "safe_mode": true,
            "apt_simulation": run("bash", &["-lc", "apt list --upgradable 2>/dev/null | head -n 25"]),
            "cargo_outdated_hint": "Run `cargo install cargo-outdated && cargo outdated` for Rust crates"
        })),
        "update-plan" => ok(json!({
            "safe_mode": true,
            "plan": [
                "1) Backup config/data and snapshot current version",
                "2) Validate connectivity and disk free space",
                "3) Dry-run package updates and capture changelog",
                "4) Schedule maintenance window",
                "5) Apply updates manually outside this plugin"
            ]
        })),
        _ => err("Unknown command. Use: update-check, update-plan"),
    }
}

fn run(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd).args(args).output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_else(|_| "not available".into())
}
fn ok(v: Value) -> PluginResponse { PluginResponse{success:true,result:v,error:None} }
fn err(msg: &str) -> PluginResponse { PluginResponse{success:false,result:Value::Null,error:Some(msg.to_string())} }
