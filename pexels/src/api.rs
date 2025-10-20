use crate::config::Config;
use crate::output::OutputFormat;
use crate::util::backoff_delay;
use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, AUTHORIZATION, USER_AGENT};
use reqwest::{Client, Response, StatusCode, Url};
use serde_json::Value as JsonValue;
use tokio::io::AsyncReadExt;
use std::time::Duration;
use tracing::{debug, info, warn};

#[derive(Clone)]
pub struct PexelsClient {
    cfg: Config,
    http: Client,
}

impl PexelsClient {
    pub fn new(cfg: Config) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        if let Some(locale) = &cfg.locale {
            headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_str(locale).unwrap_or(HeaderValue::from_static("en")));
        }

        if let Some(token) = &cfg.token {
            // Pexels uses token directly, not Bearer
            headers.insert(AUTHORIZATION, HeaderValue::from_str(token).unwrap_or(HeaderValue::from_static("")));
        }

        let ua = format!(
            "pexels-cli/{} (+https://github.com/agynio/pexels-cli) {}",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS
        );
        headers.insert(USER_AGENT, HeaderValue::from_str(&ua).unwrap());

        let http = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(cfg.timeout_secs))
            .build()?;
        Ok(Self { cfg, http })
    }

    pub fn base_photos(&self) -> Url {
        let base = self
            .cfg
            .host
            .clone()
            .unwrap_or_else(|| "https://api.pexels.com".to_string());
        Url::parse(&(base + "/v1/")).expect("valid url")
    }
    pub fn base_videos(&self) -> Url {
        let base = self
            .cfg
            .host
            .clone()
            .unwrap_or_else(|| "https://api.pexels.com".to_string());
        Url::parse(&(base + "/videos/")).expect("valid url")
    }

    async fn req(&self, url: Url, qp: Vec<(String, String)>) -> Result<JsonValue> {
        // retries with backoff
        let mut attempt = 0;
        loop {
            let res = self.http.get(url.clone()).query(&qp).send().await;
            match res {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        return parse_json(resp).await;
                    }
                    if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                        if attempt < self.cfg.max_retries {
                            attempt += 1;
                            let delay = retry_after_delay(&resp, attempt, self.cfg.retry_after);
                            warn!("http {} retrying in {:?}", status, delay);
                            tokio::time::sleep(delay).await;
                            continue;
                        }
                    }
                    return Err(http_error(resp).await);
                }
                Err(e) => {
                    if attempt < self.cfg.max_retries {
                        attempt += 1;
                        let delay = backoff_delay(attempt);
                        warn!("http error: {} retrying in {:?}", redact(&e.to_string()), delay);
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(anyhow::anyhow!(e));
                }
            }
        }
    }

    pub async fn req_bytes(&self, url: Url, qp: Vec<(String, String)>) -> Result<Vec<u8>> {
        let mut attempt = 0;
        loop {
            let res = self.http.get(url.clone()).query(&qp).send().await;
            match res {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        return Ok(resp.bytes().await?.to_vec());
                    }
                    if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                        if attempt < self.cfg.max_retries {
                            attempt += 1;
                            let delay = retry_after_delay(&resp, attempt, self.cfg.retry_after);
                            warn!("http {} retrying in {:?}", status, delay);
                            tokio::time::sleep(delay).await;
                            continue;
                        }
                    }
                    return Err(http_error(resp).await);
                }
                Err(e) => {
                    if attempt < self.cfg.max_retries {
                        attempt += 1;
                        let delay = backoff_delay(attempt);
                        warn!("http error: {} retrying in {:?}", redact(&e.to_string()), delay);
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(anyhow::anyhow!(e));
                }
            }
        }
    }

    pub async fn quota_view(&self) -> Result<JsonValue> {
        // Pexels exposes remaining via headers; for CLI, attempt a ping endpoint and echo headers
        let url = self.base_photos().join("curated").map_err(|e| anyhow::anyhow!(e))?;
        let resp = self.http.get(url).send().await?;
        if !resp.status().is_success() {
            return Err(http_error(resp).await);
        }
        let headers = resp.headers().clone();
        let body = parse_json(resp).await.unwrap_or(JsonValue::Null);
        let mut out = serde_json::Map::new();
        for (k, v) in headers.iter() {
            let key = k.as_str().to_ascii_lowercase();
            if key.contains("limit") || key.contains("remaining") || key.contains("reset") {
                if let Ok(s) = v.to_str() {
                    out.insert(key, JsonValue::String(s.to_string()));
                }
            }
        }
        out.insert("sample".into(), body);
        Ok(JsonValue::Object(out))
    }

    pub async fn photos_search(&self, query: &str, cli: &crate::cli::Cli) -> Result<JsonValue> {
        let url = self.base_photos().join("search").map_err(|e| anyhow::anyhow!(e))?;
        let mut qp = self.pagination_qp(cli);
        qp.push(("query".into(), query.into()));
        if cli.all || cli.limit.is_some() || cli.max_pages.is_some() {
            self.req_paginated(url, qp, cli, &[("photos", "photos")]).await
        } else {
            self.req(url, qp).await
        }
    }

    pub async fn photos_curated(&self, cli: &crate::cli::Cli) -> Result<JsonValue> {
        let url = self.base_photos().join("curated").map_err(|e| anyhow::anyhow!(e))?;
        let qp = self.pagination_qp(cli);
        if cli.all || cli.limit.is_some() || cli.max_pages.is_some() {
            self.req_paginated(url, qp, cli, &[("photos", "photos")]).await
        } else {
            self.req(url, qp).await
        }
    }

    pub async fn photos_get(&self, id: &str) -> Result<JsonValue> {
        let url = self
            .base_photos()
            .join(&format!("photos/{}", id))
            .map_err(|e| anyhow::anyhow!(e))?;
        self.req(url, vec![]).await
    }

    pub async fn videos_search(&self, query: &str, cli: &crate::cli::Cli) -> Result<JsonValue> {
        let url = self.base_videos().join("search").map_err(|e| anyhow::anyhow!(e))?;
        let mut qp = self.pagination_qp(cli);
        qp.push(("query".into(), query.into()));
        if cli.all || cli.limit.is_some() || cli.max_pages.is_some() {
            self.req_paginated(url, qp, cli, &[("videos", "videos")]).await
        } else {
            self.req(url, qp).await
        }
    }
    pub async fn videos_popular(&self, cli: &crate::cli::Cli) -> Result<JsonValue> {
        let url = self.base_videos().join("popular").map_err(|e| anyhow::anyhow!(e))?;
        let qp = self.pagination_qp(cli);
        if cli.all || cli.limit.is_some() || cli.max_pages.is_some() {
            self.req_paginated(url, qp, cli, &[("videos", "videos")]).await
        } else {
            self.req(url, qp).await
        }
    }
    pub async fn videos_get(&self, id: &str) -> Result<JsonValue> {
        let url = self
            .base_videos()
            .join(&format!("videos/{}", id))
            .map_err(|e| anyhow::anyhow!(e))?;
        self.req(url, vec![]).await
    }

    pub async fn collections_list(&self, cli: &crate::cli::Cli) -> Result<JsonValue> {
        let url = self.base_photos().join("collections").map_err(|e| anyhow::anyhow!(e))?;
        let qp = self.pagination_qp(cli);
        if cli.all || cli.limit.is_some() || cli.max_pages.is_some() {
            self.req_paginated(url, qp, cli, &[("collections", "collections")]).await
        } else {
            self.req(url, qp).await
        }
    }
    pub async fn collections_featured(&self, cli: &crate::cli::Cli) -> Result<JsonValue> {
        let url = self
            .base_photos()
            .join("collections/featured")
            .map_err(|e| anyhow::anyhow!(e))?;
        let qp = self.pagination_qp(cli);
        if cli.all || cli.limit.is_some() || cli.max_pages.is_some() {
            self.req_paginated(url, qp, cli, &[("collections", "collections")]).await
        } else {
            self.req(url, qp).await
        }
    }
    pub async fn collections_get(&self, id: &str) -> Result<JsonValue> {
        let url = self
            .base_photos()
            .join(&format!("collections/{}", id))
            .map_err(|e| anyhow::anyhow!(e))?;
        self.req(url, vec![]).await
    }
    pub async fn collections_items(&self, id: &str, cli: &crate::cli::Cli) -> Result<JsonValue> {
        let url = self
            .base_photos()
            .join(&format!("collections/{}/media", id))
            .map_err(|e| anyhow::anyhow!(e))?;
        let qp = self.pagination_qp(cli);
        if cli.all || cli.limit.is_some() || cli.max_pages.is_some() {
            self.req_paginated(url, qp, cli, &[("media", "media")]).await
        } else {
            self.req(url, qp).await
        }
    }

    pub async fn util_inspect(&self) -> Result<JsonValue> {
        Ok(serde_json::json!({
            "host": self.cfg.host.clone().unwrap_or_else(|| "https://api.pexels.com".into()),
            "timeout": self.cfg.timeout_secs,
            "locale": self.cfg.locale,
            "max_retries": self.cfg.max_retries,
        }))
    }
    pub async fn util_ping(&self) -> Result<()> {
        // lightweight: HEAD curated
        let url = self.base_photos().join("curated").map_err(|e| anyhow::anyhow!(e))?;
        let resp = self.http.head(url).send().await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(http_error(resp).await)
        }
    }

    pub fn pagination_qp(&self, cli: &crate::cli::Cli) -> Vec<(String, String)> {
        let mut qp = vec![];
        if let Some(p) = cli.page {
            qp.push(("page".into(), p.to_string()));
        }
        if let Some(pp) = cli.per_page {
            qp.push(("per_page".into(), pp.to_string()));
        }
        qp
    }

    async fn req_paginated(
        &self,
        url: Url,
        qp: Vec<(String, String)>,
        cli: &crate::cli::Cli,
        item_keys: &[(
            &str, // input key
            &str, // output key
        )],
    ) -> Result<JsonValue> {
        let mut next = Some((url, qp));
        let mut pages = 0u32;
        let mut collected = 0u32;
        let limit = cli.limit.unwrap_or(u32::MAX);
        let max_pages = cli.max_pages.unwrap_or(u32::MAX);
        let mut aggregate = serde_json::Map::new();
        // seed arrays
        for (_, out_key) in item_keys.iter() {
            aggregate.insert((*out_key).to_string(), JsonValue::Array(vec![]));
        }
        while let Some((u, q)) = next.take() {
            if pages >= max_pages || collected >= limit {
                break;
            }
            let resp = self.req(u.clone(), q.clone()).await?;
            // copy non-array metadata on first page
            if pages == 0 {
                if let Some(obj) = resp.as_object() {
                    for (k, v) in obj.iter() {
                        if !item_keys.iter().any(|(ik, _)| ik == k) && k != "next_page" {
                            aggregate.insert(k.clone(), v.clone());
                        }
                    }
                }
            }
            // merge arrays
            for (in_key, out_key) in item_keys.iter() {
                if let Some(arr) = resp.get(*in_key).and_then(|v| v.as_array()) {
                    let cur = aggregate.get_mut(&out_key.to_string()).unwrap();
                    let dest = cur.as_array_mut().unwrap();
                    for item in arr {
                        if collected < limit {
                            dest.push(item.clone());
                            collected += 1;
                        }
                    }
                }
            }
            pages += 1;
            if collected >= limit || pages >= max_pages {
                break;
            }
            if let Some(next_url) = resp.get("next_page").and_then(|v| v.as_str()) {
                if let Ok(parsed) = Url::parse(next_url) {
                    next = Some((parsed, vec![]));
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        Ok(JsonValue::Object(aggregate))
    }
}

async fn parse_json(resp: Response) -> Result<JsonValue> {
    let bytes = resp.bytes().await?;
    let v: JsonValue = serde_json::from_slice(&bytes).unwrap_or(JsonValue::String(String::from_utf8_lossy(&bytes).to_string()));
    Ok(v)
}

async fn http_error(resp: Response) -> anyhow::Error {
    let status = resp.status();
    let rid = resp.headers().get("x-request-id").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    let text = resp.text().await.unwrap_or_default();
    let mut err = serde_json::Map::new();
    err.insert("code".into(), JsonValue::Number(status.as_u16().into()));
    err.insert("reason".into(), JsonValue::String(status.canonical_reason().unwrap_or("error").to_string()));
    // attempt to parse Pexels error body to extract type/hint
    if let Ok(v) = serde_json::from_str::<JsonValue>(&text) {
        if let Some(t) = v.get("type").cloned() { err.insert("type".into(), t); }
        if let Some(h) = v.get("hint").cloned() { err.insert("hint".into(), h); }
    }
    if let Some(id) = rid { err.insert("request_id".into(), JsonValue::String(id)); }
    if !text.is_empty() { err.insert("body".into(), JsonValue::String(text)); }
    anyhow::anyhow!(serde_yaml::to_string(&JsonValue::Object(err)).unwrap_or_else(|_| format!("http error {}", status)))
}

fn retry_after_delay(resp: &Response, attempt: u32, override_secs: Option<u64>) -> Duration {
    if let Some(ov) = override_secs { return Duration::from_secs(ov); }
    if let Some(h) = resp.headers().get("retry-after").and_then(|v| v.to_str().ok()).and_then(|s| s.parse::<u64>().ok()) {
        return Duration::from_secs(h);
    }
    backoff_delay(attempt)
}

fn redact(s: &str) -> String {
    s.replace(|c: char| c.is_ascii_graphic(), "*")
}
