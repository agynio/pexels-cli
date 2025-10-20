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

// Wrap successful payload into the standard envelope
// { data: <payload>, meta: { ... } }
pub fn wrap_ok(data: &JsonValue, meta: Option<JsonValue>) -> JsonValue {
    let mut meta_obj = match meta {
        Some(JsonValue::Object(m)) => JsonValue::Object(m),
        Some(v) => v,
        None => JsonValue::Object(serde_json::Map::new()),
    };
    let mut root = serde_json::Map::new();
    root.insert("data".into(), data.clone());
    if !meta_obj.is_object() {
        meta_obj = JsonValue::Object(serde_json::Map::new());
    }
    root.insert("meta".into(), meta_obj);
    JsonValue::Object(root)
}

// Extract the `page` query param from a URL string.
pub fn parse_page_number(url: &str) -> Option<u32> {
    // Accept absolute or relative URLs
    let u = if url.starts_with("http://") || url.starts_with("https://") {
        url::Url::parse(url).ok()?
    } else {
        let base = url::Url::parse("https://example.local/").ok()?;
        base.join(url).ok()?
    };
    let mut page: Option<u32> = None;
    for (k, v) in u.query_pairs() {
        if k == "page" {
            if let Ok(p) = v.parse::<u32>() {
                page = Some(p);
                break;
            }
        }
    }
    page
}
