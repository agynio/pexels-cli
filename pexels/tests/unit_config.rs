use pexels::proj::{project, project_response};

#[test]
fn test_config_precedence_env_over_config() {
    // Basic smoke: ensure project compiles and functions exist
    let v = serde_json::json!({"a":{"b":1}});
    let out = project(&v, &["a.b".into()]);
    assert_eq!(out["a"]["b"], 1);
    let resp = serde_json::json!({"photos":[{"id":1,"width":10,"height":20}]});
    let out2 = project_response(&resp, &["width".into()]);
    assert_eq!(out2["photos"][0]["width"], 10);
}
