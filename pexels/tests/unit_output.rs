use pexels::cli::shape_output;
use pexels::output::{parse_page_number, wrap_ok};
use pexels::proj::project_item_with_fallback;
use serde_json::json;

#[test]
fn test_parse_page_number() {
    assert_eq!(parse_page_number("https://x/y?page=2&per_page=5"), Some(2));
    assert_eq!(parse_page_number("/v1/search?page=10"), Some(10));
    assert_eq!(parse_page_number("/v1/search?foo=bar"), None);
}

#[test]
fn test_shape_output_wraps_list_and_meta() {
    let resp = json!({
        "page": 1,
        "per_page": 2,
        "next_page": "https://api.pexels.com/v1/search?page=2&per_page=2&query=cats",
        "photos": [
            {"id": 1, "photographer": "A"},
            {"id": 2, "photographer": "B"}
        ]
    });
    let (data, meta) = shape_output(&resp);
    assert!(data.is_array());
    let out = wrap_ok(&data, Some(meta));
    assert!(out.get("data").is_some());
    assert!(out.get("meta").is_some());
    assert!(out["meta"].get("page").is_none());
    assert!(out["meta"].get("per_page").is_none());
    assert!(out["meta"]["next_page"].is_u64() || out["meta"]["next_page"].is_null());
    // Ensure page/per_page not present anywhere but original
    assert!(out.get("page").is_none());
    assert!(out.get("per_page").is_none());
}

#[test]
fn test_single_resource_projection_fallback() {
    let item = json!({
        "id": 123,
        "url": "u",
        "width": 100,
        "height": 200,
        "src": {"original": "heavy"}
    });
    // Project a field that doesn't exist to force empty projection
    let projected = project_item_with_fallback(&item, &["nonexistent".into()]);
    assert!(projected.is_object());
    // ensure we didn't end up with {}
    assert!(!projected.as_object().unwrap().is_empty());
}
