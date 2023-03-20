use std::path::{Path, PathBuf};

use clap::Parser;
use color_eyre::{eyre::ContextCompat, Result};
use toml_edit::Document;
use xshell::cmd;

#[derive(Debug, Parser)]
enum Cli {
    RegenerateCi {},
    UpdateStartBinaries {},
}

async fn run() -> Result<()> {
    match Cli::parse() {
        Cli::RegenerateCi {} => {
            let sh = xshell::Shell::new()?;

            sh.change_dir(project_root());

            let toml = sh.read_file("Cargo.toml")?;
            let mut doc = toml.parse::<Document>()?;
            if let Some(profile) = doc.get_mut("profile").and_then(|p| p.as_table_mut()) {
                profile.remove_entry("dist");
            }
            sh.write_file("Cargo.toml", doc.to_string())?;

            // NOTE: Installers produced a strange 404 error in CI. Disable for the moment.
            // cmd!(sh, "cargo dist init --ci=github --installer=github-shell --installer=github-powershell").run()?;
            cmd!(sh, "cargo dist init --ci=github").run()?;
            sh.write_file(
                "Cargo.toml",
                sh.read_file("Cargo.toml")?.trim().to_string() + "\n",
            )?;

            const ASDF: &str = r#"
      - name: Install just and typeshare
        uses: taiki-e/install-action@v2
        with:
          tool: just
      - name: Build UI
        run: just build-ui"#;

            const RELEASE_FILE: &str = ".github/workflows/release.yml";
            let mut ci = sh.read_file(RELEASE_FILE)?;

            let just_before_str = "run: ${{ matrix.install-dist }}";
            let pos = ci
                .find(just_before_str)
                .wrap_err("did not find magic string in release.yml")?;

            ci.insert_str(pos + just_before_str.len(), ASDF);
            let ci = ci
                .replace(
                    "rustup update stable && rustup default stable",
                    &["rustup update nightly", "rustup default nightly"].join(" && "),
                )
                .trim_end()
                .to_string()
                + "\n";
            sh.write_file(RELEASE_FILE, ci)?;
        }
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
