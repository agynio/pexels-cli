use crate::api::PexelsClient;
use crate::config::{Config, TokenSource};
use crate::output::{emit_data, emit_error, OutputFormat};
use crate::output::emit_raw_bytes;
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json::Value as JsonValue;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(
    name = "pexels",
    about = "Pexels CLI",
    disable_help_subcommand = true,
    after_help = r#"Examples:
  pexels photos search 'cats' --per-page 5
  pexels videos popular --json --fields @urls
  pexels collections list --all --limit 50
  PEXELS_TOKEN=... pexels quota view
  pexels --host http://localhost:8080 util ping"#
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
    Search { query: String },
    Curated,
    Get { id: String },
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
            let token = token.clone()
                .or_else(|| std::env::var("PEXELS_TOKEN").ok())
                .or_else(|| std::env::var("PEXELS_API_KEY").ok())
                .context("token not provided; use --token or env PEXELS_TOKEN")?;
            cfg.token = Some(token);
            cfg.token_source = Some(TokenSource::Config);
            cfg.save()?;
            emit_data(&OutputFormat::Yaml, &serde_json::json!({
                "status": "ok",
                "message": "token saved"
            }))
        }
        AuthSub::Status => {
            let (src, present) = cfg.token_source_with_presence();
            emit_data(&OutputFormat::Yaml, &serde_json::json!({
                "source": src,
                "present": present
            }))
        }
        AuthSub::TokenSource => {
            let (src, _present) = cfg.token_source_with_presence();
            emit_data(&OutputFormat::Yaml, &serde_json::json!({"source": src}))
        }
        AuthSub::Logout => {
            cfg.token = None;
            cfg.token_source = Some(TokenSource::None);
            cfg.save()?;
            emit_data(&OutputFormat::Yaml, &serde_json::json!({"status":"logged out"}))
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
            emit_data(&OutputFormat::Yaml, &serde_json::json!({"status":"ok"}))
        }
        ConfigSub::Get { key } => {
            let v = match key.as_str() {
                "token" | "api_key" => cfg.token.clone().unwrap_or_default(),
                _ => String::new(),
            };
            emit_data(&OutputFormat::Raw, &JsonValue::String(v))
        }
        ConfigSub::Path => {
            emit_data(&OutputFormat::Raw, &JsonValue::String(cfg.path().display().to_string()))
        }
    }
}

async fn run_quota(_cmd: &QuotaCmd, client: PexelsClient, cli: &Cli) -> Result<()> {
    // Reachability check: HEAD curated
    let reachable = client.util_ping().await.is_ok();
    let mut data = client.quota_view().await.unwrap_or_else(|_| serde_json::json!({}));
    if let Some(obj) = data.as_object_mut() {
        obj.insert("reachable".into(), serde_json::json!(reachable));
    }
    emit_projected(cli, data, &DefaultFields::None)
}

async fn run_photos(cmd: &PhotosCmd, client: PexelsClient, cli: &Cli) -> Result<()> {
    match &cmd.sub {
        PhotosSub::Search { query } => {
            let data = client.photos_search(query, cli).await?;
            emit_projected(cli, data, &DefaultFields::Photos)
        }
        PhotosSub::Curated => {
            if cli.raw {
                let url = client.base_photos().join("curated").map_err(|e| anyhow::anyhow!(e))?;
                let bytes = client.req_bytes(url, client.pagination_qp(cli)).await?;
                emit_raw_bytes(&bytes)
            } else {
                let data = client.photos_curated(cli).await?;
                emit_projected(cli, data, &DefaultFields::Photos)
            }
        }
        PhotosSub::Get { id } => {
            let data = client.photos_get(id).await?;
            emit_projected(cli, data, &DefaultFields::Photos)
        }
    }
}

async fn run_videos(cmd: &VideosCmd, client: PexelsClient, cli: &Cli) -> Result<()> {
    let data = match &cmd.sub {
        VideosSub::Search { query } => client.videos_search(query, cli).await?,
        VideosSub::Popular => client.videos_popular(cli).await?,
        VideosSub::Get { id } => client.videos_get(id).await?,
    };
    emit_projected(cli, data, &DefaultFields::Videos)
}

async fn run_collections(cmd: &CollectionsCmd, client: PexelsClient, cli: &Cli) -> Result<()> {
    let data = match &cmd.sub {
        CollectionsSub::List => client.collections_list(cli).await?,
        CollectionsSub::Featured => client.collections_featured(cli).await?,
        CollectionsSub::Get { id } => client.collections_get(id).await?,
        CollectionsSub::Items { id } => client.collections_items(id, cli).await?,
    };
    emit_projected(cli, data, &DefaultFields::Collections)
}

async fn run_util(cmd: &UtilCmd, client: PexelsClient, cli: &Cli) -> Result<()> {
    match &cmd.sub {
        UtilSub::Inspect => {
            let data = client.util_inspect().await?;
            emit_data(&fmt_from_cli(cli), &data)
        }
        UtilSub::Ping => {
            client.util_ping().await?;
            emit_data(&fmt_from_cli(cli), &serde_json::json!({"ok":true}))
        }
    }
}

enum DefaultFields {
    None,
    Photos,
    Videos,
    Collections,
}

fn emit_projected(cli: &Cli, data: JsonValue, defaults: &DefaultFields) -> Result<()> {
    let fmt = fmt_from_cli(cli);
    let fields = if cli.fields.is_empty() {
        match defaults {
            DefaultFields::None => vec![],
            DefaultFields::Photos => vec![
                "photographer".into(),
                "alt".into(),
                "width".into(),
                "height".into(),
                "avg_color".into(),
            ],
            DefaultFields::Videos => vec!["duration".into(), "width".into(), "height".into()],
            DefaultFields::Collections => vec![
                "title".into(),
                "description".into(),
                "media_count".into(),
            ],
        }
    } else {
        cli.fields.clone()
    };

    if matches!(fmt, OutputFormat::Raw) {
        // Emit exact bytes of JSON body if available
        let s = serde_json::to_string(&data)?;
        emit_raw_bytes(s.as_bytes())
    } else {
        let projected = crate::proj::project_response(&data, &fields);
        emit_data(&fmt, &projected)
    }
}
