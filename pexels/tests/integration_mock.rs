use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};
use rand::Rng;
use std::path::PathBuf;

#[tokio::test]
async fn photos_search_defaults_yaml_and_projection() {
    let server = MockServer::start().await;
    let body = serde_json::json!({
        "page":1,
        "per_page":1,
        "photos":[{"id":123,"photographer":"Ann","alt":"a","width":10,"height":20,"avg_color":"#fff","src":{"original":"u"}}]
    });
    Mock::given(method("GET"))
        .and(path("/v1/search"))
        .and(query_param("query", "cat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let mut cmd = Command::cargo_bin("pexels").unwrap();
    cmd.arg("--host")
        .arg(server.uri())
        .arg("photos")
        .arg("search")
        .arg("cat");
    cmd.assert()
        .success()
        // Envelope keys
        .stdout(predicate::str::contains("data:"))
        .stdout(predicate::str::contains("meta:"))
        // Projected fields include id by default
        .stdout(predicate::str::contains("id:"))
        .stdout(predicate::str::contains("photographer:"))
        .stdout(predicate::str::contains("alt:"))
        .stdout(predicate::str::contains("width:"));
}

#[tokio::test]
async fn pagination_all_limit_respected() {
    let server = MockServer::start().await;
    let page1 = serde_json::json!({
        "page":1,
        "per_page":2,
        "next_page": format!("{}/v1/search?page=2&per_page=2&query=dog", server.uri()),
        "photos":[{"id":1},{"id":2}]
    });
    let page2 = serde_json::json!({
        "page":2,
        "per_page":2,
        "photos":[{"id":3},{"id":4}]
    });
    Mock::given(method("GET"))
        .and(path("/v1/search"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page2))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/search"))
        .and(query_param("query", "dog"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page1))
        .mount(&server)
        .await;

    let mut cmd = Command::cargo_bin("pexels").unwrap();
    cmd.arg("--host")
        .arg(server.uri())
        .arg("--all")
        .arg("--limit")
        .arg("3")
        .arg("photos")
        .arg("search")
        .arg("dog")
        .arg("--json");
    let out = cmd.assert().success().get_output().stdout.clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["data"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn raw_output_streams_bytes() {
    let server = MockServer::start().await;
    let body = "RAW-BYTES";
    Mock::given(method("GET"))
        .and(path("/v1/curated"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/plain"))
        .mount(&server)
        .await;

    let mut cmd = Command::cargo_bin("pexels").unwrap();
    cmd.arg("--host")
        .arg(server.uri())
        .arg("--raw")
        .arg("photos")
        .arg("curated");
    cmd.assert().success().stdout(predicate::eq(body));
}

#[tokio::test]
async fn photos_url_returns_string_under_data() {
    let server = MockServer::start().await;
    let photo_id = 999;
    let original = format!("{}/files/{}.jpg", server.uri(), photo_id);
    let body = serde_json::json!({
        "id": photo_id,
        "src": {"original": original},
        "alt": "a",
        "photographer": "Ann"
    });
    Mock::given(method("GET"))
        .and(path(format!("/v1/photos/{}", photo_id)))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let mut cmd = Command::cargo_bin("pexels").unwrap();
    cmd.arg("--host")
        .arg(server.uri())
        .arg("photos")
        .arg("url")
        .arg(photo_id.to_string())
        .arg("--json");
    let out = cmd.assert().success().get_output().stdout.clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert!(v["data"].as_str().unwrap().starts_with("http"));
}

#[tokio::test]
async fn photos_download_reports_path_and_bytes() {
    let server = MockServer::start().await;
    let photo_id = 1001;
    let img_path = "/files/test.jpg";
    let img_bytes = b"IMG-BYTES".to_vec();
    let original = format!("{}{}", server.uri(), img_path);
    let body = serde_json::json!({
        "id": photo_id,
        "src": {"original": original},
        "alt": "x",
        "photographer": "Bob"
    });
    // Photo GET
    Mock::given(method("GET"))
        .and(path(format!("/v1/photos/{}", photo_id)))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    // Bytes GET
    Mock::given(method("GET")).and(path(img_path)).respond_with(
        ResponseTemplate::new(200).set_body_raw(img_bytes.clone(), "image/jpeg"),
    )
    .mount(&server)
    .await;

    // temp path
    let mut rng = rand::thread_rng();
    let suffix: u64 = rng.gen();
    let mut path = std::env::temp_dir();
    path.push(format!("pexels-test-{}.jpg", suffix));
    // ensure parent exists
    let _ = std::fs::create_dir_all(path.parent().unwrap());

    let mut cmd = Command::cargo_bin("pexels").unwrap();
    cmd.arg("--host")
        .arg(server.uri())
        .arg("photos")
        .arg("download")
        .arg(photo_id.to_string())
        .arg(path.to_string_lossy().to_string())
        .arg("--json");
    let out = cmd.assert().success().get_output().stdout.clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let reported = v["data"]["path"].as_str().unwrap();
    assert!(std::path::Path::new(reported).exists());
    assert!(v["data"]["bytes"].as_u64().unwrap() > 0);
    // cleanup
    let _ = std::fs::remove_file(PathBuf::from(reported));
}
