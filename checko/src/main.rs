use checko::cli::Cli;

use clap::Parser;
use color_eyre::Result;
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    tracing_subscriber::registry::Registry::default()
        .with(tracing_error::ErrorLayer::default())
        .with(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .without_time(),
        )
        .init();

    Cli::parse().run().await?;

    Ok(())
}
