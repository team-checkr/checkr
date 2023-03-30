use std::{
    fs,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use checko::{
    config::{
        CanonicalProgramsConfig, CanonicalProgramsEnvConfig, GroupConfig, GroupsConfig,
        ProgramsConfig,
    },
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
    /// Parse all provided program TOML files and print out in a canonicalized format.
    DumpPrograms {
        /// The configs file specifying the programs to run in the competition.
        #[clap(long, short)]
        programs: Vec<PathBuf>,
    },
    /// Run the programs for all groups and store the results.
    RunTests {
        /// The configs file specifying the programs to run in the competition.
        #[clap(long, short)]
        programs: Vec<PathBuf>,
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
fn collect_programs(
    programs: impl IntoIterator<Item = impl AsRef<Path>>,
) -> Result<ProgramsConfig> {
    programs
        .into_iter()
        .map(read_programs)
        .reduce(|acc, p| {
            let mut acc = acc?;
            acc.extend(p?);
            Ok(acc)
        })
        .unwrap_or_else(|| Ok(Default::default()))
}

async fn run() -> Result<()> {
    match Cli::parse() {
        Cli::DumpPrograms { programs } => {
            let envs = collect_programs(programs)?
                .envs
                .iter()
                .map(|(&analysis, env)| {
                    (
                        analysis,
                        CanonicalProgramsEnvConfig {
                            programs: env
                                .programs
                                .iter()
                                .map(|p| p.canonicalize(analysis).unwrap())
                                .collect(),
                        },
                    )
                })
                .collect();

            println!(
                "{}",
                toml::to_string_pretty(&CanonicalProgramsConfig { envs })?
            );

            Ok(())
        }
        Cli::RunTests {
            programs,
            groups,
            submissions,
            local_docker,
            concurrent,
        } => {
            let programs = collect_programs(programs)?;
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

            let num_groups = groups.groups.len();
            for (g_idx, g) in groups.groups.into_iter().enumerate() {
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

                        // TODO: Replace this with a try-block when they are stable
                        let result: Result<()> = (|| async {
                            let sh = env
                                .shell_in_default_branch()
                                .wrap_err("getting shell in default branch")?;
                            let cwd = sh.current_dir();
                            drop(sh);
                            let output =
                                TestRunInput::run_in_docker(&image, &cwd, programs.clone()).await?;
                            match &output.data {
                                checko::test_runner::TestRunData::CompileError(_) => {
                                    warn!("failed to compile. compile error saved")
                                }
                                checko::test_runner::TestRunData::Sections(_) => {}
                            }
                            info!(
                                file = env.latest_run_path().display().to_string(),
                                "writing result"
                            );
                            env.write_latest_run(&output)?;
                            Ok(())
                        })()
                        .await;

                        if let Err(e) = result {
                            error!(error = e.to_string(), "errored");
                            eprintln!("{e:?}");
                        }
                    }
                    .instrument(span!(
                        Level::INFO,
                        "Group",
                        name = name,
                        nr = format!("{}/{num_groups}", g_idx + 1)
                    )),
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
            let mut groups_with_errors = vec![];

            for g in &groups.groups {
                let span = span!(Level::INFO, "Group", name = g.name);
                let _enter = span.enter();

                let run = || -> Result<()> {
                    let env = GroupEnv::new(&submissions, g);
                    let data = env.latest_run()?;
                    let report = IndividualMarkdown {
                        group_name: g.name.clone(),
                        data,
                    };

                    let sh = env.shell_in_results_branch()?;

                    sh.write_file("README.md", report.to_string())?;
                    cmd!(sh, "git add .").run()?;
                    let now = chrono::DateTime::<chrono::Utc>::from(std::time::SystemTime::now());
                    let msg = format!("Ran tests at {}", now.format("%+"));
                    if execute {
                        info!("pushing to results branch");
                        cmd!(sh, "git commit -m {msg}").run()?;
                        retry(
                            5.try_into().expect("it's positive"),
                            Duration::from_millis(500),
                            || cmd!(sh, "git push --force --set-upstream origin results").run(),
                        )?;
                    } else {
                        info!("skipping push to results branch");
                        info!("(skipping) > git commit -m {msg:?}");
                        info!("(skipping) > git push --force --set-upstream origin results");
                    }
                    Ok(())
                };

                if let Err(err) = run() {
                    error!("failed to push");
                    eprintln!("{err:?}");
                    groups_with_errors.push(g);
                }
            }

            error!("{} groups errored", groups_with_errors.len());
            for g in groups_with_errors {
                error!(group = g.name, "errored");
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
                    Ok(data) => match data.data {
                        checko::test_runner::TestRunData::CompileError(msg) => {
                            error!("did not have a latest run (they failed to compile)");
                            eprintln!("{msg}");
                        }
                        checko::test_runner::TestRunData::Sections(sections) => {
                            for sec in sections {
                                input
                                    .sections
                                    .entry(sec.analysis)
                                    .or_default()
                                    .entry(g.name.clone())
                                    .or_insert(sec.programs);
                            }
                        }
                    },
                    Err(e) => {
                        error!("did not have a latest run");
                        eprintln!("{e:?}");
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
    fn shell_in_default_branch(&self) -> Result<Shell> {
        let sh = self.shell_in_folder()?;
        sh.remove_path("repo")?;

        let before_clone = std::time::Instant::now();
        let git = &self.config.git;
        let dst = sh.current_dir().join("repo");
        info!(repo = git, dst = dst.display().to_string(), "cloning");
        cmd!(sh, "git clone --filter=blob:none --no-checkout {git} {dst}")
            .ignore_stdout()
            .ignore_stderr()
            .quiet()
            .run()?;

        sh.change_dir("repo");

        // TODO: This should not be hardcoded to master, but rather look up the default branch
        cmd!(sh, "git checkout master")
            .ignore_stdout()
            .ignore_stderr()
            .quiet()
            .run()?;
        info!(took = format!("{:?}", before_clone.elapsed()), "cloned");

        // TODO: possibly change to the latest commit just before a deadline

        Ok(sh)
    }
    fn shell_in_results_branch(&self) -> Result<Shell> {
        let sh = self.shell_in_default_branch()?;

        retry(
            5.try_into().expect("it's positive"),
            Duration::from_millis(500),
            || -> Result<()> {
                if cmd!(sh, "git checkout results").run().is_err() {
                    cmd!(sh, "git switch --orphan results").run()?;
                }
                cmd!(sh, "git reset --hard").run()?;
                cmd!(sh, "git clean -xdf").run()?;
                if let Err(err) = cmd!(sh, "git pull").run() {
                    warn!("failed to pull, but continuing anyway");
                    eprintln!("{err:?}");
                }
                Ok(())
            },
        )?;

        Ok(sh)
    }
}

fn retry<T, E>(
    tries: NonZeroUsize,
    delay: Duration,
    mut f: impl FnMut() -> Result<T, E>,
) -> Result<T, E> {
    let mut error = None;

    for _ in 0..tries.get() {
        match f() {
            Ok(res) => return Ok(res),
            Err(err) => error = Some(err),
        }
        std::thread::sleep(delay);
    }

    Err(error.unwrap())
}
