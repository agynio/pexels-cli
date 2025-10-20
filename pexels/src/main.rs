mod api;
mod cli;
mod config;
mod output;
mod proj;
mod util;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    // Init logging to stderr only
    let filter = if cli.debug {
        "debug"
    } else if cli.verbose {
        "info"
    } else {
        "warn"
    };
    let use_color = match cli.color {
        Some(cli::ColorChoice::Always) => true,
        Some(cli::ColorChoice::Never) => false,
        _ => std::env::var("NO_COLOR").is_err(),
    };
    let _ = fmt()
        .with_env_filter(EnvFilter::new(filter))
        .with_writer(std::io::stderr)
        .with_ansi(use_color)
        .try_init();

    if let Err(err) = cli::run(cli).await {
        // Render structured error to stderr
        crate::output::emit_error(&err)?;
        // Ensure non-zero exit via anyhow error
        return Err(err);
    }
    Ok(())
}
