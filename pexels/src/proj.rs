use serde_json::{json, Map, Value};

// Simple projection supporting:
// - dot paths: a.b.c
// - wildcard for arrays: items[*].id
// - named sets: @ids, @urls, @files, @thumbnails, @all
pub fn project(input: &Value, fields: &[String]) -> Value {
    if fields.is_empty() || fields.iter().any(|f| f == "@all") {
        return input.clone();
    }
    let mut out = Value::Object(Map::new());
    for f in fields {
        match f.as_str() {
            "@ids" => {
                let v = extract_ids(input);
                merge(&mut out, &v);
            }
            "@urls" => {
                let v = extract_urls(input);
                merge(&mut out, &v);
            }
            "@files" => {
                let v = extract_by_keys(input, &["video_files", "src"]);
                merge(&mut out, &v);
            }
            "@thumbnails" => {
                let v = extract_by_keys(input, &["image", "thumbnail", "thumb", "tiny"]);
                merge(&mut out, &v);
            }
            path => {
                let val = select_path(input, path);
                if !val.is_null() {
                    let nested = make_nested(path, val);
                    merge(&mut out, &nested);
                }
            }
        }
    }
    out
}

// Apply projection to known item arrays while preserving top-level metadata
pub fn project_response(input: &Value, fields: &[String]) -> Value {
    if fields.is_empty() {
        return input.clone();
    }
    match input {
        Value::Object(map) => {
            let mut out = map.clone();
            for key in ["photos", "videos", "collections", "media"] {
                if let Some(Value::Array(items)) = out.get(key) {
                    let new_items = items.iter().map(|it| project(it, fields)).collect();
                    out.insert(key.to_string(), Value::Array(new_items));
                }
            }
            Value::Object(out)
        }
        _ => project(input, fields),
    }
}

fn merge(dst: &mut Value, src: &Value) {
    match (dst.clone(), src) {
        (_, Value::Null) => {}
        (Value::Null, _) => *dst = src.clone(),
        (Value::Object(mut a), Value::Object(b)) => {
            let mut merged = a;
            for (k, v) in b {
                merged.insert(k.clone(), v.clone());
            }
            *dst = Value::Object(merged);
        }
        _ => {}
    }
}

fn select_path(input: &Value, path: &str) -> Value {
    let parts: Vec<&str> = path.split('.').collect();
    select_inner(input, &parts)
}

fn select_inner(input: &Value, parts: &[&str]) -> Value {
    if parts.is_empty() {
        return input.clone();
    }
    match input {
        Value::Object(map) => {
            if let Some((head, tail)) = parts.split_first() {
                if *head == "*" {
                    return Value::Null;
                }
                if head.ends_with("[*]") {
                    let base = head.trim_end_matches("[*]");
                    if let Some(Value::Array(arr)) = map.get(base) {
                        let sub = arr
                            .iter()
                            .map(|v| select_inner(v, tail))
                            .collect::<Vec<_>>();
                        return Value::Array(sub);
                    } else {
                        return Value::Null;
                    }
                }
                if let Some(v) = map.get(*head) {
                    select_inner(v, tail)
                } else {
                    Value::Null
                }
            } else {
                Value::Null
            }
        }
        Value::Array(arr) => {
            if let Some((head, tail)) = parts.split_first() {
                if *head == "[*]" || head.ends_with("[*]") {
                    let sub = arr
                        .iter()
                        .map(|v| select_inner(v, tail))
                        .collect::<Vec<_>>();
                    Value::Array(sub)
                } else {
                    // index unsupported -> null
                    Value::Null
                }
            } else {
                input.clone()
            }
        }
        _ => Value::Null,
    }
}

fn insert_path(dst: &mut Map<String, Value>, path: &str, value: Value) {
    let mut parts = path.split('.').peekable();
    let mut cur: *mut Map<String, Value> = dst as *mut _;
    while let Some(p) = parts.next() {
        let is_last = parts.peek().is_none();
        if is_last {
            unsafe { (&mut *cur).insert(p.to_string(), value.clone()); }
        } else {
            // Use raw pointer to avoid borrow checker issues in nested inserts
            unsafe {
                let map = &mut *cur;
                let entry = map.entry(p.to_string()).or_insert_with(|| Value::Object(Map::new()));
                if !entry.is_object() {
                    *entry = Value::Object(Map::new());
                }
                if let Value::Object(ref mut m) = entry {
                    cur = m as *mut _;
                }
            }
        }
    }
}

fn make_nested(path: &str, value: Value) -> Value {
    let mut cur = value;
    for part in path.split('.').rev() {
        let mut m = Map::new();
        m.insert(part.to_string(), cur);
        cur = Value::Object(m);
    }
    cur
}

fn extract_ids(input: &Value) -> Value {
    // Heuristic: collect fields named id, ids
    match input {
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                if k == "id" || k.ends_with("_id") || k == "ids" {
                    out.insert(k.clone(), v.clone());
                }
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(extract_ids).collect()),
        _ => Value::Null,
    }
}

fn extract_urls(input: &Value) -> Value {
    match input {
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                if k.contains("url") || k.contains("link") || k.contains("href") {
                    out.insert(k.clone(), v.clone());
                }
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(extract_urls).collect()),
        _ => Value::Null,
    }
}

fn extract_by_keys(input: &Value, keys: &[&str]) -> Value {
    match input {
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                if keys.contains(&k.as_str()) {
                    out.insert(k.clone(), v.clone());
                }
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(|v| extract_by_keys(v, keys)).collect()),
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_dot() {
        let v = json!({"a":{"b":{"c":1}}, "id": 5, "url":"x"});
        let out = project(&v, &["a.b.c".into()]);
        assert_eq!(out["a"]["b"]["c"], 1);
    }

    #[test]
    fn test_sets() {
        let v = json!({"id":1,"url":"u","video_files":[{"link":"x"}]});
        let out = project(&v, &["@ids".into(), "@urls".into(), "@files".into()]);
        assert!(out.is_object());
    }

    #[test]
    fn test_project_response_array_items() {
        let v = json!({"page":1, "photos":[{"id":1,"width":100,"height":200,"src":{"original":"u"}}]});
        let out = project_response(&v, &["width".into(), "height".into()]);
        assert_eq!(out["photos"][0]["width"], 100);
        assert!(out["photos"][0].get("src").is_none());
        assert_eq!(out["page"], 1);
    }
}
