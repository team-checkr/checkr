use std::{
    cmp::Reverse,
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use clap::Parser;
use infra::RunOption;
use itertools::Itertools;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use tracing::error;
use verification_lawyer::{
    driver::Driver,
    env::{
        pv::ProgramVerificationEnv, Analysis, Environment, InterpreterEnv, SecurityEnv, SignEnv,
        ToMarkdown, ValidationResult,
    },
};
use xshell::{cmd, Shell};

#[derive(Debug, Parser)]
enum Cli {
    Test {
        #[clap(short, default_value = "false")]
        no_hidden: bool,
        #[clap(long, short)]
        base: PathBuf,
        config: PathBuf,
    },
    Report {
        dir: PathBuf,
        #[clap(long, short)]
        group_nr: u64,
        #[clap(long, short, default_value_t = false)]
        pull: bool,
        #[clap(long, default_value_t = false)]
        no_hidden: bool,
        #[clap(long, short)]
        output: PathBuf,
    },
    /// Run and generate the results of competition
    ///
    /// This pulls down all of the repos from the config, and build and runs the
    /// tests in individual containers.
    Competition {
        /// The folder where the student projects will be downloaded
        #[clap(long, short)]
        base: PathBuf,
        /// The file where the resulting markdown file will be written
        #[clap(long, short)]
        output: PathBuf,
        /// The file containing the configuration for the class
        config: PathBuf,
    },
    /// The command used within the docker container to generate competition
    /// results of a single group
    InternalSingleCompetition { input: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    base_seed: u64,
    samples: u64,
    groups: Vec<GroupConfig>,
}

impl Config {
    fn seeds(&self) -> impl Iterator<Item = u64> + '_ {
        (0..self.samples).map(|seed| seed + self.base_seed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GroupConfig {
    name: String,
    git: String,
}

const DEFAULT_BASE_SEED: u64 = 12341231234;
const DEFAULT_SAMPLES: u64 = 10;

#[derive(Debug, Serialize, Deserialize)]
struct SingleCompetitionInput {
    group_name: String,
    base_seed: u64,
    samples: u64,
}

impl SingleCompetitionInput {
    const RESULT_FILE: &'static str = "result.json";

    fn run_in_docker(&self, sh: &Shell) -> anyhow::Result<IndividualMarkdown> {
        let cwd = sh.current_dir();

        let input = serde_json::to_string(self).unwrap();

        const DOCKER_IMAGE_NAME: &str = "vl-infra";
        const DOCKER_BINARY_NAME: &str = "infra";
        const SINGLE_COMPETITION_CMD: &str = "internal-single-competition";
        let cmd = [
            DOCKER_IMAGE_NAME,
            DOCKER_BINARY_NAME,
            SINGLE_COMPETITION_CMD,
        ];

        cmd!(
            sh,
            "docker run -w /root/code --rm -v {cwd}:/root/code {cmd...} {input}"
        )
        .run()
        .unwrap();

        let output = sh.read_file(Self::RESULT_FILE).unwrap();

        Ok(serde_json::from_str(&output)?)
    }
    fn run_from_within_docker(input: &str) -> anyhow::Result<()> {
        let sh = Shell::new()?;

        let input: SingleCompetitionInput = serde_json::from_str(input)?;

        let results = GroupResults::generate(
            &Config {
                base_seed: input.base_seed,
                samples: input.samples,
                groups: vec![],
            },
            &input.group_name,
            &sh,
        )?;

        sh.write_file(Self::RESULT_FILE, serde_json::to_string(&results)?)?;

        Ok(())
    }
}

async fn run() -> anyhow::Result<()> {
    match Cli::parse() {
        Cli::Test {
            no_hidden,
            base,
            config,
        } => {
            let config: Config = toml::from_str(&fs::read_to_string(config)?)?;

            for g in &config.groups {
                if let Err(e) = test_group(&config, no_hidden, &base, g) {
                    error!(group = g.name, error = format!("{e:?}"), "Group errored")
                }
            }

            Ok(())
        }
        Cli::Report {
            dir,
            group_nr,
            pull,
            no_hidden,
            output,
        } => {
            let sh = Shell::new()?;
            sh.change_dir(dir);

            if pull {
                let primary_branch = determine_primary_branch(&sh)?;
                cmd!(sh, "git checkout {primary_branch}").run()?;
                cmd!(sh, "git pull").run()?;
            }

            let result = SingleCompetitionInput {
                group_name: group_nr.to_string(),
                base_seed: DEFAULT_BASE_SEED,
                samples: DEFAULT_SAMPLES,
            }
            .run_in_docker(&sh)?;

            fs::write(output, result.to_string())?;
            Ok(())
        }
        Cli::Competition {
            base,
            config,
            output,
        } => {
            let config: Config = toml::from_str(&fs::read_to_string(config)?)?;

            let mut input = CompetitionInput::default();

            let results = config
                .groups
                .par_iter()
                .filter_map(|g| {
                    let sh = match setup_shell_in_group(&base, g) {
                        Ok(sh) => sh,
                        Err(err) => {
                            error!(group = g.name, error = format!("{err:?}"), "Group errored");
                            return None;
                        }
                    };
                    let input = SingleCompetitionInput {
                        group_name: g.name.clone(),
                        base_seed: config.base_seed,
                        samples: config.samples,
                    };
                    let output = input.run_in_docker(&sh).unwrap();

                    Some((g, output))
                })
                .collect::<Vec<(&GroupConfig, IndividualMarkdown)>>();
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

            let result = input.generate_markdown()?;
            fs::write(output, result)?;

            Ok(())
        }
        Cli::InternalSingleCompetition { input } => {
            SingleCompetitionInput::run_from_within_docker(&input)?;
            Ok(())
        }
    }
}

struct GroupResults<'a> {
    config: &'a Config,
    driver: &'a Driver,

    sections: Vec<IndividualMarkdownSection>,
}

impl GroupResults<'_> {
    fn generate(
        config: &Config,
        group_name: &str,
        sh: &Shell,
    ) -> anyhow::Result<IndividualMarkdown> {
        let run: RunOption = toml::from_str(&sh.read_file("run.toml")?)?;
        let driver = run.driver(sh.current_dir())?;

        let mut results = GroupResults {
            config,
            driver: &driver,
            sections: vec![],
        };

        results
            .push(&InterpreterEnv)
            .push(&SignEnv)
            .push(&SecurityEnv)
            .push(&ProgramVerificationEnv);
        // .push(&GraphEnv);

        Ok(IndividualMarkdown {
            group_name: group_name.to_string(),
            num_shown: 2,
            sections: results.sections,
        })
    }
    fn push<E: Environment>(&mut self, env: &E) -> &mut Self
    where
        E::Input: ToMarkdown,
        E::Output: ToMarkdown,
    {
        self.sections.push(IndividualMarkdownSection {
            analysis: E::ANALYSIS,
            programs: generate_test_results(self.config, env, self.driver),
        });
        self
    }
}

fn generate_test_results<E: Environment>(
    config: &Config,
    env: &E,
    driver: &Driver,
) -> Vec<TestResult>
where
    E::Input: ToMarkdown,
    E::Output: ToMarkdown,
{
    config
        .seeds()
        .map(|seed| {
            let summary = env
                .setup_generation()
                .seed(Some(seed))
                .build()
                .run_analysis(env, driver);
            TestResult {
                analysis: E::ANALYSIS,
                src: summary.cmds.to_string(),
                input_json: serde_json::to_string(&summary.input)
                    .expect("failed to serialize input"),
                result: match summary.result {
                    Ok(r) => match r {
                        ValidationResult::CorrectTerminated => TestResultType::CorrectTerminated,
                        ValidationResult::CorrectNonTerminated { iterations } => {
                            TestResultType::CorrectNonTerminated { iterations }
                        }
                        ValidationResult::Mismatch { reason } => {
                            TestResultType::Mismatch { reason }
                        }
                        ValidationResult::TimeOut => TestResultType::TimeOut,
                    },
                    Err(err) => TestResultType::Error {
                        description: err.to_string(),
                    },
                },
                time: summary.time,
            }
        })
        .collect_vec()
}

#[derive(Debug, Serialize, Deserialize)]
enum TestResultType {
    CorrectTerminated,
    CorrectNonTerminated { iterations: u64 },
    Mismatch { reason: String },
    TimeOut,
    Error { description: String },
}

#[derive(Debug, Serialize, Deserialize)]
struct TestResult {
    analysis: Analysis,
    src: String,
    input_json: String,
    result: TestResultType,
    time: Duration,
}

impl TestResultType {
    fn is_correct(&self) -> bool {
        matches!(
            self,
            TestResultType::CorrectTerminated | TestResultType::CorrectNonTerminated { .. }
        )
    }
}

#[derive(Debug, Default)]
struct CompetitionInput {
    sections: BTreeMap<Analysis, BTreeMap<String, Vec<TestResult>>>,
}

impl CompetitionInput {
    fn generate_markdown(&self) -> anyhow::Result<String> {
        use std::fmt::Write;

        let mut buf = String::new();

        for (analysis, groups) in &self.sections {
            let sorted_groups = groups
                .iter()
                .map(|(g, test_results)| {
                    let num_correct = test_results
                        .iter()
                        .filter(|t| t.result.is_correct())
                        .count();
                    let time: Duration = test_results.iter().map(|t| t.time).sum();
                    (Reverse(num_correct), test_results.len(), time, g)
                })
                .sorted();

            writeln!(buf, "## {analysis}")?;

            let mut table = comfy_table::Table::new();
            table
                .load_preset(comfy_table::presets::ASCII_MARKDOWN)
                .set_header(["Rank", "Group", "Result", "Time"]);

            for (rank_0, (Reverse(num_correct), num_tests, time, g)) in sorted_groups.enumerate() {
                table.add_row([
                    format!("{}", rank_0 + 1),
                    g.to_string(),
                    format!("{num_correct}/{num_tests} passed"),
                    format!("{time:?}"),
                ]);
            }

            writeln!(buf, "\n{table}")?;
        }

        Ok(buf)
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

fn setup_shell_in_group(base: &Path, g: &GroupConfig) -> anyhow::Result<Shell> {
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
    config: &Config,
    no_hidden: bool,
    base: &Path,
    g: &GroupConfig,
) -> anyhow::Result<()> {
    let sh = setup_shell_in_group(base, g)?;

    let report = SingleCompetitionInput {
        group_name: g.name.clone(),
        base_seed: config.base_seed,
        samples: config.samples,
    }
    .run_in_docker(&sh)?;

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

#[derive(Debug, Serialize, Deserialize)]
struct IndividualMarkdown {
    group_name: String,
    num_shown: usize,
    sections: Vec<IndividualMarkdownSection>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IndividualMarkdownSection {
    analysis: Analysis,
    programs: Vec<TestResult>,
}

impl std::fmt::Display for IndividualMarkdown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "# {}", self.group_name)?;

        for sec in &self.sections {
            writeln!(f, "## {}", sec.analysis)?;

            let mut table = comfy_table::Table::new();
            table
                .load_preset(comfy_table::presets::ASCII_MARKDOWN)
                .set_header(["Program", "Result", "Time", "Link"]);

            for (idx, summary) in sec.programs.iter().enumerate() {
                table.add_row([
                    format!("Program {}", idx + 1),
                    match &summary.result {
                        TestResultType::CorrectTerminated => "Correct",
                        TestResultType::CorrectNonTerminated { .. } => "Correct<sup>*</sup>",
                        TestResultType::Mismatch { .. } => "Mismatch",
                        TestResultType::TimeOut => "Time out",
                        TestResultType::Error { .. } => "Error",
                    }
                    .to_string(),
                    format!("{:?}", summary.time),
                    if idx < self.num_shown {
                        let mut target = String::new();
                        let mut serializer = url::form_urlencoded::Serializer::new(&mut target);
                        serializer
                            .append_pair("analysis", sec.analysis.command())
                            .append_pair("src", &summary.src)
                            .append_pair("input", &summary.input_json);
                        format!("[Link](http://localhost:3000/?{target})")
                    } else {
                        "Hidden".to_string()
                    },
                ]);
            }
            writeln!(f, "\n{table}")?;
        }

        let mut table = comfy_table::Table::new();
        table
            .load_preset(comfy_table::presets::ASCII_MARKDOWN)
            .set_header(["Result", "Explanation"])
            .add_row(["Correct", "Nice job! :)"])
            .add_row([
                "Correct<sup>*</sup>",
                "The program ran correctly for a limited number of steps",
            ])
            .add_row(["Mismatch", "The result did not match the expected output"])
            .add_row(["Error", "Unable to parse the output"]);
        writeln!(f, "\n## Result explanations")?;
        writeln!(f, "\n{table}")?;

        Ok(())
    }
}
