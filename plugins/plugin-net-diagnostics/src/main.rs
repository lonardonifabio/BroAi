use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{io::{self, Read}, net::{TcpStream, ToSocketAddrs}, process::Command, time::{Duration, Instant}};

#[derive(Debug, Deserialize)] struct PluginRequest { action: String, payload: Value }
#[derive(Debug, Serialize)] struct PluginResponse { success: bool, result: Value, error: Option<String> }

fn main(){let mut i=String::new();io::stdin().read_to_string(&mut i).unwrap_or(0);let r=serde_json::from_str::<PluginRequest>(&i).unwrap_or(PluginRequest{action:"ping".into(),payload:json!({})});println!("{}",serde_json::to_string(&handle(r)).unwrap());}

fn handle(req: PluginRequest)->PluginResponse{
 let cmd=req.payload.get("command").and_then(|v|v.as_str()).unwrap_or(req.action.as_str());
 let args=req.payload.get("args").and_then(|v|v.as_str()).unwrap_or("");
 let target=if args.is_empty(){"8.8.8.8:53"}else{args};
 let result=match cmd{
  "ping"=>json!({"target":target,"reachable":tcp(target).is_ok()}),
  "latency"=>{let s=samples(target,3);let avg=if s.is_empty(){None}else{Some(s.iter().sum::<u128>() as f64/s.len() as f64)};json!({"target":target,"samples_ms":s,"avg_ms":avg})},
  "dns"=>{let host=if args.is_empty(){"openai.com"}else{args};json!({"host":host,"nslookup":run("nslookup", &[host]),"resolved_by_socket":host.to_socket_addrs().is_ok()})},
  _=>return PluginResponse{success:false,result:Value::Null,error:Some("Unknown command. Use: ping, dns, latency".into())}
 };
 PluginResponse{success:true,result,error:None}
}
fn tcp(t:&str)->Result<(),String>{let a=t.to_socket_addrs().map_err(|e|e.to_string())?.next().ok_or("no addr")?;TcpStream::connect_timeout(&a,Duration::from_secs(2)).map(|_|()).map_err(|e|e.to_string())}
fn samples(t:&str,n:usize)->Vec<u128>{(0..n).filter_map(|_|{let a=t.to_socket_addrs().ok()?.next()?;let st=Instant::now();TcpStream::connect_timeout(&a,Duration::from_secs(2)).ok()?;Some(st.elapsed().as_millis())}).collect()}
fn run(cmd:&str,args:&[&str])->String{Command::new(cmd).args(args).output().map(|o|String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_else(|_|"not available".into())}
