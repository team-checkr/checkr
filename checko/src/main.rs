#![feature(try_blocks)]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use checko::{
    config::{GroupConfig, GroupsConfig, ProgramsConfig},
    docker::DockerImage,
    fmt::{CompetitionMarkdown, IndividualMarkdown},
    test_runner::{TestRunInput, TestRunResults},
};

use clap::Parser;
use color_eyre::{eyre::Context, Result};
use tracing::{error, info, span, warn, Instrument, Level};
use tracing_subscriber::prelude::*;
use xshell::{cmd, Shell};

#[derive(Debug, Parser)]
#[command(version)]
enum Cli {
    /// Run the programs for all groups and store the results.
    RunTests {
        /// The configs file specifying the programs to run in the competition.
        #[clap(long, short)]
        programs: PathBuf,
        /// The configs file specifying the groups which are part of the competition.
        #[clap(long, short)]
        groups: PathBuf,
        /// The folder where group projects are downloaded.
        #[clap(long, short)]
        submissions: PathBuf,
        /// When set will build the docker image from the source
        #[clap(long, short, default_value_t = false)]
        local_docker: bool,
        /// Number of concurrent projects being evaluated.
        #[clap(long, short, default_value_t = 2)]
        concurrent: usize,
    },
    /// Generate and push results of previously run tests
    PushResultsToRepos {
        /// The configs file specifying the groups which are part of the competition.
        #[clap(long, short)]
        groups: PathBuf,
        /// The folder where group projects are downloaded.
        #[clap(long, short)]
        submissions: PathBuf,
        #[clap(long)]
        execute: bool,
    },
    /// Generate the competition Markdown from previously run tests
    GenerateCompetition {
        /// The configs file specifying the groups which are part of the competition.
        #[clap(long, short)]
        groups: PathBuf,
        /// The folder where group projects are downloaded.
        #[clap(long, short)]
        submissions: PathBuf,
        /// Where the Markdown file containing the competition results will be written.
        #[clap(long, short)]
        output: PathBuf,
    },
    /// The command used within the docker container to generate competition
    /// results of a single group. This is not intended to be used by humans.
    InternalSingleCompetition { input: String },
}

fn read_programs(programs: impl AsRef<Path>) -> Result<ProgramsConfig> {
    let p = programs.as_ref();
    let src =
        fs::read_to_string(p).wrap_err_with(|| format!("could not read programs at {p:?}"))?;
    let parsed =
        toml::from_str(&src).wrap_err_with(|| format!("error parsing programs from file {p:?}"))?;
    Ok(parsed)
}
fn read_groups(groups: impl AsRef<Path>) -> Result<GroupsConfig> {
    let p = groups.as_ref();
    let src = fs::read_to_string(p).wrap_err_with(|| format!("could not read groups at {p:?}"))?;
    let parsed =
        toml::from_str(&src).wrap_err_with(|| format!("error parsing groups from file {p:?}"))?;
    Ok(parsed)
}

async fn run() -> Result<()> {
    match Cli::parse() {
        Cli::RunTests {
            programs,
            groups,
            submissions,
            local_docker,
            concurrent,
        } => {
            let programs = read_programs(programs)?;
            let groups = read_groups(groups)?;

            // NOTE: Check of docker daemon is running
            {
                let sh = Shell::new()?;
                cmd!(sh, "docker ps")
                    .quiet()
                    .ignore_stdout()
                    .run()
                    .wrap_err("docker does not seem to be running")?;
            }

            let image = if local_docker {
                DockerImage::build_in_tree().await?
            } else {
                DockerImage::build().await?
            };

            let mut tasks = vec![];

            let task_permits = Arc::new(tokio::sync::Semaphore::new(concurrent));

            for g in groups.groups {
                let task_permits = Arc::clone(&task_permits);

                let name = g.name.clone();
                let image = image.clone();
                let submissions = submissions.clone();
                let programs = programs.clone();
                let task = tokio::spawn(
                    async move {
                        let _permit = task_permits.acquire().await.unwrap();

                        info!("evaluating group");
                        let env = GroupEnv::new(&submissions, &g);

                        let result: Result<()> = try {
                            let (sh, _) = env
                                .shell_in_default_branch()
                                .wrap_err("getting shell in default branch")?;
                            let cwd = sh.current_dir();
                            drop(sh);
                            let output =
                                TestRunInput::run_in_docker(&image, &cwd, programs.clone()).await?;
                            info!(
                                file = format!("{:?}", env.latest_run_path()),
                                "writing result"
                            );
                            env.write_latest_run(&output)?;
                        };

                        if let Err(e) = result {
                            error!(error = e.to_string(), "errored");
                        }
                    }
                    .instrument(span!(Level::INFO, "Group", name = name)),
                );
                tasks.push(task);
            }

            for task in tasks {
                task.await?;
            }

            Ok(())
        }
        Cli::PushResultsToRepos {
            groups,
            submissions,
            execute,
        } => {
            let groups = read_groups(groups)?;

            for g in &groups.groups {
                let span = span!(Level::INFO, "Group", name = g.name);
                let _enter = span.enter();
                let env = GroupEnv::new(&submissions, g);
                let data = env.latest_run()?;
                let report = IndividualMarkdown {
                    group_name: g.name.clone(),
                    data,
                };

                let (sh, _) = env.shell_in_results_branch()?;

                sh.write_file("README.md", report.to_string())?;
                cmd!(sh, "git add .").run()?;
                let msg = format!("Ran tests at {:?}", std::time::Instant::now());
                if execute {
                    info!("pushing to results branch");
                    cmd!(sh, "git commit -m {msg}").run()?;
                    if let Err(err) =
                        cmd!(sh, "git push --force --set-upstream origin results").run()
                    {
                        error!(error = format!("{err:?}"), "failed to push results");
                    }
                } else {
                    info!("skipping push to results branch");
                    info!("(skipping) > git commit -m {msg:?}");
                    info!("(skipping) > git push --force --set-upstream origin results");
                }
            }

            Ok(())
        }
        Cli::GenerateCompetition {
            groups,
            submissions,
            output,
        } => {
            let groups = read_groups(groups)?;

            let mut input = CompetitionMarkdown::default();

            for g in &groups.groups {
                let span = span!(Level::INFO, "Group", name = g.name);
                let _enter = span.enter();
                match GroupEnv::new(&submissions, g).latest_run() {
                    Ok(data) => {
                        for sec in data.sections {
                            input
                                .sections
                                .entry(sec.analysis)
                                .or_default()
                                .entry(g.name.clone())
                                .or_insert(sec.programs);
                        }
                    }
                    Err(e) => {
                        error!(err = format!("{e}"), "did not have a latest run")
                    }
                }
            }

            info!("writing output");
            fs::write(output, input.to_string())?;

            Ok(())
        }
        Cli::InternalSingleCompetition { input } => {
            let sh = Shell::new()?;
            TestRunInput::run_from_within_docker(&sh, &input).await?;
            Ok(())
        }
    }
}

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

    run().await?;

    Ok(())
}

struct GroupEnv<'a> {
    submissions_folder: &'a Path,
    config: &'a GroupConfig,
}

impl<'a> GroupEnv<'a> {
    fn new(submissions_folder: &'a Path, config: &'a GroupConfig) -> Self {
        Self {
            submissions_folder,
            config,
        }
    }

    fn latest_run_path(&self) -> PathBuf {
        self.group_folder().join("run.json")
    }
    fn write_latest_run(&self, run: &TestRunResults) -> Result<()> {
        fs::write(self.latest_run_path(), serde_json::to_string(run)?)?;
        Ok(())
    }
    fn latest_run(&self) -> Result<TestRunResults> {
        let p = self.latest_run_path();
        let src = fs::read_to_string(&p)
            .wrap_err_with(|| format!("could not read latest run at {p:?}"))?;
        let parsed = serde_json::from_str(&src)
            .wrap_err_with(|| format!("error parsing latest run from file {p:?}"))?;
        Ok(parsed)
    }
    fn group_folder(&self) -> PathBuf {
        self.submissions_folder.join(&self.config.name)
    }
    fn shell_in_folder(&self) -> Result<Shell> {
        let g_dir = self.group_folder();
        let sh = Shell::new()?;
        sh.create_dir(&g_dir)?;
        sh.change_dir(&g_dir);
        Ok(sh)
    }
    fn shell_in_default_branch(&self) -> Result<(Shell, gix::Repository)> {
        let sh = self.shell_in_folder()?;
        sh.remove_path("repo")?;

        info!(repo = self.config.git, "cloning repo");
        let (mut prepare_checkout, _) = gix::prepare_clone(
            gix::Url::try_from(&*self.config.git)?,
            sh.current_dir().join("repo"),
        )?
        .fetch_then_checkout(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;

        let (repo, _) = prepare_checkout
            .main_worktree(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;
        info!(repo = self.config.git, "cloned");

        sh.change_dir("repo");

        // TODO: possibly change to the latest commit just before a deadline

        Ok((sh, repo))
    }
    fn shell_in_results_branch(&self) -> Result<(Shell, gix::Repository)> {
        let (sh, repo) = self.shell_in_default_branch()?;

        if cmd!(sh, "git checkout results").run().is_err() {
            cmd!(sh, "git switch --orphan results").run()?;
        }
        cmd!(sh, "git reset --hard").run()?;
        cmd!(sh, "git clean -xdf").run()?;
        if let Err(err) = cmd!(sh, "git pull").run() {
            warn!(
                error = format!("{err:?}"),
                "failed to pull, but continuing anyway",
            );
        }
        Ok((sh, repo))
    }
}
