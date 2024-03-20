mod env_template;

use std::path::{Path, PathBuf};

use clap::Parser;
use color_eyre::Result;
use heck::{ToKebabCase, ToPascalCase, ToSnakeCase};
use itertools::Itertools;
use toml_edit::Document;
use xshell::cmd;

#[derive(Debug, Parser)]
enum Cli {
    /// Create a new environment
    ///
    /// Example usage:
    ///   cargo xtask new-env --short-name "calculator" --long-name "Calculator"
    NewEnv {
        /// The short name of the environment
        ///
        /// For example, for sign analysis this is "sign"
        #[clap(long)]
        short_name: String,
        /// The long name of the environment
        ///
        /// For example, for sign analysis this is "Sign Analysis"
        #[clap(long)]
        long_name: String,
    },
}

async fn run() -> Result<()> {
    match Cli::parse() {
        Cli::NewEnv {
            short_name,
            long_name,
        } => {
            let sh = xshell::Shell::new()?;

            let crate_name = format!("ce-{}", short_name.to_kebab_case());

            // NOTE: Setup new crate
            sh.change_dir(project_root());
            sh.change_dir("envs");
            cmd!(sh, "cargo new --lib {crate_name}").run()?;
            sh.change_dir(&crate_name);

            cmd!(
                sh,
                "cargo add ce-core serde serde_json tracing tapi itertools"
            )
            .run()?;
            let template_src =
                include_str!("./env_template.rs").replace("Template", &short_name.to_pascal_case());
            sh.write_file("./src/lib.rs", template_src)?;

            // NOTE: Add crate to project Cargo.toml
            sh.change_dir(project_root());
            let toml = sh.read_file("Cargo.toml")?;
            let mut doc = toml.parse::<Document>()?;
            let table = [("path".to_string(), format!("./envs/{crate_name}"))]
                .into_iter()
                .collect::<toml_edit::InlineTable>();
            doc["workspace"]["dependencies"]
                .as_table_mut()
                .unwrap()
                .insert(
                    &crate_name,
                    toml_edit::Item::Value(toml_edit::Value::InlineTable(table)),
                );
            doc["workspace"]["dependencies"]
                .as_table_mut()
                .unwrap()
                .sort_values();
            sh.write_file("Cargo.toml", doc.to_string())?;

            // NOTE: Add crate to shell
            sh.change_dir(project_root());
            sh.change_dir("ce-shell");
            cmd!(sh, "cargo add {crate_name}").run()?;
            let shell_file = "src/lib.rs";
            let shell = sh.read_file(shell_file)?;
            let marker = "define_shell!(";
            let define_shell_start = shell.find(marker).unwrap() + marker.len();
            let define_shell_end =
                define_shell_start + shell[define_shell_start..].find(')').unwrap();

            let mut envs = shell[define_shell_start..define_shell_end]
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect_vec();
            envs.push(format!(
                r#"{ce_snake}::{pascal}Env[{pascal}, {long_name:?}],"#,
                ce_snake = crate_name.to_snake_case(),
                pascal = short_name.to_pascal_case(),
            ));
            envs.sort();
            for env in &mut envs {
                *env = format!("    {env}");
            }
            let new_shell = format!(
                "{}\n{}\n{}",
                &shell[0..define_shell_start],
                envs.iter().format("\n"),
                &shell[define_shell_end..]
            );
            sh.write_file(shell_file, new_shell)?;

            // NOTE: Create +page for the new environment
            sh.change_dir(project_root());
            sh.change_dir("inspectify-app/src/routes/env");

            let template_src = include_str!(
                "../../inspectify/app/src/routes/(inspectify)/env/Template/+page.svelte"
            )
            .replace("Parser", &short_name.to_pascal_case());
            sh.create_dir(short_name.to_pascal_case())?;
            sh.change_dir(short_name.to_pascal_case());
            sh.write_file("./+page.svelte", template_src)?;
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
