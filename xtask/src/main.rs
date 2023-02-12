use std::path::{Path, PathBuf};

use clap::Parser;
use color_eyre::Result;

#[derive(Debug, Parser)]
enum Cli {
    UpdateStartBinaries {},
}

async fn run() -> Result<()> {
    match Cli::parse() {
        Cli::UpdateStartBinaries {} => {
            struct Target {
                triple: &'static str,
                name: &'static str,
            }

            let targets = [
                Target {
                    triple: "x86_64-apple-darwin",
                    name: "macos",
                },
                Target {
                    triple: "x86_64-pc-windows-msvc",
                    name: "win.exe",
                },
                Target {
                    triple: "x86_64-unknown-linux-gnu",
                    name: "linux",
                },
            ];

            let base = project_root().join("starters/fsharp-starter/dev");
            tokio::fs::create_dir_all(&base).await?;

            for target in targets {
                binswap_github::builder()
                    .repo_author("team-checkr")
                    .repo_name("checkr")
                    .bin_name("inspectify")
                    .add_target(target.triple)
                    .no_check_with_cmd(true)
                    .no_confirm(true)
                    .build()?
                    .fetch_and_write_to(base.join(target.name))
                    .await?;
            }
        }
    }

    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    run().await
}
