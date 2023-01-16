use std::{
    collections::HashMap,
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};

use axum::{http::StatusCode, routing::post, Json, Router};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tracing::debug;
use verification_lawyer::environment::{StepWise, ValidationResult};
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

            #[derive(Debug, Deserialize)]
            struct Hook {
                id: u64,
            }

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
    }
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

    sh.change_dir(git_path);
    cmd!(sh, "git pull").run()?;

    let run: RunOption = toml::from_str(&sh.read_file("run.toml")?)?;

    eprintln!("{run:?}");

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

    let mut score: HashMap<_, u64> = HashMap::default();

    for _ in 0..100 {
        let result = verification_lawyer::run_analysis(
            StepWise,
            sh.current_dir(),
            None,
            None,
            &run_command,
            "interpreter",
        )?;
        let result = match result {
            ValidationResult::CorrectTerminated => ValidationResult::CorrectTerminated,
            ValidationResult::CorrectNonTerminated => ValidationResult::CorrectNonTerminated,
            ValidationResult::Mismatch { reason } => ValidationResult::Mismatch {
                reason: "...".to_string(),
            },
            ValidationResult::TimeOut => ValidationResult::TimeOut,
        };
        *score.entry(result).or_default() += 1;
    }

    eprintln!("{score:?}");

    Ok(())
}
