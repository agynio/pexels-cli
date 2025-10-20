use serde_json::{Map, Value};

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

// project_response removed; envelope now constructed in CLI.

// Ensure projected item is not an empty object; if empty, fallback to minimal descriptive fields
fn ensure_non_empty_item(projected: &Value, original: &Value) -> Value {
    if matches!(projected, Value::Object(map) if map.is_empty()) {
        minimal_item(original)
    } else {
        projected.clone()
    }
}

fn minimal_item(original: &Value) -> Value {
    match original {
        Value::Object(map) => {
            let mut o = Map::new();
            for k in [
                "id",
                "url",
                "photographer",
                "alt",
                "title",
                "description",
                "duration",
            ] {
                if let Some(v) = map.get(k) {
                    o.insert(k.to_string(), v.clone());
                }
            }
            if o.is_empty() {
                // Fallback: include first scalar field if any
                for (k, v) in map.iter() {
                    if matches!(v, Value::String(_) | Value::Number(_) | Value::Bool(_)) {
                        o.insert(k.clone(), v.clone());
                        break;
                    }
                }
            }
            Value::Object(o)
        }
        _ => original.clone(),
    }
}

// Public helpers for projecting items with fallback
pub fn project_item_with_fallback(item: &Value, fields: &[String]) -> Value {
    let p = project(item, fields);
    ensure_non_empty_item(&p, item)
}

pub fn project_items_with_fallback(items: &[Value], fields: &[String]) -> Vec<Value> {
    items
        .iter()
        .map(|it| project_item_with_fallback(it, fields))
        .collect()
}

fn merge(dst: &mut Value, src: &Value) {
    match (dst.clone(), src) {
        (_, Value::Null) => {}
        (Value::Null, _) => *dst = src.clone(),
        (Value::Object(a), Value::Object(b)) => {
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
    use serde_json::json;

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
    fn test_project_array_items_direct() {
        let items = json!([{"id":1,"width":100,"height":200,"src":{"original":"u"}}]);
        let proj: Vec<Value> = items
            .as_array()
            .unwrap()
            .iter()
            .map(|it| project(it, &["width".into(), "height".into()]))
            .collect();
        assert_eq!(proj[0]["width"], 100);
        assert!(proj[0].get("src").is_none());
    }
}
