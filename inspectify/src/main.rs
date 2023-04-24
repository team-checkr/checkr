use clap::Parser;
use color_eyre::eyre::Context;
use inspectify::{compilation::Compilation, do_self_update, ApplicationState};
use std::{net::SocketAddr, path::PathBuf, time::Duration};
use tracing_subscriber::prelude::*;

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
    /// Update the binary to the latest release from GitHub
    #[clap(short = 'u', long, default_value_t = false)]
    self_update: bool,
}

async fn run() -> color_eyre::Result<()> {
    let cli = Cli::parse();

    if cli.self_update {
        do_self_update().await?;

        return Ok(());
    }

    let run = run_options_from_file(cli.dir.join("run.toml"))
        .wrap_err_with(|| format!("could not read {:?}", cli.dir.join("run.toml")))?;

    let compilation = Compilation::initialize(cli.dir, run).await?;

    let app = inspectify::routes::router(ApplicationState { compilation });
    // NOTE: Enable for HTTP logging
    // .layer(TraceLayer::new_for_http());

    if cli.open {
        tokio::task::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            open::that(format!("http://localhost:{}", cli.port)).unwrap();
        });
    }

    {
        use crossterm::{
            cursor,
            style::{self, Stylize},
            terminal, ExecutableCommand,
        };
        use std::io::stdout;

        stdout()
            .execute(terminal::Clear(terminal::ClearType::All))?
            .execute(cursor::MoveTo(3, 2))?
            .execute(style::PrintStyledContent("Inspectify".bold().green()))?
            .execute(style::PrintStyledContent(" is running".green()))?
            .execute(cursor::MoveTo(3, 4))?
            .execute(style::Print("  ➜  "))?
            .execute(style::PrintStyledContent("Local:".bold()))?
            .execute(style::PrintStyledContent("   http://localhost:".cyan()))?
            .execute(style::PrintStyledContent(
                cli.port.to_string().cyan().bold(),
            ))?
            .execute(style::PrintStyledContent("/".cyan()))?
            .execute(cursor::MoveTo(0, 7))?;
    }

    let addr = SocketAddr::from(([127, 0, 0, 1], cli.port));
    axum::Server::bind(&addr).serve(app).await.unwrap();

    Ok(())
}

fn run_options_from_file(
    path: impl AsRef<std::path::Path>,
) -> color_eyre::Result<checkr::config::RunOption> {
    let p = path.as_ref();
    let src = std::fs::read_to_string(p)
        .wrap_err_with(|| format!("could not read run options at {p:?}"))?;
    let parsed = toml::from_str(&src)
        .wrap_err_with(|| format!("error parsing run options from file {p:?}"))?;
    Ok(parsed)
}
