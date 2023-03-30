use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use crate::{
    config::{CanonicalProgramsConfig, CanonicalProgramsEnvConfig, GroupsConfig, ProgramsConfig},
    docker::DockerImage,
    fmt::{CompetitionMarkdown, IndividualMarkdown},
    group_env::GroupEnv,
    retry,
    test_runner::TestRunInput,
};

use clap::Parser;
use color_eyre::{eyre::Context, Result};
use tracing::{error, info, span, warn, Instrument, Level};
use xshell::{cmd, Shell};

#[derive(Debug, Parser)]
#[command(version)]
pub enum Cli {
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

impl Cli {
    pub async fn run(self) -> Result<()> {
        match self {
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
                                    TestRunInput::run_in_docker(&image, &cwd, programs.clone())
                                        .await?;
                                match &output.data {
                                    crate::test_runner::TestRunData::CompileError(_) => {
                                        warn!("failed to compile. compile error saved")
                                    }
                                    crate::test_runner::TestRunData::Sections(_) => {}
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
                        let now =
                            chrono::DateTime::<chrono::Utc>::from(std::time::SystemTime::now());
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
                            crate::test_runner::TestRunData::CompileError(_) => {
                                error!("did not have a latest run (they failed to compile)")
                            }
                            crate::test_runner::TestRunData::Sections(sections) => {
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
}
