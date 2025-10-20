use anyhow::Result;
use serde_json::Value as JsonValue;
use std::io::{self, Write};

#[derive(Clone, Debug)]
pub enum OutputFormat {
    Yaml,
    Json,
    Raw,
}

pub fn emit_data(fmt: &OutputFormat, data: &JsonValue) -> Result<()> {
    match fmt {
        OutputFormat::Yaml => {
            let s = serde_yaml::to_string(data)?;
            println!("{}", s.trim_end());
        }
        OutputFormat::Json => {
            let s = serde_json::to_string_pretty(data)?;
            println!("{}", s);
        }
        OutputFormat::Raw => {
            if let Some(s) = data.as_str() {
                print!("{}", s);
            } else {
                print!("{}", serde_json::to_string(data)?);
            }
        }
    }
    Ok(())
}

pub fn emit_error(err: &anyhow::Error) -> Result<()> {
    // Try to parse the error string as YAML map; else wrap into structured map
    let obj = if let Ok(val) = serde_yaml::from_str::<JsonValue>(&err.to_string()) {
        val
    } else {
        let mut map = serde_json::Map::new();
        map.insert("error".into(), JsonValue::String(err.to_string()));
        JsonValue::Object(map)
    };
    let s = serde_yaml::to_string(&obj)?;
    let _ = writeln!(io::stderr(), "{}", s.trim_end());
    Ok(())
}

pub fn emit_raw_bytes(bytes: &[u8]) -> Result<()> {
    let mut out = io::stdout().lock();
    out.write_all(bytes)?;
    out.flush()?;
    Ok(())
}
