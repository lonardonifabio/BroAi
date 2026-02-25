use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{io::{self, Read}, path::Path};

const DB_PATH: &str = "./scheduler.db";

#[derive(Debug, Deserialize)]
struct PluginRequest { action: String, payload: Value }
#[derive(Debug, Serialize)]
struct PluginResponse { success: bool, result: Value, error: Option<String> }

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or(0);
    let req = serde_json::from_str::<PluginRequest>(&input).unwrap_or(PluginRequest{action:"remind".into(),payload:json!({})});
    println!("{}", serde_json::to_string(&handle(req)).unwrap());
}

fn handle(req: PluginRequest) -> PluginResponse {
    let conn = match init_db() {
        Ok(c) => c,
        Err(e) => return PluginResponse{success:false,result:Value::Null,error:Some(e)},
    };
    let command = req.payload.get("command").and_then(|v| v.as_str()).unwrap_or(req.action.as_str());
    match command {
        "remind" => {
            let text = req.payload.get("args").and_then(|v| v.as_str()).unwrap_or("").trim();
            if text.is_empty() { return err("Usage: /remind <text>"); }
            let now = Utc::now().to_rfc3339();
            if let Err(e) = conn.execute("INSERT INTO jobs(task, created_at, done) VALUES (?1, ?2, 0)", params![text, now]) {
                return err(&format!("Insert failed: {e}"));
            }
            ok(json!({"message":"Reminder saved","task":text}))
        }
        "jobs" => {
            let mut stmt = match conn.prepare("SELECT id, task, created_at, done FROM jobs ORDER BY id DESC LIMIT 50") {
                Ok(s) => s,
                Err(e) => return err(&format!("Query prep failed: {e}")),
            };
            let rows = stmt.query_map([], |r| Ok(json!({"id": r.get::<_, i64>(0)?, "task": r.get::<_, String>(1)?, "created_at": r.get::<_, String>(2)?, "done": r.get::<_, i64>(3)? == 1 })))
                .and_then(|mapped| mapped.collect::<Result<Vec<_>, _>>());
            match rows { Ok(jobs) => ok(json!({"jobs": jobs})), Err(e) => err(&format!("Query failed: {e}")) }
        }
        _ => err("Unknown command. Use: remind, jobs"),
    }
}

fn init_db() -> Result<Connection, String> {
    let path = Path::new(DB_PATH);
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.execute("CREATE TABLE IF NOT EXISTS jobs(id INTEGER PRIMARY KEY AUTOINCREMENT, task TEXT NOT NULL, created_at TEXT NOT NULL, done INTEGER NOT NULL DEFAULT 0)", [])
        .map_err(|e| e.to_string())?;
    Ok(conn)
}
fn ok(v: Value) -> PluginResponse { PluginResponse{success:true,result:v,error:None} }
fn err(msg: &str) -> PluginResponse { PluginResponse{success:false,result:Value::Null,error:Some(msg.to_string())} }
