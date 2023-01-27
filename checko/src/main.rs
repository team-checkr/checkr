use std::{
    fs,
    path::{Path, PathBuf},
};

use checko::{
    config::{GroupConfig, GroupsConfig, ProgramsConfig},
    fmt::{CompetitionMarkdown, IndividualMarkdown},
    test_runner::{TestRunInput, TestRunResults},
};

use clap::Parser;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use tracing::error;
use xshell::{cmd, Shell};

#[derive(Debug, Parser)]
enum Cli {
    Test {
        #[clap(long, short)]
        programs: PathBuf,
        #[clap(long, short)]
        groups: PathBuf,
        #[clap(short, default_value = "false")]
        no_hidden: bool,
        #[clap(long, short)]
        base: PathBuf,
    },
    /// Run and generate the results of competition
    ///
    /// This pulls down all of the repos from the group config, and build and
    /// runs the inputs from the programs config in individual containers.
    Competition {
        /// The configs file specifying the programs to run in the competition.
        #[clap(long, short)]
        programs: PathBuf,
        /// The configs file specifying the groups which are part of the competition.
        #[clap(long, short)]
        groups: PathBuf,
        /// The folder where group projects are downloaded.
        #[clap(long, short)]
        submissions: PathBuf,
        /// The path where the Markdown file for the competition results should go.
        #[clap(long, short)]
        output: PathBuf,
    },
    /// The command used within the docker container to generate competition
    /// results of a single group. This is not intended to be used by humans.
    InternalSingleCompetition { input: String },
}

async fn run() -> anyhow::Result<()> {
    match Cli::parse() {
        Cli::Test {
            programs,
            groups,
            no_hidden,
            base,
        } => {
            let programs: ProgramsConfig = toml::from_str(&fs::read_to_string(programs)?)?;
            let groups: GroupsConfig = toml::from_str(&fs::read_to_string(groups)?)?;

            for g in &groups.groups {
                if let Err(e) = test_group(&programs, no_hidden, &base, g) {
                    error!(group = g.name, error = format!("{e:?}"), "Group errored")
                }
            }

            Ok(())
        }
        Cli::Competition {
            programs,
            groups,
            submissions,
            output,
        } => {
            let programs: ProgramsConfig = toml::from_str(&fs::read_to_string(programs)?)?;
            let groups: GroupsConfig = toml::from_str(&fs::read_to_string(groups)?)?;

            let mut input = CompetitionMarkdown::default();

            let results = groups
                .groups
                .par_iter()
                .filter_map(|g| {
                    let sh = match setup_shell_in_group_repo(&submissions, g) {
                        Ok(sh) => sh,
                        Err(err) => {
                            error!(group = g.name, error = format!("{err:?}"), "Group errored");
                            return None;
                        }
                    };
                    let output = TestRunInput::run_in_docker(&sh, programs.clone()).unwrap();

                    sh.write_file("../run.json", serde_json::to_string(&output).unwrap())
                        .unwrap();

                    Some((g, output))
                })
                .collect::<Vec<(&GroupConfig, TestRunResults)>>();
            for (g, categories) in results {
                for sec in categories.sections {
                    input
                        .sections
                        .entry(sec.analysis)
                        .or_default()
                        .entry(g.name.clone())
                        .or_insert(sec.programs);
                }
            }

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
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .without_time()
        .init();
    run().await
}

fn determine_primary_branch(sh: &Shell) -> anyhow::Result<String> {
    let result = cmd!(sh, "git symbolic-ref refs/remotes/origin/HEAD").read()?;
    Ok(result
        .split('/')
        .last()
        .expect("no primary branch")
        .to_string())
}

fn setup_shell_in_group_repo(base: &Path, g: &GroupConfig) -> anyhow::Result<Shell> {
    let g_dir = base.join(&g.name);
    let sh = Shell::new()?;
    sh.create_dir(&g_dir)?;
    sh.change_dir(&g_dir);

    if sh.read_dir("repo").is_err() {
        let git = &g.git;
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

fn test_group(
    programs: &ProgramsConfig,
    no_hidden: bool,
    base: &Path,
    g: &GroupConfig,
) -> anyhow::Result<()> {
    let sh = setup_shell_in_group_repo(base, g)?;

    let data = TestRunInput::run_in_docker(&sh, programs.clone())?;
    let report = IndividualMarkdown {
        data,
        num_shown: if no_hidden {
            programs.programs.len()
        } else {
            2
        },
        group_name: g.name.clone(),
    };

    if cmd!(sh, "git checkout results").run().is_err() {
        cmd!(sh, "git switch --orphan results").run()?;
    }
    cmd!(sh, "git reset --hard").run()?;
    cmd!(sh, "git clean -xdf").run()?;
    sh.write_file("README.md", report.to_string())?;
    cmd!(sh, "git add .").run()?;
    let msg = format!("Ran tests at {:?}", std::time::Instant::now());
    // cmd!(sh, "git commit -m {msg}").run()?;
    // cmd!(sh, "git push --force --set-upstream origin results").run()?;

    Ok(())
}
