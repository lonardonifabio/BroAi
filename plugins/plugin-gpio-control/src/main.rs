use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{io::{self, Read}, path::Path, process::Command};

#[derive(Debug, Deserialize)]
struct PluginRequest { action: String, payload: Value }
#[derive(Debug, Serialize)]
struct PluginResponse { success: bool, result: Value, error: Option<String> }

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or(0);
    let req = serde_json::from_str::<PluginRequest>(&input).unwrap_or(PluginRequest{action:"gpio".into(),payload:json!({})});
    println!("{}", serde_json::to_string(&handle(req)).unwrap());
}

fn handle(req: PluginRequest) -> PluginResponse {
    let args = req.payload.get("args").and_then(|v| v.as_str()).unwrap_or("");
    let parts: Vec<&str> = args.split_whitespace().collect();
    let sub = parts.first().copied().unwrap_or("read");
    let pin = parts.get(1).and_then(|s| s.parse::<u8>().ok()).unwrap_or(17);

    if Path::new("/usr/bin/raspi-gpio").exists() {
        let cmd = match sub {
            "on" => format!("set {} op dh", pin),
            "off" => format!("set {} op dl", pin),
            _ => format!("get {}", pin),
        };
        let out = Command::new("raspi-gpio").args(cmd.split_whitespace()).output();
        return match out {
            Ok(o) => ok(json!({"pin": pin, "action": sub, "output": String::from_utf8_lossy(&o.stdout)})),
            Err(e) => err(&format!("GPIO command failed: {e}")),
        };
    }

    ok(json!({"pin": pin, "action": sub, "note": "raspi-gpio not available in this environment (dry-run mode)"}))
}

fn ok(v: Value) -> PluginResponse { PluginResponse{success:true,result:v,error:None} }
fn err(msg: &str) -> PluginResponse { PluginResponse{success:false,result:Value::Null,error:Some(msg.to_string())} }
