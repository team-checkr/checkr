//! Responsible for running programs on external implementations.
//!
//! [`TestRunInput`] are a collection of programs which are run in a Docker
//! container using [`TestRunInput::run_in_docker`]. Internally this calls
//! [`TestRunInput::run_from_within_docker`] which compiles and runs all of the
//! programs within the Docker container. The results of the run are written to
//! a file within the container and are read from the outside to produce the
//! final [`TestRunResults`].

use std::{
    path::Path,
    time::{Duration, SystemTime},
};

use checkr::{
    driver::Driver,
    env::{self, Analysis, AnyEnvironment, Environment, ValidationResult},
};
use color_eyre::{
    eyre::{eyre, Context, ContextCompat},
    Result,
};
use serde::{Deserialize, Serialize};
use tracing::info;
use xshell::Shell;

use crate::{config::ProgramsConfig, docker::DockerImage, RunOption};

#[derive(Debug, Serialize, Deserialize)]
pub struct TestRunInput {
    programs: ProgramsConfig,
}

impl TestRunInput {
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

        let start = std::time::Instant::now();
        info!(container_name = image.name(), "spawning docker container");
        let output = cmd
            .output()
            .await
            .wrap_err("Failed to spawn Docker container")?;
        info!(
            status = output.status.to_string(),
            duration = format!("{:?}", start.elapsed()),
            "docker container finished"
        );

        if !output.status.success() {
            tracing::error!(
                stdout = std::str::from_utf8(&output.stdout)?,
                "running in docker failed"
            );
            eprintln!("{}", std::str::from_utf8(&output.stderr)?);
            return Err(eyre!("Running in Docker failed"));
        }

        let output = String::from_utf8(output.stdout)?;

        Ok(serde_json::from_str(&output)?)
    }
    pub async fn run_from_within_docker(sh: &Shell, input: &str) -> Result<()> {
        let input: Self = serde_json::from_str(input)?;

        let run: RunOption = toml::from_str(&sh.read_file("run.toml")?)?;
        let results = match run.driver(sh.current_dir()) {
            Ok(driver) => GroupResults::generate(&input.programs, &driver).await?,
            Err(err) => {
                let msg = match err {
                    checkr::driver::DriverError::RunCompile(output) => format!(
                        "running '{}' failed:\n  {output}",
                        run.compile.as_deref().unwrap_or("compiler"),
                    ),
                    checkr::driver::DriverError::CompileFailure(output) => format!(
                        "failed to compile:\n  {}\n\n  {}",
                        std::str::from_utf8(&output.stdout).unwrap(),
                        std::str::from_utf8(&output.stderr).unwrap()
                    ),
                };
                TestRunResults {
                    ran_at: SystemTime::now(),
                    data: TestRunData::CompileError(msg),
                }
            }
        };

        println!("{}", serde_json::to_string(&results)?);

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestRunResults {
    pub ran_at: SystemTime,
    pub data: TestRunData,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TestRunData {
    CompileError(String),
    Sections(Vec<TestRunResultsSection>),
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
    pub shown: bool,
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

        for key in config.envs.keys() {
            match key {
                // NOTE: Skip graph
                Analysis::Graph => {}
                Analysis::Parse => results.push(&env::ParseEnv).await,
                Analysis::Interpreter => results.push(&env::InterpreterEnv).await,
                Analysis::ProgramVerification => results.push(&env::ProgramVerificationEnv).await,
                Analysis::Sign => results.push(&env::SignEnv).await,
                Analysis::Security => results.push(&env::SecurityEnv).await,
            }
        }

        Ok(TestRunResults {
            ran_at: SystemTime::now(),
            data: TestRunData::Sections(results.sections),
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

    let Some(programs) = config.envs.get(&E::ANALYSIS) else { return vec![] };

    for program in &programs.programs {
        let generated = program.generated_program(env.analysis()).unwrap();
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
            shown: program.shown,
        };

        results.push(result);
    }

    results
}
