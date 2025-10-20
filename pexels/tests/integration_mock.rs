use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use wiremock::matchers::{method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

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
    cmd.arg("--host").arg(server.uri())
        .arg("photos").arg("search").arg("cat");
    cmd.assert()
        .success()
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
    Mock::given(method("GET")).and(path("/v1/search")).and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page2)).mount(&server).await;
    Mock::given(method("GET")).and(path("/v1/search")).and(query_param("query", "dog"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page1)).mount(&server).await;

    let mut cmd = Command::cargo_bin("pexels").unwrap();
    cmd.arg("--host").arg(server.uri())
        .arg("--all").arg("--limit").arg("3")
        .arg("photos").arg("search").arg("dog")
        .arg("--json");
    let out = cmd.assert().success().get_output().stdout.clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["photos"].as_array().unwrap().len(), 3);
}
