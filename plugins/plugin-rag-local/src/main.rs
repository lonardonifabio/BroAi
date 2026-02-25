use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, fs, io::{self, Read}, path::{Path, PathBuf}};

const KB_DIR: &str = "./docs";

#[derive(Debug, Deserialize)]
struct PluginRequest { action: String, payload: Value }
#[derive(Debug, Serialize)]
struct PluginResponse { success: bool, result: Value, error: Option<String> }

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or(0);
    let req = serde_json::from_str::<PluginRequest>(&input).unwrap_or(PluginRequest { action: "kb".into(), payload: json!({}) });
    println!("{}", serde_json::to_string(&handle(req)).unwrap());
}

fn handle(req: PluginRequest) -> PluginResponse {
    let cmd = req.payload.get("command").and_then(|v| v.as_str()).unwrap_or(req.action.as_str());
    match cmd {
        "kb" => {
            let found = list_files(Path::new(KB_DIR)).into_iter().map(|p| p.display().to_string()).collect::<Vec<_>>();
            ok(json!({"kb_path": KB_DIR, "documents": found.len(), "files": found}))
        }
        "search-doc" => {
            let q = req.payload.get("args").and_then(|v| v.as_str()).unwrap_or("").trim();
            if q.is_empty() { return err("Usage: /search-doc <query>"); }
            let qv = embed(q);
            let mut scored = vec![];
            for path in list_files(Path::new(KB_DIR)) {
                if let Ok(content) = fs::read_to_string(&path) {
                    scored.push(json!({
                        "path": path.display().to_string(),
                        "score": cosine(&qv, &embed(&content)),
                        "snippet": content.chars().take(220).collect::<String>()
                    }));
                }
            }
            scored.sort_by(|a, b| b["score"].as_f64().partial_cmp(&a["score"].as_f64()).unwrap());
            ok(json!({"query": q, "results": scored.into_iter().take(5).collect::<Vec<_>>() }))
        }
        _ => err("Unknown command. Use: kb, search-doc"),
    }
}

fn list_files(root: &Path) -> Vec<PathBuf> {
    let mut out = vec![];
    if !root.exists() { return out; }
    if let Ok(rd) = fs::read_dir(root) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                out.extend(list_files(&p));
            } else if let Some(ext) = p.extension().and_then(|x| x.to_str()) {
                if ["txt", "md", "log", "rst"].contains(&ext) { out.push(p); }
            }
        }
    }
    out
}

fn embed(text: &str) -> HashMap<String, f64> {
    let mut map = HashMap::new();
    for tok in text.to_lowercase().split(|c: char| !c.is_alphanumeric()) {
        if tok.len() >= 3 { *map.entry(tok.to_string()).or_insert(0.0) += 1.0; }
    }
    map
}

fn cosine(a: &HashMap<String, f64>, b: &HashMap<String, f64>) -> f64 {
    let dot: f64 = a.iter().map(|(k, v)| *v * b.get(k).unwrap_or(&0.0)).sum();
    let na: f64 = a.values().map(|v| v * v).sum::<f64>().sqrt();
    let nb: f64 = b.values().map(|v| v * v).sum::<f64>().sqrt();
    if na == 0.0 || nb == 0.0 { 0.0 } else { dot / (na * nb) }
}

fn ok(v: Value) -> PluginResponse { PluginResponse { success: true, result: v, error: None } }
fn err(msg: &str) -> PluginResponse { PluginResponse { success: false, result: Value::Null, error: Some(msg.into()) } }
