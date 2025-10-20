use crate::api::PexelsClient;
use crate::config::{Config, TokenSource};
use crate::output::emit_raw_bytes;
use crate::output::{emit_data, wrap_ok, OutputFormat};
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json::Value as JsonValue;
// std::time::Duration not used here

#[derive(Parser, Debug)]
#[command(
    name = "pexels",
    about = "Pexels CLI",
    disable_help_subcommand = true,
    after_help = r#"Examples:
  pexels auth status
  pexels photos search -q cats
  pexels photos curated
  pexels videos popular
  pexels collections featured"#
)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    /// JSON output
    #[arg(long, global = true)]
    pub json: bool,
    /// Raw output (HTTP body)
    #[arg(long, global = true)]
    pub raw: bool,
    /// Fields selection (dot paths or sets)
    #[arg(long, global = true)]
    pub fields: Vec<String>,
    /// Page number
    #[arg(long, global = true)]
    pub page: Option<u32>,
    /// Per page
    #[arg(long = "per-page", global = true)]
    pub per_page: Option<u32>,
    /// Fetch all pages
    #[arg(long, global = true)]
    pub all: bool,
    /// Limit total items when --all
    #[arg(long, global = true)]
    pub limit: Option<u32>,
    /// Max pages when --all
    #[arg(long = "max-pages", global = true)]
    pub max_pages: Option<u32>,
    /// jq expression passthrough (not executed in CLI, forwarded intent)
    #[arg(long, global = true)]
    pub jq: Option<String>,
    /// jmes expression passthrough
    #[arg(long, global = true)]
    pub jmes: Option<String>,
    /// Timeout seconds
    #[arg(long, global = true, default_value_t = 15)]
    pub timeout: u64,
    /// Max retries
    #[arg(long = "max-retries", global = true, default_value_t = 3)]
    pub max_retries: u32,
    /// Retry-After cap seconds (override)
    #[arg(long = "retry-after", global = true)]
    pub retry_after: Option<u64>,
    /// Host override for testing
    #[arg(long, global = true)]
    pub host: Option<String>,
    /// Locale for Accept-Language
    #[arg(long, global = true)]
    pub locale: Option<String>,
    /// Verbose logging
    #[arg(long, global = true)]
    pub verbose: bool,
    /// Debug logging
    #[arg(long, global = true)]
    pub debug: bool,
    /// Color control
    #[arg(long, global = true, value_enum)]
    pub color: Option<ColorChoice>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ColorChoice {
    Always,
    Auto,
    Never,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Auth(AuthCmd),
    Config(ConfigCmd),
    Quota(QuotaCmd),
    Photos(PhotosCmd),
    Videos(VideosCmd),
    Collections(CollectionsCmd),
    Util(UtilCmd),
}

#[derive(Args, Debug)]
pub struct AuthCmd {
    #[command(subcommand)]
    sub: AuthSub,
}
#[derive(Subcommand, Debug)]
pub enum AuthSub {
    Login { token: Option<String> },
    Status,
    TokenSource,
    Logout,
}

#[derive(Args, Debug)]
pub struct ConfigCmd {
    #[command(subcommand)]
    sub: ConfigSub,
}
#[derive(Subcommand, Debug)]
pub enum ConfigSub {
    Set { key: String, value: String },
    Get { key: String },
    Path,
}

#[derive(Args, Debug)]
pub struct QuotaCmd {
    #[command(subcommand)]
    sub: QuotaSub,
}
#[derive(Subcommand, Debug)]
pub enum QuotaSub {
    View,
}

#[derive(Args, Debug)]
pub struct PhotosCmd {
    #[command(subcommand)]
    sub: PhotosSub,
}
#[derive(Subcommand, Debug)]
pub enum PhotosSub {
    Search {
        #[arg(short = 'q', long = "query")]
        query: String,
    },
    Curated,
    Get {
        id: String,
    },
    /// Return canonical photo URL (src.original)
    Url {
        id: String,
        /// Size variant from src.* (default: original)
        #[arg(long, value_enum)]
        size: Option<PhotoSize>,
    },
    /// Download the original photo bytes to path
    Download {
        id: String,
        path: String,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum PhotoSize {
    #[value(name = "original")]
    Original,
    #[value(name = "large2x")]
    Large2x,
    #[value(name = "large")]
    Large,
    #[value(name = "medium")]
    Medium,
    #[value(name = "small")]
    Small,
    #[value(name = "portrait")]
    Portrait,
    #[value(name = "landscape")]
    Landscape,
    #[value(name = "tiny")]
    Tiny,
}

impl PhotoSize {
    fn key(&self) -> &'static str {
        match self {
            PhotoSize::Original => "original",
            PhotoSize::Large2x => "large2x",
            PhotoSize::Large => "large",
            PhotoSize::Medium => "medium",
            PhotoSize::Small => "small",
            PhotoSize::Portrait => "portrait",
            PhotoSize::Landscape => "landscape",
            PhotoSize::Tiny => "tiny",
        }
    }
}

#[derive(Args, Debug)]
pub struct VideosCmd {
    #[command(subcommand)]
    sub: VideosSub,
}
#[derive(Subcommand, Debug)]
pub enum VideosSub {
    Search { query: String },
    Popular,
    Get { id: String },
}

#[derive(Args, Debug)]
pub struct CollectionsCmd {
    #[command(subcommand)]
    sub: CollectionsSub,
}
#[derive(Subcommand, Debug)]
pub enum CollectionsSub {
    List,
    Featured,
    Get { id: String },
    Items { id: String },
}

#[derive(Args, Debug)]
pub struct UtilCmd {
    #[command(subcommand)]
    sub: UtilSub,
}
#[derive(Subcommand, Debug)]
pub enum UtilSub {
    Inspect,
    Ping,
}

pub async fn run(cli: Cli) -> Result<()> {
    // Load config and build client
    let mut cfg = Config::load().context("load config")?;
    cfg.apply_env();
    cfg.apply_cli(&cli);

    let client = PexelsClient::new(cfg.clone())?;

    match &cli.command {
        Commands::Auth(auth) => run_auth(auth, cfg).await,
        Commands::Config(cmd) => run_config(cmd, cfg).await,
        Commands::Quota(cmd) => run_quota(cmd, client, &cli).await,
        Commands::Photos(cmd) => run_photos(cmd, client, &cli).await,
        Commands::Videos(cmd) => run_videos(cmd, client, &cli).await,
        Commands::Collections(cmd) => run_collections(cmd, client, &cli).await,
        Commands::Util(cmd) => run_util(cmd, client, &cli).await,
    }
}

fn fmt_from_cli(cli: &Cli) -> OutputFormat {
    if cli.raw {
        OutputFormat::Raw
    } else if cli.json {
        OutputFormat::Json
    } else {
        OutputFormat::Yaml
    }
}

async fn run_auth(cmd: &AuthCmd, mut cfg: Config) -> Result<()> {
    match &cmd.sub {
        AuthSub::Login { token } => {
            let token = token
                .clone()
                .or_else(|| std::env::var("PEXELS_TOKEN").ok())
                .or_else(|| std::env::var("PEXELS_API_KEY").ok())
                .context("token not provided; use --token or env PEXELS_TOKEN")?;
            cfg.token = Some(token);
            cfg.token_source = Some(TokenSource::Config);
            cfg.save()?;
            let payload = serde_json::json!({
                "status": "ok",
                "message": "token saved"
            });
            let out = wrap_ok(
                &payload,
                Some(serde_json::json!({
                    "next_page": null,
                    "prev_page": null
                })),
            );
            emit_data(&OutputFormat::Yaml, &out)
        }
        AuthSub::Status => {
            let (src, present) = cfg.token_source_with_presence();
            let payload = serde_json::json!({
                "source": src,
                "present": present
            });
            let out = wrap_ok(
                &payload,
                Some(serde_json::json!({
                    "next_page": null,
                    "prev_page": null
                })),
            );
            emit_data(&OutputFormat::Yaml, &out)
        }
        AuthSub::TokenSource => {
            let (src, _present) = cfg.token_source_with_presence();
            let payload = serde_json::json!({"source": src});
            let out = wrap_ok(
                &payload,
                Some(serde_json::json!({
                    "next_page": null,
                    "prev_page": null
                })),
            );
            emit_data(&OutputFormat::Yaml, &out)
        }
        AuthSub::Logout => {
            cfg.token = None;
            cfg.token_source = Some(TokenSource::None);
            cfg.save()?;
            let payload = serde_json::json!({"status":"logged out"});
            let out = wrap_ok(
                &payload,
                Some(serde_json::json!({
                    "next_page": null,
                    "prev_page": null
                })),
            );
            emit_data(&OutputFormat::Yaml, &out)
        }
    }
}

async fn run_config(cmd: &ConfigCmd, mut cfg: Config) -> Result<()> {
    match &cmd.sub {
        ConfigSub::Set { key, value } => {
            match key.as_str() {
                "token" | "api_key" => cfg.token = Some(value.clone()),
                _ => anyhow::bail!("unsupported key"),
            }
            cfg.save()?;
            let payload = serde_json::json!({"status":"ok"});
            let out = wrap_ok(
                &payload,
                Some(serde_json::json!({
                    "next_page": null,
                    "prev_page": null
                })),
            );
            emit_data(&OutputFormat::Yaml, &out)
        }
        ConfigSub::Get { key } => {
            let v = match key.as_str() {
                "token" | "api_key" => cfg.token.clone().unwrap_or_default(),
                _ => String::new(),
            };
            emit_data(&OutputFormat::Raw, &JsonValue::String(v))
        }
        ConfigSub::Path => emit_data(
            &OutputFormat::Raw,
            &JsonValue::String(cfg.path().display().to_string()),
        ),
    }
}

async fn run_quota(_cmd: &QuotaCmd, client: PexelsClient, cli: &Cli) -> Result<()> {
    // Reachability check: HEAD curated
    let reachable = client.util_ping().await.is_ok();
    let mut data = client
        .quota_view()
        .await
        .unwrap_or_else(|_| serde_json::json!({}));
    if let Some(obj) = data.as_object_mut() {
        obj.insert("reachable".into(), serde_json::json!(reachable));
    }
    emit_enveloped(cli, data, &DefaultFields::None)
}

async fn run_photos(cmd: &PhotosCmd, client: PexelsClient, cli: &Cli) -> Result<()> {
    match &cmd.sub {
        PhotosSub::Search { query } => {
            let data = client.photos_search(query, cli).await?;
            emit_enveloped(cli, data, &DefaultFields::Photos)
        }
        PhotosSub::Curated => {
            if cli.raw {
                let url = client
                    .base_photos()
                    .join("curated")
                    .map_err(|e| anyhow::anyhow!(e))?;
                let bytes = client.req_bytes(url, client.pagination_qp(cli)).await?;
                emit_raw_bytes(&bytes)
            } else {
                let data = client.photos_curated(cli).await?;
                emit_enveloped(cli, data, &DefaultFields::Photos)
            }
        }
        PhotosSub::Get { id } => {
            let data = client.photos_get(id).await?;
            emit_enveloped(cli, data, &DefaultFields::Photos)
        }
        PhotosSub::Url { id, size } => {
            let data = client.photos_get(id).await?;
            let size = size.unwrap_or(PhotoSize::Original);
            let url = data
                .get("src")
                .and_then(|v| v.get(size.key()))
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!(format!("src.{} not found", size.key())))?;
            let fmt = fmt_from_cli(cli);
            let out = serde_json::json!({
                "data": url,
                "meta": { "id": id, "size": size.key() }
            });
            emit_data(&fmt, &out)
        }
        PhotosSub::Download { id, path } => {
            let data = client.photos_get(id).await?;
            let url = data
                .get("src")
                .and_then(|v| v.get("original"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("src.original not found"))?;
            // download bytes
            let bytes = client.download_url_bytes(url).await?;
            // write file
            use std::fs::{self, File};
            use std::io::Write as _;
            use std::path::Path;
            let p = Path::new(path);
            if let Some(dir) = p.parent() {
                fs::create_dir_all(dir)?;
            }
            let mut f = File::create(p)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = f.metadata()?.permissions();
                perms.set_mode(0o600);
                f.set_permissions(perms)?;
            }
            f.write_all(&bytes)?;
            let abs = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
            let fmt = fmt_from_cli(cli);
            let out = serde_json::json!({
                "data": { "path": abs.display().to_string(), "bytes": bytes.len() },
                "meta": { "id": id, "url": url }
            });
            emit_data(&fmt, &out)
        }
    }
}

async fn run_videos(cmd: &VideosCmd, client: PexelsClient, cli: &Cli) -> Result<()> {
    let data = match &cmd.sub {
        VideosSub::Search { query } => client.videos_search(query, cli).await?,
        VideosSub::Popular => client.videos_popular(cli).await?,
        VideosSub::Get { id } => client.videos_get(id).await?,
    };
    emit_enveloped(cli, data, &DefaultFields::Videos)
}

async fn run_collections(cmd: &CollectionsCmd, client: PexelsClient, cli: &Cli) -> Result<()> {
    let data = match &cmd.sub {
        CollectionsSub::List => client.collections_list(cli).await?,
        CollectionsSub::Featured => client.collections_featured(cli).await?,
        CollectionsSub::Get { id } => client.collections_get(id).await?,
        CollectionsSub::Items { id } => client.collections_items(id, cli).await?,
    };
    emit_enveloped(cli, data, &DefaultFields::Collections)
}

async fn run_util(cmd: &UtilCmd, client: PexelsClient, cli: &Cli) -> Result<()> {
    match &cmd.sub {
        UtilSub::Inspect => {
            let data = client.util_inspect().await?;
            emit_wrapped(&fmt_from_cli(cli), &data)
        }
        UtilSub::Ping => {
            client.util_ping().await?;
            emit_wrapped(&fmt_from_cli(cli), &serde_json::json!({"ok":true}))
        }
    }
}

enum DefaultFields {
    None,
    Photos,
    Videos,
    Collections,
}

fn emit_enveloped(cli: &Cli, data: JsonValue, defaults: &DefaultFields) -> Result<()> {
    let fmt = fmt_from_cli(cli);
    let fields = if cli.fields.is_empty() {
        match defaults {
            DefaultFields::None => vec![],
            DefaultFields::Photos => vec![
                "id".into(),
                "photographer".into(),
                "alt".into(),
                "width".into(),
                "height".into(),
                "avg_color".into(),
            ],
            DefaultFields::Videos => vec!["duration".into(), "width".into(), "height".into()],
            DefaultFields::Collections => {
                vec!["title".into(), "description".into(), "media_count".into()]
            }
        }
    } else {
        cli.fields.clone()
    };

    if matches!(fmt, OutputFormat::Raw) {
        let s = serde_json::to_string(&data)?;
        return emit_raw_bytes(s.as_bytes());
    }

    // New pipeline: compute meta from full response, extract items, then project items and wrap.
    use serde_json::Value as V;
    let (data_val, meta) = shape_output(&data);
    let out = match (&data, &data_val) {
        (V::Object(_obj), V::Array(items)) => {
            let projected_items = crate::proj::project_items_with_fallback(items, &fields);
            wrap_ok(&V::Array(projected_items), Some(meta))
        }
        _ => {
            // Single-resource path: project object as a whole with fallback to avoid empty {}
            let projected = if let V::Object(_) = &data {
                crate::proj::project_item_with_fallback(&data, &fields)
            } else {
                crate::proj::project(&data, &fields)
            };
            wrap_ok(&projected, Some(meta))
        }
    };
    emit_data(&fmt, &out)
}

// Convert API response into the new output shape
// - data: items array for list endpoints, or object for single-resource
// - meta: includes total_results?, next_page?, prev_page?, request_id? (best effort)
// - remove page/per_page from output
pub fn shape_output(input: &JsonValue) -> (JsonValue, JsonValue) {
    use serde_json::{json, Value};
    let mut meta = serde_json::Map::new();
    // Move known meta keys if present
    if let Some(n) = input.get("total_results").and_then(|v| v.as_u64()) {
        meta.insert("total_results".into(), json!(n));
    }
    // next/prev can be URLs; convert to ints
    let next_page_num = input.get("next_page").and_then(|v| {
        v.as_u64()
            .map(|u| u as u32)
            .or_else(|| v.as_str().and_then(crate::output::parse_page_number))
    });
    let prev_page_num = input.get("prev_page").and_then(|v| {
        v.as_u64()
            .map(|u| u as u32)
            .or_else(|| v.as_str().and_then(crate::output::parse_page_number))
    });
    meta.insert(
        "next_page".into(),
        match next_page_num {
            Some(p) => json!(p),
            None => Value::Null,
        },
    );
    meta.insert(
        "prev_page".into(),
        match prev_page_num {
            Some(p) => json!(p),
            None => Value::Null,
        },
    );
    // Data extraction: prefer items arrays
    if let Some(obj) = input.as_object() {
        for key in ["photos", "videos", "collections", "media"] {
            if let Some(Value::Array(items)) = obj.get(key) {
                let data = Value::Array(items.clone());
                return (data, Value::Object(meta));
            }
        }
    }
    (input.clone(), Value::Object(meta))
}

fn emit_wrapped(fmt: &OutputFormat, payload: &JsonValue) -> Result<()> {
    let out = wrap_ok(
        payload,
        Some(serde_json::json!({
            "next_page": null,
            "prev_page": null,
        })),
    );
    emit_data(fmt, &out)
}
