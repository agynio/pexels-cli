use anyhow::Result;
use serde::Serialize;
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
    let mut map = serde_json::Map::new();
    map.insert("error".into(), JsonValue::String(err.to_string()));
    let s = serde_yaml::to_string(&JsonValue::Object(map))?;
    let _ = writeln!(io::stderr(), "{}", s.trim_end());
    Ok(())
}
