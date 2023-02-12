//! Responsible for running programs on external implementations.
//!
//! [`TestRunInput`] are a collection of programs which are run in a Docker
//! container using [`TestRunInput::run_in_docker`]. Internally this calls
//! [`TestRunInput::run_from_within_docker`] which compiles and runs all of the
//! programs within the Docker container. The results of the run are written to
//! a file within the container and are read from the outside to produce the
//! final [`TestRunResults`].

use std::{path::Path, time::Duration};

use checkr::{
    driver::Driver,
    env::{
        Analysis, Environment, InterpreterEnv, ProgramVerificationEnv, SecurityEnv, SignEnv,
        ValidationResult,
    },
};
use color_eyre::{
    eyre::{eyre, Context, ContextCompat},
    Result,
};
use itertools::{Either, Itertools};
use serde::{Deserialize, Serialize};
use tracing::info;
use xshell::Shell;

use crate::{config::ProgramsConfig, docker::DockerImage, RunOption};

#[derive(Debug, Serialize, Deserialize)]
pub struct TestRunInput {
    programs: ProgramsConfig,
}

impl TestRunInput {
    const RESULT_FILE: &'static str = "result.json";

    pub async fn run_in_docker(
        image: &DockerImage,
        cwd: &Path,
        programs: ProgramsConfig,
    ) -> Result<TestRunResults> {
        let input = serde_json::to_string(&TestRunInput { programs }).unwrap();

        const SINGLE_COMPETITION_CMD: &str = "internal-single-competition";

        let mut cmd = image.run_cmd(&[
            "-w",
            "/root/code",
            "-v",
            &format!(
                "{}:/root/code",
                cwd.to_str()
                    .wrap_err("failed to create a str from cwd when spawning docker")?
            ),
        ]);
        cmd.args(["checko", SINGLE_COMPETITION_CMD]).arg(input);

        info!("spawning docker container: {}", format_cmd(&cmd));
        let output = cmd
            .output()
            .await
            .wrap_err("Failed to spawn Docker container")?;
        info!(
            status = output.status.to_string(),
            "docker container finished"
        );

        if !output.status.success() {
            tracing::error!(
                stdout = std::str::from_utf8(&output.stdout)?,
                stderr = std::str::from_utf8(&output.stderr)?,
                "running in docker failed"
            );
            return Err(eyre!("Running in Docker failed: {cmd:?}"));
        }

        let output = tokio::fs::read_to_string(cwd.join(Self::RESULT_FILE))
            .await
            .wrap_err("failed to read result file")?;

        Ok(serde_json::from_str(&output)?)
    }
    pub async fn run_from_within_docker(sh: &Shell, input: &str) -> Result<()> {
        let input: Self = serde_json::from_str(input)?;

        let run: RunOption = toml::from_str(&sh.read_file("run.toml")?)?;
        let driver = run.driver(sh.current_dir())?;

        let results = GroupResults::generate(&input.programs, &driver).await?;

        sh.write_file(Self::RESULT_FILE, serde_json::to_string(&results)?)?;

        Ok(())
    }
}

fn format_cmd(cmd: &tokio::process::Command) -> impl std::fmt::Display + '_ {
    let cmd = cmd.as_std();

    let program = Either::Left(Path::new(cmd.get_program()).display());

    let program_args = cmd
        .get_args()
        .map(std::ffi::OsStr::to_string_lossy)
        .map(Either::Right);

    std::iter::once(program).chain(program_args).format(" ")
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestRunResults {
    pub ran_at: std::time::SystemTime,
    pub sections: Vec<TestRunResultsSection>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestRunResultsSection {
    pub analysis: Analysis,
    pub programs: Vec<TestResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TestResultType {
    CorrectTerminated,
    CorrectNonTerminated { iterations: u64 },
    Mismatch { reason: String },
    TimeOut,
    Error { description: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestResult {
    pub analysis: Analysis,
    pub src: String,
    pub input_json: String,
    pub result: TestResultType,
    pub time: Duration,
}

impl TestResultType {
    pub fn is_correct(&self) -> bool {
        matches!(
            self,
            TestResultType::CorrectTerminated | TestResultType::CorrectNonTerminated { .. }
        )
    }
}

struct GroupResults<'a> {
    config: &'a ProgramsConfig,
    driver: &'a Driver,

    sections: Vec<TestRunResultsSection>,
}

impl GroupResults<'_> {
    async fn generate(config: &ProgramsConfig, driver: &Driver) -> Result<TestRunResults> {
        let mut results = GroupResults {
            config,
            driver,
            sections: vec![],
        };

        results.push(&InterpreterEnv).await;
        results.push(&SignEnv).await;
        results.push(&SecurityEnv).await;
        results.push(&ProgramVerificationEnv).await;

        Ok(TestRunResults {
            ran_at: std::time::SystemTime::now(),
            sections: results.sections,
        })
    }
    async fn push<E: Environment>(&mut self, env: &E) {
        self.sections.push(TestRunResultsSection {
            analysis: E::ANALYSIS,
            programs: generate_test_results(self.config, env, self.driver).await,
        });
    }
}

async fn generate_test_results<E: Environment>(
    config: &ProgramsConfig,
    env: &E,
    driver: &Driver,
) -> Vec<TestResult> {
    let mut results = vec![];

    for program in &config.programs {
        let builder = env.setup_generation().seed(Some(program.seed));
        let generated = match program.src.as_ref() {
            Some(src) => builder.from_cmds(checkr::parse::parse_commands(src).unwrap()),
            None => builder.build(),
        };

        let summary = generated.run_analysis(env, driver).await;
        let result = TestResult {
            analysis: E::ANALYSIS,
            src: summary.cmds.to_string(),
            input_json: serde_json::to_string(&summary.input).expect("failed to serialize input"),
            result: match summary.result {
                Ok(r) => match r {
                    ValidationResult::CorrectTerminated => TestResultType::CorrectTerminated,
                    ValidationResult::CorrectNonTerminated { iterations } => {
                        TestResultType::CorrectNonTerminated { iterations }
                    }
                    ValidationResult::Mismatch { reason } => TestResultType::Mismatch { reason },
                    ValidationResult::TimeOut => TestResultType::TimeOut,
                },
                Err(err) => TestResultType::Error {
                    description: err.to_string(),
                },
            },
            time: summary.time,
        };

        results.push(result);
    }

    results
}
