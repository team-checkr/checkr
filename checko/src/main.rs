use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use checko::{
    config::{GroupConfig, GroupsConfig, ProgramsConfig},
    fmt::{CompetitionMarkdown, IndividualMarkdown},
    test_runner::{TestRunInput, TestRunResults},
};

use clap::Parser;
use tracing::{error, info, span, Level};
use xshell::{cmd, Shell};

#[derive(Debug, Parser)]
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

async fn run() -> Result<()> {
    match Cli::parse() {
        Cli::RunTests {
            programs,
            groups,
            submissions,
        } => {
            let programs: ProgramsConfig = toml::from_str(&fs::read_to_string(programs)?)?;
            let groups: GroupsConfig = toml::from_str(&fs::read_to_string(groups)?)?;

            // NOTE: This could easily be parallelized using rayon, but for the
            // time being we keep it single threaded for easier debugging.
            for g in &groups.groups {
                let span = span!(Level::INFO, "Group", g.name);
                let _enter = span.enter();
                info!("evaluating group");
                let env = GroupEnv::new(&submissions, g);
                let sh = match env.shell_in_default_branch() {
                    Ok(sh) => sh,
                    Err(err) => {
                        error!(
                            error = format!("{err:?}"),
                            "getting shell in default branch"
                        );
                        continue;
                    }
                };
                let output = TestRunInput::run_in_docker(&sh, programs.clone())?;
                info!("writing result");
                env.write_latest_run(&output)?;
            }

            Ok(())
        }
        Cli::PushResultsToRepos {
            groups,
            submissions,
            execute,
        } => {
            let groups: GroupsConfig = toml::from_str(&fs::read_to_string(groups)?)?;

            for g in &groups.groups {
                let span = span!(Level::TRACE, "Group: {}", g.name);
                let _enter = span.enter();
                let env = GroupEnv::new(&submissions, g);
                let data = env.latest_run()?;
                let report = IndividualMarkdown {
                    group_name: g.name.clone(),
                    num_shown: 2,
                    data,
                };

                let sh = env.shell_in_results_branch()?;

                sh.write_file("README.md", report.to_string())?;
                cmd!(sh, "git add .").run()?;
                let msg = format!("Ran tests at {:?}", std::time::Instant::now());
                if execute {
                    info!("pushing to results branch");
                    cmd!(sh, "git commit -m {msg}").run()?;
                    cmd!(sh, "git push --force --set-upstream origin results").run()?;
                } else {
                    info!("skipping push to results branch");
                    eprintln!("(skipping) > git commit -m {msg:?}");
                    eprintln!("(skipping) > git push --force --set-upstream origin results");
                }
            }

            Ok(())
        }
        Cli::GenerateCompetition {
            groups,
            submissions,
            output,
        } => {
            let groups: GroupsConfig = toml::from_str(&fs::read_to_string(groups)?)?;

            let mut input = CompetitionMarkdown::default();

            for g in &groups.groups {
                let span = span!(Level::TRACE, "Group: {}", g.name);
                let _enter = span.enter();
                let data = GroupEnv::new(&submissions, g).latest_run()?;

                for sec in data.sections {
                    input
                        .sections
                        .entry(sec.analysis)
                        .or_default()
                        .entry(g.name.clone())
                        .or_insert(sec.programs);
                }
            }

            info!("writing output");
            fs::write(output, input.to_string())?;

            Ok(())
        }
        Cli::InternalSingleCompetition { input } => {
            let sh = Shell::new()?;
            TestRunInput::run_from_within_docker(&sh, &input)?;
            Ok(())
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .without_time()
        .init();
    run().await
}

fn determine_primary_branch(sh: &Shell) -> Result<String> {
    let result = cmd!(sh, "git symbolic-ref refs/remotes/origin/HEAD").read()?;
    Ok(result
        .split('/')
        .last()
        .expect("no primary branch")
        .to_string())
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
        Ok(serde_json::from_str(&fs::read_to_string(
            self.latest_run_path(),
        )?)?)
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
        if sh.read_dir("repo").is_err() {
            let git = &self.config.git;
            cmd!(sh, "git clone {git} repo").run()?;
        }
        sh.change_dir("repo");

        let primary_branch = determine_primary_branch(&sh)?;

        cmd!(sh, "git reset --hard").run()?;
        cmd!(sh, "git clean -xdf").run()?;
        cmd!(sh, "git checkout {primary_branch}").run()?;
        cmd!(sh, "git pull").run()?;

        Ok(sh)
    }
    fn shell_in_results_branch(&self) -> Result<Shell> {
        let sh = self.shell_in_default_branch()?;
        if cmd!(sh, "git checkout results").run().is_err() {
            cmd!(sh, "git switch --orphan results").run()?;
        }
        cmd!(sh, "git reset --hard").run()?;
        cmd!(sh, "git clean -xdf").run()?;
        cmd!(sh, "git pull").run()?;
        Ok(sh)
    }
}
