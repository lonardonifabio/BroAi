use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, io::{self, Read}, process::Command};

#[derive(Debug, Deserialize)]
struct PluginRequest { action: String, payload: Value }
#[derive(Debug, Serialize)]
struct PluginResponse { success: bool, result: Value, error: Option<String> }

fn main() {
    let mut input = String::new(); io::stdin().read_to_string(&mut input).unwrap_or(0);
    let req = serde_json::from_str::<PluginRequest>(&input).unwrap_or(PluginRequest{action:"sysinfo".into(),payload:json!({})});
    println!("{}", serde_json::to_string(&handle(req)).unwrap());
}

fn handle(req: PluginRequest) -> PluginResponse {
    let command = req.payload.get("command").and_then(|v| v.as_str()).unwrap_or(req.action.as_str());
    let mem = read_meminfo();
    let disks = Command::new("bash").args(["-lc", "df -B1 --output=target,size,avail | tail -n +2"]).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
    let result = match command {
        "uptime" => json!({"uptime": read_uptime()}),
        "disk" => json!({"disk": disks}),
        _ => json!({"cpu_temp_c": read_cpu_temp(), "ram_free_kb": mem.0, "ram_total_kb": mem.1, "uptime": read_uptime(), "disk": disks}),
    };
    PluginResponse{success:true,result,error:None}
}

fn read_cpu_temp() -> Option<f64> { fs::read_to_string("/sys/class/thermal/thermal_zone0/temp").ok().and_then(|s| s.trim().parse::<f64>().ok()).map(|v| v/1000.0) }
fn read_uptime() -> String { fs::read_to_string("/proc/uptime").ok().and_then(|s| s.split_whitespace().next().map(|x| x.to_string())).unwrap_or_else(|| "unknown".into()) }
fn read_meminfo() -> (u64,u64) {
    let txt = fs::read_to_string("/proc/meminfo").unwrap_or_default(); let mut total=0; let mut avail=0;
    for l in txt.lines() { if l.starts_with("MemTotal:") { total=l.split_whitespace().nth(1).and_then(|x| x.parse().ok()).unwrap_or(0); }
        if l.starts_with("MemAvailable:") { avail=l.split_whitespace().nth(1).and_then(|x| x.parse().ok()).unwrap_or(0); }} (avail,total)
}
