use std::io::{self,Write};
struct Counter(usize);
impl Write for Counter { fn write(&mut self,b:&[u8])->io::Result<usize>{self.0=self.0.checked_add(b.len()).ok_or_else(||io::Error::other("counter overflow"))?;Ok(b.len())} fn flush(&mut self)->io::Result<()>{Ok(())}}
fn main()->Result<(),Box<dyn std::error::Error>>{let path=std::env::args().nth(1).ok_or("input path required")?;let bytes=std::fs::read(path)?;if bytes.len()>134217728{return Err("input witness cap".into())}let value:serde_json::Value=serde_json::from_slice(&bytes)?;let mut count=Counter(0);serde_json::to_writer(&mut count,&value)?;println!("{}",count.0);Ok(())}
