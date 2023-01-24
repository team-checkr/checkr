use std::{
    cmp::Reverse,
    collections::BTreeMap,
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};

use axum::{http::StatusCode, routing::post, Json, Router};
use clap::Parser;
use infra::RunOption;
use itertools::Itertools;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tracing::{debug, error};
use verification_lawyer::{
    driver::Driver,
    env::{
        graph::GraphEnv, pv::ProgramVerificationEnv, Environment, SecurityEnv, SignEnv,
        StepWiseEnv, ToMarkdown, ValidationResult,
    },
    generation::Generate,
    AnalysisSummary,
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
    Server {
        #[clap(long, short, default_value = "25565")]
        port: u16,
    },
    GenerateReport {
        dir: PathBuf,
        #[clap(long, short)]
        group_nr: u64,
        #[clap(long, short, default_value = "false")]
        pull: bool,
        #[clap(long, default_value = "false")]
        no_hidden: bool,
        #[clap(long, short)]
        output: PathBuf,
    },
    GenerateCompetition {
        #[clap(short, default_value = "false")]
        no_hidden: bool,
        #[clap(long, short)]
        base: PathBuf,
        config: PathBuf,
        #[clap(long, short)]
        output: PathBuf,
    },
    SingleCompetition {
        input: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    base_seed: u64,
    samples: u64,
    groups: Vec<GroupConfig>,
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
    base_seed: u64,
    samples: u64,
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
                    error!(group = g.name, error = format!("{:?}", e), "Group errored")
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
            axum::Server::bind(&SocketAddr::from(([0; 4], port)))
                .serve(app.into_make_service())
                .await?;
            Ok(())
        }
        Cli::GenerateReport {
            dir,
            group_nr,
            pull,
            no_hidden,
            output,
        } => {
            let sh = Shell::new()?;
            sh.change_dir(dir);

            if pull {
                cmd!(sh, "git checkout master").run()?;
                cmd!(sh, "git pull").run()?;
            }

            let result = generate_report(
                &Config {
                    base_seed: DEFAULT_BASE_SEED,
                    samples: DEFAULT_SAMPLES,
                    groups: vec![],
                },
                &sh,
                no_hidden,
                group_nr,
            )?;
            fs::write(output, result)?;
            Ok(())
        }
        Cli::GenerateCompetition {
            no_hidden,
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

                    let cwd = sh.current_dir();

                    let input = SingleCompetitionInput {
                        base_seed: config.base_seed,
                        samples: config.samples,
                    };
                    let input = serde_json::to_string(&input).unwrap();

                    let cmd = ["vl-infra", "infra", "single-competition"];

                    cmd!(
                        sh,
                        "docker run -w /root/code --rm -v {cwd}:/root/code {cmd...} {input}"
                    )
                    .run()
                    .unwrap();

                    let output = sh.read_file("result.json").unwrap();

                    Some((g, serde_json::from_str(&output).unwrap()))
                })
                .collect::<Vec<(&GroupConfig, Vec<_>)>>();
            for (g, categories) in results {
                for (cat, results) in categories {
                    input
                        .categories
                        .entry(cat)
                        .or_default()
                        .entry(g.name.clone())
                        .or_insert(results);
                }
            }

            let result = input.generate_markdown()?;
            fs::write(output, result)?;

            Ok(())
        }
        Cli::SingleCompetition { input } => {
            let sh = Shell::new()?;

            let input: SingleCompetitionInput = serde_json::from_str(&input)?;

            let results = GroupResults::generate(
                &Config {
                    base_seed: input.base_seed,
                    samples: input.samples,
                    groups: vec![],
                },
                &sh,
            )?;

            sh.write_file("result.json", serde_json::to_string(&results)?)?;

            Ok(())
        }
    }
}

struct GroupResults<'a> {
    config: &'a Config,
    driver: &'a Driver,

    categories: Vec<(String, Vec<TestResult>)>,
}

impl GroupResults<'_> {
    fn generate(config: &Config, sh: &Shell) -> anyhow::Result<Vec<(String, Vec<TestResult>)>> {
        let run: RunOption = toml::from_str(&sh.read_file("run.toml")?)?;
        let driver = run.driver(sh.current_dir())?;

        let mut results = GroupResults {
            config,
            driver: &driver,
            categories: vec![],
        };

        results
            .push(&StepWiseEnv)
            .push(&SignEnv)
            .push(&SecurityEnv)
            .push(&ProgramVerificationEnv);
        // .push(&GraphEnv);

        Ok(results.categories)
    }
    fn push<E: Environment>(&mut self, env: &E) -> &mut Self
    where
        E::Input: ToMarkdown,
        E::Output: ToMarkdown,
    {
        self.categories.push((
            env.name(),
            generate_test_results(self.config, env, self.driver),
        ));
        self
    }
}

fn generate_report(
    config: &Config,
    sh: &Shell,
    no_hidden: bool,
    group_nr: impl std::fmt::Display,
) -> anyhow::Result<String> {
    let categories = GroupResults::generate(config, sh)?;

    use std::fmt::Write;

    let mut output = String::new();
    writeln!(output, "# Group {group_nr}")?;

    for (category, summaries) in categories {
        generate_markdown(no_hidden, &category, &mut output, &summaries)?;
    }

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

fn generate_test_results<E: Environment>(
    config: &Config,
    env: &E,
    driver: &Driver,
) -> Vec<TestResult>
where
    E::Input: ToMarkdown,
    E::Output: ToMarkdown,
{
    (0..config.samples)
        .map(|idx| {
            let summary =
                verification_lawyer::run_analysis(env, None, Some(config.base_seed + idx), driver);
            TestResult {
                analysis: E::command().to_string(),
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
    analysis: String,
    src: String,
    input_json: String,
    result: TestResultType,
    time: Duration,
}

fn generate_markdown(
    no_hidden: bool,
    name: &str,
    mut f: impl std::fmt::Write,
    summaries: &[TestResult],
) -> anyhow::Result<()> {
    const NUM_VISIBLE: usize = 2;

    writeln!(f, "## {name}")?;

    let mut table = comfy_table::Table::new();
    table
        .load_preset(comfy_table::presets::ASCII_MARKDOWN)
        .set_header(["Program", "Result", "Time", "Link"]);

    for (idx, summary) in summaries.iter().enumerate() {
        let mut target = String::new();
        let mut serializer = url::form_urlencoded::Serializer::new(&mut target);
        serializer
            .append_pair("analysis", &summary.analysis)
            .append_pair("src", &summary.src)
            .append_pair("input", &summary.input_json);

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
            if no_hidden || idx < NUM_VISIBLE {
                format!("[Link](http://localhost:3000/?{target})")
            } else {
                "Hidden".to_string()
            },
        ]);
    }
    writeln!(f, "\n{table}")?;

    Ok(())
}

#[derive(Debug, Default)]
struct CompetitionInput {
    categories: BTreeMap<String, BTreeMap<String, Vec<TestResult>>>,
}

impl CompetitionInput {
    fn generate_markdown(&self) -> anyhow::Result<String> {
        use std::fmt::Write;

        let mut buf = String::new();

        for (cat, groups) in &self.categories {
            let sorted_groups = groups
                .iter()
                .map(|(g, test_results)| {
                    let num_correct = test_results
                        .iter()
                        .filter(|t| match t.result {
                            TestResultType::CorrectTerminated
                            | TestResultType::CorrectNonTerminated { .. } => true,
                            TestResultType::Mismatch { .. }
                            | TestResultType::TimeOut
                            | TestResultType::Error { .. } => false,
                        })
                        .count();
                    let time: Duration = test_results.iter().map(|t| t.time).sum();
                    (Reverse(num_correct), test_results.len(), time, g)
                })
                .sorted();

            writeln!(buf, "## {cat}")?;

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

    let result = cmd!(sh, "git symbolic-ref refs/remotes/origin/HEAD").read()?;
    let primary_branch = result.split('/').last().expect("no primary branch");

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

    let report = generate_report(config, &sh, no_hidden, &g.name)?;

    if cmd!(sh, "git checkout results").run().is_err() {
        cmd!(sh, "git switch --orphan results").run()?;
    }
    cmd!(sh, "git reset --hard").run()?;
    cmd!(sh, "git clean -xdf").run()?;
    sh.write_file("README.md", report)?;
    cmd!(sh, "git add .").run()?;
    let msg = format!("Ran tests at {:?}", std::time::Instant::now());
    // cmd!(sh, "git commit -m {msg}").run()?;
    // cmd!(sh, "git push --force --set-upstream origin results").run()?;

    Ok(())
}
