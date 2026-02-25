use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{io::{self, Read}, process::Command, time::{SystemTime, UNIX_EPOCH}};

#[derive(Debug, Deserialize)] struct PluginRequest { action: String, payload: Value }
#[derive(Debug, Serialize)] struct PluginResponse { success: bool, result: Value, error: Option<String> }

fn main(){let mut i=String::new();io::stdin().read_to_string(&mut i).unwrap_or(0);let r=serde_json::from_str::<PluginRequest>(&i).unwrap_or(PluginRequest{action:"make-xlsx".into(),payload:json!({})});println!("{}",serde_json::to_string(&handle(r)).unwrap());}

fn handle(req: PluginRequest)->PluginResponse{
 let cmd=req.payload.get("command").and_then(|v|v.as_str()).unwrap_or(req.action.as_str());
 match cmd {
  "sheet-template" => ok(json!({"templates":["inventory","timesheet","report"],"usage":"/make-xlsx <titolo>"})),
  "make-xlsx" => {
    let title=req.payload.get("args").and_then(|v|v.as_str()).unwrap_or("Generated Sheet");
    let name=format!("generated_{}.xlsx", now());
    let py=format!(r#"import zipfile
name={name:?}
title={title:?}
ct='''<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>'''
rels='''<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>'''
wb='''<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets></workbook>'''
wb_rels='''<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>'''
sh=f'''<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>{{title}}</t></is></c></row><row r="2"><c r="A2" t="inlineStr"><is><t>Value A</t></is></c><c r="B2"><v>10</v></c></row><row r="3"><c r="A3" t="inlineStr"><is><t>Value B</t></is></c><c r="B3"><v>20</v></c></row></sheetData></worksheet>'''
with zipfile.ZipFile(name,'w') as z:
 z.writestr('[Content_Types].xml',ct);z.writestr('_rels/.rels',rels);z.writestr('xl/workbook.xml',wb);z.writestr('xl/_rels/workbook.xml.rels',wb_rels);z.writestr('xl/worksheets/sheet1.xml',sh)
"#);
    let ok_run=Command::new("python3").args(["-c", &py]).status().map(|s|s.success()).unwrap_or(false);
    if ok_run { ok(json!({"file":name})) } else { err("Failed generating XLSX (python3 unavailable?)") }
  }
  _ => err("Unknown command. Use: make-xlsx, sheet-template")
 }
}
fn now()->u64{SystemTime::now().duration_since(UNIX_EPOCH).map(|d|d.as_secs()).unwrap_or(0)}
fn ok(v:Value)->PluginResponse{PluginResponse{success:true,result:v,error:None}} fn err(m:&str)->PluginResponse{PluginResponse{success:false,result:Value::Null,error:Some(m.into())}}
