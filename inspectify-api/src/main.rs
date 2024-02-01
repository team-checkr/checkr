mod endpoints;

use std::{net::SocketAddr, path::PathBuf};

use axum::Router;
use clap::Parser;
use tapi::RouterExt;
use tracing_subscriber::prelude::*;

use crate::endpoints::AppState;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    tracing_subscriber::Registry::default()
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
        .with(tracing_subscriber::filter::FilterFn::new(|m| {
            !m.target().contains("hyper")
        }))
        .init();

    run().await
}

#[derive(Debug, Parser)]
#[command(version)]
struct Cli {
    /// Automatically open inspectify in the browser
    #[clap(short, long, default_value_t = false)]
    open: bool,
    /// Location of the directory containing `run.toml`
    #[clap(default_value = ".")]
    dir: PathBuf,
    /// The port to host the server on
    #[clap(short, long, default_value = "3000")]
    port: u16,
    // /// Update the binary to the latest release from GitHub
    // #[clap(short = 'u', long, default_value_t = false)]
    // self_update: bool,
}

async fn run() -> color_eyre::Result<()> {
    let cli = Cli::parse();

    let endpoints = endpoints::endpoints().with_ty::<ce_shell::Envs>();

    let run_toml_path = cli.dir.join("run.toml");
    let hub = driver::Hub::new(cli.dir.clone())?;
    let driver = driver::Driver::new_from_path(hub.clone(), run_toml_path)?;
    if let Some(job) = driver.start_recompile() {
        job?;
    }

    driver.spawn_watcher()?;

    let api = Router::new()
        .tapis(&endpoints)
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(AppState { hub, driver });
    let app = Router::new().nest("/api", api);

    if !populate_ts_client(&endpoints) {
        println!("{}", endpoints.ts_client());
    }

    let addr = SocketAddr::from(([127, 0, 0, 1], cli.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Write the TypeScript client to the inspectify-app/src/lib/api.ts file if it exists.
///
/// Returns `true` if the file exists, `false` otherwise.
fn populate_ts_client(endpoints: &tapi::Endpoints<AppState>) -> bool {
    let ts_client_path = std::path::PathBuf::from("./inspectify-app/src/lib/api.ts");
    // write TypeScript client if and only if the path already exists
    if ts_client_path.exists() {
        // only write if the contents are different
        let ts_client = endpoints.ts_client();
        let prev = std::fs::read_to_string(&ts_client_path).unwrap_or_default();
        if prev != ts_client {
            let _ = std::fs::write(&ts_client_path, ts_client);
        }
        true
    } else {
        false
    }
}
