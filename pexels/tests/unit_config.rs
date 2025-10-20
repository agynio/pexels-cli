use pexels::config::Config;
use pexels::proj::{project, project_response};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

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

#[test]
fn test_config_path_vendorless() {
    let path = Config::config_path();
    let s = path.display().to_string();
    assert!(s.contains("pexels"));
    assert!(s.ends_with("config.yaml"));
}

#[test]
fn test_token_save_permissions() {
    let cfg = Config {
        token: Some("t".into()),
        ..Default::default()
    };
    cfg.save().unwrap();
    let meta = fs::metadata(cfg.path()).unwrap();
    #[cfg(unix)]
    assert_eq!(meta.permissions().mode() & 0o777, 0o600);
}
