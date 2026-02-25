use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{io::{self, Read}, process::Command, time::{SystemTime, UNIX_EPOCH}};

#[derive(Debug, Deserialize)] struct PluginRequest { action: String, payload: Value }
#[derive(Debug, Serialize)] struct PluginResponse { success: bool, result: Value, error: Option<String> }

fn main(){let mut i=String::new();io::stdin().read_to_string(&mut i).unwrap_or(0);let r=serde_json::from_str::<PluginRequest>(&i).unwrap_or(PluginRequest{action:"make-docx".into(),payload:json!({})});println!("{}",serde_json::to_string(&handle(r)).unwrap());}

fn handle(req: PluginRequest)->PluginResponse{
 let cmd=req.payload.get("command").and_then(|v|v.as_str()).unwrap_or(req.action.as_str());
 match cmd {
  "doc-template" => ok(json!({"templates":["report","verbale","lettera"],"usage":"/make-docx <contenuto>"})),
  "make-docx" => {
    let text=req.payload.get("args").and_then(|v|v.as_str()).unwrap_or("Documento generato da BroAi");
    let name=format!("generated_{}.docx", now());
    let py = format!(r#"import zipfile
name={name:?}
text={text:?}
xml=f'''<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>{{text}}</w:t></w:r></w:p></w:body></w:document>'''
ct='''<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>'''
rels='''<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>'''
with zipfile.ZipFile(name,'w') as z:
 z.writestr('[Content_Types].xml',ct)
 z.writestr('_rels/.rels',rels)
 z.writestr('word/document.xml',xml)
"#);
    let ok_run=Command::new("python3").args(["-c", &py]).status().map(|s|s.success()).unwrap_or(false);
    if ok_run { ok(json!({"file":name})) } else { err("Failed generating DOCX (python3 unavailable?)") }
  }
  _ => err("Unknown command. Use: make-docx, doc-template")
 }
}
fn now()->u64{SystemTime::now().duration_since(UNIX_EPOCH).map(|d|d.as_secs()).unwrap_or(0)}
fn ok(v:Value)->PluginResponse{PluginResponse{success:true,result:v,error:None}} fn err(m:&str)->PluginResponse{PluginResponse{success:false,result:Value::Null,error:Some(m.into())}}
