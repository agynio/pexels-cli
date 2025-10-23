use pexels::cli::build_auth_status;
use pexels::config::{Config, TokenSource};
use pexels::output::wrap_ok;
use serde_json::json;

#[test]
fn auth_status_details_no_meta_and_schema() {
    let mut cfg = Config::default();
    // simulate config-sourced token present
    cfg.token = Some("t".into());
    cfg.token_source = Some(TokenSource::Config);

    // Build payload like handler and ensure no meta when wrapped with None
    let payload = serde_json::json!({
        "present": true,
        "source": "config",
        "details": { "path": cfg.path().display().to_string(), "profile": null }
    });
    let out = wrap_ok(&payload, None);
    assert!(out.get("data").is_some());
    assert!(out.get("meta").is_none());
}

#[test]
fn login_success_payloads() {
    // positional token path
    let out = wrap_ok(&json!({"status":"ok","message":"token saved"}), None);
    assert!(out.get("meta").is_none());
    // env path example
    let out2 = wrap_ok(
        &json!({"status":"ok","message":"token saved from env PEXELS_TOKEN"}),
        None,
    );
    assert!(out2.get("meta").is_none());
}

#[test]
fn auth_status_env_details() {
    // Set env and apply
    std::env::set_var("PEXELS_TOKEN", "envtoken");
    let mut cfg = Config::default();
    cfg.apply_env();
    let payload = build_auth_status(&cfg);
    assert_eq!(payload["source"], "env");
    assert_eq!(payload["details"]["var"], "PEXELS_TOKEN");
    assert_eq!(payload["details"]["set"], true);
    // cleanup
    std::env::remove_var("PEXELS_TOKEN");
}

#[test]
fn auth_status_none_reason() {
    let cfg = Config::default();
    let payload = build_auth_status(&cfg);
    assert_eq!(payload["source"], "none");
    assert_eq!(payload["present"], false);
    assert_eq!(payload["details"]["reason"], "no token found");
}
