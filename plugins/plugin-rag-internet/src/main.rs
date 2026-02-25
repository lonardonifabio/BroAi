use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, Read};

#[derive(Debug, Deserialize)]
struct PluginRequest { action: String, payload: Value }
#[derive(Debug, Serialize)]
struct PluginResponse { success: bool, result: Value, error: Option<String> }

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or(0);
    let req = serde_json::from_str::<PluginRequest>(&input).unwrap_or(PluginRequest{action:"web-search".into(),payload:json!({})});
    println!("{}", serde_json::to_string(&handle(req)).unwrap());
}

fn handle(req: PluginRequest) -> PluginResponse {
    let command = req.payload.get("command").and_then(|v| v.as_str()).unwrap_or(req.action.as_str());
    let q = req.payload.get("args").and_then(|v| v.as_str()).unwrap_or("").trim();
    if q.is_empty() { return err("Usage: /web-search <query> or /web-rag <query>"); }

    match search(q) {
        Ok(results) => {
            if command == "web-rag" {
                let synthesis = results.iter().take(3).map(|r| format!("- {} ({})", r["title"].as_str().unwrap_or(""), r["url"].as_str().unwrap_or(""))).collect::<Vec<_>>().join("\n");
                ok(json!({"query": q, "summary": format!("Top web evidence for '{q}':\n{synthesis}"), "sources": results}))
            } else {
                ok(json!({"query": q, "results": results}))
            }
        }
        Err(e) => err(&e),
    }
}

fn search(q: &str) -> Result<Vec<Value>, String> {
    let encoded = q.replace(" ", "+");
    let url = format!("https://api.duckduckgo.com/?q={}&format=json&no_redirect=1&no_html=1", encoded);
    let resp: Value = ureq::get(&url).call().map_err(|e| e.to_string())?.into_json().map_err(|e| e.to_string())?;
    let mut out = vec![];
    if let Some(arr) = resp["RelatedTopics"].as_array() {
        for item in arr.iter().take(8) {
            if item.get("Text").is_some() {
                out.push(json!({"title": item["Text"], "url": item["FirstURL"], "snippet": item["Text"]}));
            } else if let Some(topics) = item.get("Topics").and_then(|t| t.as_array()) {
                for t in topics.iter().take(3) {
                    out.push(json!({"title": t["Text"], "url": t["FirstURL"], "snippet": t["Text"]}));
                }
            }
        }
    }
    Ok(out)
}

fn ok(v: Value) -> PluginResponse { PluginResponse{success:true,result:v,error:None} }
fn err(msg: &str) -> PluginResponse { PluginResponse{success:false,result:Value::Null,error:Some(msg.to_string())} }
