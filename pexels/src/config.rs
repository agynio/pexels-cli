use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub token: Option<String>,
    #[serde(default)]
    pub token_source: Option<TokenSource>,
    #[serde(skip)]
    pub host: Option<String>,
    #[serde(skip)]
    pub timeout_secs: u64,
    #[serde(skip)]
    pub locale: Option<String>,
    #[serde(skip)]
    pub max_retries: u32,
    #[serde(skip)]
    pub retry_after: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenSource {
    Env,
    Config,
    Cli,
    None,
}

impl Default for TokenSource {
    fn default() -> Self {
        TokenSource::None
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let data = fs::read_to_string(&path).context("read config file")?;
            let mut cfg: Config = serde_yaml::from_str(&data).context("parse config yaml")?;
            cfg.timeout_secs = 15;
            cfg.max_retries = 3;
            Ok(cfg)
        } else {
            Ok(Config { timeout_secs: 15, max_retries: 3, ..Default::default() })
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = self.path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).context("create config dir")?;
        }
        let data = serde_yaml::to_string(&self).context("serialize config")?;
        let mut f = fs::File::create(&path).context("create config file")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = f.metadata()?.permissions();
            perms.set_mode(0o600);
            f.set_permissions(perms)?;
        }
        f.write_all(data.as_bytes()).context("write config file")?;
        Ok(())
    }

    pub fn path(&self) -> PathBuf {
        Self::config_path()
    }

    pub fn config_path() -> PathBuf {
        // Vendorless per spec
        let proj = ProjectDirs::from("", "", "pexels").expect("config dirs");
        let path = proj.config_dir().join("config.yaml");
        path
    }

    pub fn apply_env(&mut self) {
        if let Ok(v) = std::env::var("PEXELS_TOKEN") {
            if !v.is_empty() {
                self.token = Some(v);
                self.token_source = Some(TokenSource::Env);
            }
        } else if let Ok(v) = std::env::var("PEXELS_API_KEY") {
            if !v.is_empty() {
                self.token = Some(v);
                self.token_source = Some(TokenSource::Env);
            }
        }
    }

    pub fn apply_cli(&mut self, cli: &crate::cli::Cli) {
        self.timeout_secs = cli.timeout;
        self.max_retries = cli.max_retries;
        self.retry_after = cli.retry_after;
        if let Some(host) = cli.host.clone() {
            self.host = Some(host);
        }
        if let Some(locale) = cli.locale.clone() {
            self.locale = Some(locale);
        }
    }

    pub fn token_source_with_presence(&self) -> (String, bool) {
        let present = self.token.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
        let src = match self.token_source.clone().unwrap_or(TokenSource::None) {
            TokenSource::Env => "env",
            TokenSource::Config => "config",
            TokenSource::Cli => "cli",
            TokenSource::None => "none",
        };
        (src.to_string(), present)
    }
}
