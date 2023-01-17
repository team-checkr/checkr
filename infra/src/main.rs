use std::{
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};

use axum::{http::StatusCode, routing::post, Json, Router};
use clap::Parser;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tracing::debug;
use verification_lawyer::{
    env::{Environment, SecurityEnv, SignEnv, StepWiseEnv, ToMarkdown, ValidationResult},
    AnalysisSummary,
};
use xshell::{cmd, Shell};

#[derive(Debug, Parser)]
enum Cli {
    Test {
        #[clap(long, short)]
        base: PathBuf,
        config: PathBuf,
    },
    Server {
        #[clap(long, short, default_value = "25565")]
        port: String,
    },
    GenerateReport {
        dir: PathBuf,
        #[clap(long, short)]
        group_nr: u64,
        #[clap(long, short)]
        output: PathBuf,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    groups: Vec<GroupConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GroupConfig {
    name: String,
    git: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RunOption {
    run: String,
    compile: Option<String>,
}

async fn run() -> anyhow::Result<()> {
    match Cli::parse() {
        Cli::Test { base, config } => {
            let config: Config = toml::from_str(&fs::read_to_string(config)?)?;

            for g in &config.groups {
                if let Err(e) = test_group(&base, g) {
                    eprintln!("Group {} errored: {e:?}", g.name)
                }
            }

            Ok(())
        }
        Cli::Server { port } => {
            use gitlab::api::AsyncQuery;

            let glab = gitlab::GitlabBuilder::new("gitlab.gbar.dtu.dk", "N-aZmK-zJSDCT4JYRUx6")
                .build_async()
                .await?;

            let result: serde_json::Value = gitlab::api::groups::Groups::builder()
                .all_available(true)
                .build()?
                .query_async(&glab)
                .await?;
            debug!("{result:?}");

            let pid = "verification-lawyer-dev-env/demo-group-01";

            let result: Vec<gitlab::Hook> = gitlab::api::projects::hooks::Hooks::builder()
                .project(pid)
                .build()?
                .query_async(&glab)
                .await?;
            debug!("{result:?}");

            for h in result {
                tokio::time::sleep(Duration::from_millis(1000)).await;
                debug!("Deleting {h:?}");
                let result: serde_json::Value = gitlab::api::projects::hooks::DeleteHook::builder()
                    .project(pid)
                    .hook_id(h.id.value())
                    .build()?
                    .query_async(&glab)
                    .await?;
                debug!("deleted {h:?}: {result:?}");
            }

            tokio::time::sleep(Duration::from_millis(1000)).await;

            let result: serde_json::Value = gitlab::api::projects::hooks::CreateHook::builder()
                .push_events(true)
                .project(pid)
                .url("http://2.108.179.189:25565")
                .build()?
                .query_async(&glab)
                .await?;
            debug!("{result:?}");

            let app = Router::new()
                .route(
                    "/",
                    post(|data: Json<gitlab::webhooks::WebHook>| async move {
                        debug!("{data:#?}");
                        StatusCode::OK
                    }),
                )
                .layer(ServiceBuilder::new().layer(tower_http::trace::TraceLayer::new_for_http()));
            axum::Server::bind(&SocketAddr::from(([0; 4], 25565)))
                .serve(app.into_make_service())
                .await?;
            Ok(())
        }
        Cli::GenerateReport {
            dir,
            group_nr,
            output,
        } => {
            let sh = Shell::new()?;
            sh.change_dir(dir);

            cmd!(sh, "git checkout master").run()?;
            cmd!(sh, "git pull").run()?;

            let result = generate_report(&sh, group_nr)?;
            fs::write(output, result)?;
            Ok(())
        }
    }
}

fn generate_report(sh: &Shell, group_nr: impl std::fmt::Display) -> anyhow::Result<String> {
    use std::fmt::Write;

    let run: RunOption = toml::from_str(&sh.read_file("run.toml")?)?;

    if let Some(compile) = &run.compile {
        let mut args = compile.split(' ');
        let program = args.next().unwrap();

        let mut cmd = std::process::Command::new(program);
        cmd.args(args);
        cmd.current_dir(sh.current_dir());

        let output = cmd.output()?;
        eprintln!("{output:?}");
    }

    let run_command = if run.run.starts_with('.') {
        let (cmd, args) = run.run.split_once(' ').unwrap_or((&run.run, ""));

        format!("{} {}", sh.current_dir().join(cmd).to_string_lossy(), args)
            .trim()
            .to_string()
    } else {
        run.run
    };

    let mut output = String::new();

    let base_seed = 1234123_1234;

    let samples = 10;

    writeln!(output, "# Group {group_nr}")?;

    let env = StepWiseEnv;
    let summaries = (0..samples)
        .map(|idx| {
            verification_lawyer::run_analysis(
                &env,
                sh.current_dir(),
                None,
                Some(base_seed + idx),
                &run_command,
            )
        })
        .collect_vec();
    generate_markdown(&env, &mut output, &summaries)?;

    let env = SignEnv;
    let summaries = (0..samples)
        .map(|idx| {
            verification_lawyer::run_analysis(
                &env,
                sh.current_dir(),
                None,
                Some(base_seed + idx),
                &run_command,
            )
        })
        .collect_vec();
    generate_markdown(&env, &mut output, &summaries)?;

    let env = SecurityEnv;
    let summaries = (0..samples)
        .map(|idx| {
            verification_lawyer::run_analysis(
                &env,
                sh.current_dir(),
                None,
                Some(base_seed + idx),
                &run_command,
            )
        })
        .collect_vec();
    generate_markdown(&env, &mut output, &summaries)?;

    let mut table = comfy_table::Table::new();
    table
        .load_preset(comfy_table::presets::ASCII_MARKDOWN)
        .set_header(["Result", "Explanation"])
        .add_row(["Correct", "Nice job! :)"])
        .add_row([
            "Correct<sup>*</sup>",
            "The program ran correctly for the first {iterations} steps",
        ])
        .add_row(["Mismatch", "The result did not match the expected output"])
        .add_row(["Error", "Unable to parse the output"]);
    writeln!(output, "\n## Result explanations")?;
    writeln!(output, "\n{table}")?;

    Ok(output)
}

fn generate_markdown<E: Environment>(
    env: &E,
    mut f: impl std::fmt::Write,
    summaries: &[AnalysisSummary<E>],
) -> anyhow::Result<()>
where
    E::Input: std::fmt::Debug + ToMarkdown,
    E::Output: ToMarkdown,
{
    writeln!(f, "## {}", env.name())?;
    for (idx, summary) in summaries.iter().enumerate().take(2) {
        writeln!(f, "<details><summary>")?;
        writeln!(f, "<strong>Program {}</strong> – ", idx + 1)?;
        match &summary.result {
            Ok(ValidationResult::CorrectTerminated) => {
                writeln!(f, "Correct")?;
            }
            Ok(ValidationResult::CorrectNonTerminated { iterations }) => {
                writeln!(f, "Correct<sup>*</sup>",)?;
            }
            Ok(ValidationResult::Mismatch { reason }) => {
                writeln!(
                    f,
                    "{:?}",
                    ValidationResult::Mismatch {
                        reason: reason.to_string(),
                        // reason: "...".to_string(),
                    }
                )?;
            }
            Ok(ValidationResult::TimeOut) => {
                writeln!(f, "{:?}", ValidationResult::TimeOut)?;
            }
            Err(e) => {
                writeln!(f, "{e:?}")?;
            }
        }
        writeln!(f, "</summary>")?;
        writeln!(f)?;

        writeln!(f, "\n\n```py\n{}\n```\n\n", summary.cmds)?;
        writeln!(f, "### Input\n\n{}\n\n", summary.input.to_markdown())?;
        if let Some(output) = &summary.output {
            writeln!(f, "### Output \n\n{}\n\n", output.to_markdown())?;
        } else {
            writeln!(
                f,
                "<details><summary>`stdout`</summary><p>\n\n```json\n{}\n```\n\n</p></details>\n",
                summary.stdout
            )?;
        }

        writeln!(f, "</details>")?;
    }

    let mut table = comfy_table::Table::new();
    table
        .load_preset(comfy_table::presets::ASCII_MARKDOWN)
        .set_header(["Program", "Result", "Time"]);

    for (idx, summary) in summaries.iter().enumerate() {
        table.add_row([
            format!("Program {}", idx + 1),
            match &summary.result {
                Ok(ValidationResult::CorrectTerminated) => "Correct".to_string(),
                Ok(ValidationResult::CorrectNonTerminated { .. }) => {
                    "Correct<sup>*</sup>".to_string()
                }
                Ok(ValidationResult::Mismatch { .. }) => "Mismatch".to_string(),
                Ok(ValidationResult::TimeOut) => "Time out".to_string(),
                Err(_) => "Error".to_string(),
            },
            format!("{:?}", summary.time),
        ]);
    }
    writeln!(f, "\n{table}")?;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .without_time()
        .init();
    run().await
}

fn test_group(base: &Path, g: &GroupConfig) -> anyhow::Result<()> {
    let g_dir = base.join(&g.name);
    let sh = Shell::new()?;
    sh.create_dir(&g_dir)?;
    sh.change_dir(&g_dir);

    let git_path = if let Ok(Some(git_path)) = sh.read_dir(".").map(|x| x.first().cloned()) {
        git_path
    } else {
        let git = &g.git;
        cmd!(sh, "git clone {git}").run()?;
        sh.read_dir(".")?.first().unwrap().clone()
    };

    sh.change_dir(&git_path);
    cmd!(sh, "git reset --hard").run()?;
    cmd!(sh, "git clean -xdf").run()?;
    cmd!(sh, "git checkout master").run()?;
    cmd!(sh, "git pull").run()?;

    let report = generate_report(&sh, &g.name)?;

    if cmd!(sh, "git checkout results").run().is_err() {
        cmd!(sh, "git switch --orphan results").run()?;
    }
    cmd!(sh, "git reset --hard").run()?;
    cmd!(sh, "git clean -xdf").run()?;
    sh.write_file("README.md", report)?;
    cmd!(sh, "git add .").run()?;
    let msg = format!("Ran tests at {:?}", std::time::Instant::now());
    cmd!(sh, "git commit -m {msg}").run()?;
    cmd!(sh, "git push --force --set-upstream origin results").run()?;

    Ok(())
}
