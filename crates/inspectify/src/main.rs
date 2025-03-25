mod checko;
mod endpoints;

use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

use axum::{
    Router,
    response::{Html, IntoResponse},
};
use clap::Parser;
use endpoints::InspectifyJobMeta;
use tapi::{Tapi, endpoints::RouterExt};
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
    /// Location of `run.toml` relative to `dir`
    #[clap(long, default_value = "run.toml")]
    run: PathBuf,
    /// The port to host the server on
    #[clap(short, long, default_value = "3000")]
    port: u16,
    /// Watch for file changes and recompile automatically
    #[clap(long, default_value = "true")]
    watch: Option<bool>,
    /// Start the default driver
    ///
    /// If false, the server will not start the default driver and will not
    /// watch for file changes.
    #[clap(long, default_value = "true")]
    driver: Option<bool>,
    /// The path to the checko SQLite database
    #[clap(long)]
    checko: Option<PathBuf>,
}

async fn run() -> color_eyre::Result<()> {
    let cli = Cli::parse();

    let dir = dunce::canonicalize(&cli.dir)?;

    let hub = driver::Hub::new()?;
    let driver = if cli.driver == Some(false) {
        None
    } else {
        let run_toml_path = dir.join(cli.run);
        let driver = driver::Driver::new_from_path(hub.clone(), dir.clone(), run_toml_path)?;
        let _: Option<_> = driver.start_recompile(InspectifyJobMeta::default());
        Some(driver)
    };

    let public_state = Arc::new(std::sync::RwLock::new(None));
    let checko = if let Some(checko_path) = cli.checko {
        let checko = Arc::new(checko::Checko::open(hub.clone(), &checko_path)?);
        tokio::spawn({
            let checko = Arc::clone(&checko);
            async move {
                checko.work().await.unwrap();
            }
        });
        tokio::spawn({
            let checko = Arc::clone(&checko);
            let public_state = Arc::clone(&public_state);
            async move {
                loop {
                    *public_state.write().unwrap() =
                        Some(checko::scoreboard::compute_public_state(&checko).await);
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        });
        Some(checko)
    } else {
        if let Some(driver) = &driver {
            if cli.watch != Some(false) {
                driver.spawn_watcher(InspectifyJobMeta::default())?;
            }
        }
        None
    };

    let endpoints = endpoints::endpoints().with_ty::<ce_shell::Envs>();

    let api = Router::new()
        .tapis(&endpoints)
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(AppState {
            hub,
            driver,
            checko,
            public_state,
        });
    let app = Router::new().nest("/api", api).fallback(static_dir);

    populate_ts_client(&endpoints);
    populate_fs_types(&ce_shell::Envs::all_dependencies());

    if cli.open {
        tokio::task::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            open::that(format!("http://localhost:{}", cli.port)).unwrap();
        });
    }

    let addr = SocketAddr::from(([127, 0, 0, 1], cli.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    {
        use color_eyre::owo_colors::OwoColorize;

        println!(
            r#"
  {name} {version}

  {arrow}  {visit}:   {url}
  {arrow}  use {open} to open in browser
    "#,
            name = "Inspectify".bold().bright_green(),
            version = format!("v{}", env!("CARGO_PKG_VERSION")).green(),
            arrow = "➜".green(),
            visit = "Visit".bold(),
            url = format!("http://{addr:?}/").cyan(),
            open = "--open".bold(),
        );
    }
    axum::serve(listener, app).await?;

    Ok(())
}

pub async fn static_dir(uri: axum::http::Uri) -> impl axum::response::IntoResponse {
    #[derive(rust_embed::RustEmbed)]
    #[folder = "../../apps/inspectify/build/"]
    struct Frontend;

    if uri.path() == "/" {
        if let Some(index) = Frontend::get("index.html") {
            return Html(index.data).into_response();
        } else {
            return Html("Frontend has not been build for release yet! Visit <a href=\"http://localhost:5173/\">localhost:5173</a> for the development site!").into_response();
        }
    }

    let get = |path: String| Frontend::get(&path).map(|file| (path, file));

    let plain = get(uri.path()[1..].to_string());
    let html = get(format!("{}.html", &uri.path()[1..]));

    match (plain, html) {
        (Some((path, file)), _) | (_, Some((path, file))) => {
            let mime_type = mime_guess::from_path(path)
                .first_raw()
                .map(axum::http::HeaderValue::from_static)
                .unwrap_or_else(|| {
                    axum::http::HeaderValue::from_str(
                        mime_guess::mime::APPLICATION_OCTET_STREAM.as_ref(),
                    )
                    .unwrap()
                });
            ([(axum::http::header::CONTENT_TYPE, mime_type)], file.data).into_response()
        }
        _ => axum::http::StatusCode::NOT_FOUND.into_response(),
    }
}

/// Write the TypeScript client to the apps/inspectify/src/lib/api.ts file if it exists.
///
/// Returns `true` if the file exists, `false` otherwise.
fn populate_ts_client(endpoints: &tapi::endpoints::Endpoints<AppState>) -> bool {
    let ts_client_path = std::path::PathBuf::from("./apps/inspectify/src/lib/api.ts");
    // write TypeScript client if and only if the path already exists
    if ts_client_path.exists() {
        // only write if the contents are different
        let contents = endpoints.ts_client();
        let prev = std::fs::read_to_string(&ts_client_path).unwrap_or_default();
        if prev != contents {
            let _ = std::fs::write(&ts_client_path, contents);
        }
        true
    } else {
        false
    }
}

/// Write the F# types to the starters/fsharp-starter/Io.fs
///
/// Returns `true` if the file exists, `false` otherwise.
fn populate_fs_types(tys: &[tapi::DynTapi]) -> bool {
    let fs_types_path = std::path::PathBuf::from("./starters/fsharp-starter/src/Io.fs");
    // write F# types if and only if the path already exists
    if fs_types_path.exists() {
        // only write if the contents are different
        let contents = tapi::targets::fs::builder().types(tys.iter().copied());
        let prev = std::fs::read_to_string(&fs_types_path).unwrap_or_default();
        if prev != contents {
            let _ = std::fs::write(&fs_types_path, contents);
        }
        true
    } else {
        false
    }
}
