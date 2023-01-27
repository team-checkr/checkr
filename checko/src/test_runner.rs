//! Responsible for running programs on external implementations.
//!
//! [`TestRunInput`] are a collection of programs which are run in a Docker
//! container using [`TestRunInput::run_in_docker`]. Internally this calls
//! [`TestRunInput::run_from_within_docker`] which compiles and runs all of the
//! programs within the Docker container. The results of the run are written to
//! a file within the container and are read from the outside to produce the
//! final [`TestRunResults`].

use std::time::Duration;

use checkr::{
    driver::Driver,
    env::{
        Analysis, Environment, InterpreterEnv, ProgramVerificationEnv, SecurityEnv, SignEnv,
        ValidationResult,
    },
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use xshell::{cmd, Shell};

use crate::{config::ProgramsConfig, RunOption};

#[derive(Debug, Serialize, Deserialize)]
pub struct TestRunInput {
    programs: ProgramsConfig,
}

impl TestRunInput {
    const RESULT_FILE: &'static str = "result.json";

    pub fn run_in_docker(sh: &Shell, programs: ProgramsConfig) -> anyhow::Result<TestRunResults> {
        let cwd = sh.current_dir();

        let input = serde_json::to_string(&TestRunInput { programs }).unwrap();

        // TODO: Don't duplicate the image name
        const DOCKER_IMAGE_NAME: &str =
            "gitlab.gbar.dtu.dk/checkr-dev-env/demo-group-01/image:latest";
        const DOCKER_BINARY_NAME: &str = "checko";
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
        .run()?;

        let output = sh.read_file(Self::RESULT_FILE).unwrap();

        Ok(serde_json::from_str(&output)?)
    }
    pub fn run_from_within_docker(sh: &Shell, input: &str) -> anyhow::Result<()> {
        let input: Self = serde_json::from_str(input)?;

        let run: RunOption = toml::from_str(&sh.read_file("run.toml")?)?;
        let driver = run.driver(sh.current_dir())?;

        let results = GroupResults::generate(&input.programs, &driver)?;

        sh.write_file(Self::RESULT_FILE, serde_json::to_string(&results)?)?;

        Ok(())
    }
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
    fn generate(config: &ProgramsConfig, driver: &Driver) -> anyhow::Result<TestRunResults> {
        let mut results = GroupResults {
            config,
            driver,
            sections: vec![],
        };

        results
            .push(&InterpreterEnv)
            .push(&SignEnv)
            .push(&SecurityEnv)
            .push(&ProgramVerificationEnv);

        Ok(TestRunResults {
            ran_at: std::time::SystemTime::now(),
            sections: results.sections,
        })
    }
    fn push<E: Environment>(&mut self, env: &E) -> &mut Self {
        self.sections.push(TestRunResultsSection {
            analysis: E::ANALYSIS,
            programs: generate_test_results(self.config, env, self.driver),
        });
        self
    }
}

fn generate_test_results<E: Environment>(
    config: &ProgramsConfig,
    env: &E,
    driver: &Driver,
) -> Vec<TestResult> {
    config
        .programs
        .iter()
        .map(|program| {
            let builder = env.setup_generation().seed(Some(program.seed));
            let generated = match program.src.as_ref() {
                Some(src) => builder.from_cmds(checkr::parse::parse_commands(src).unwrap()),
                None => builder.build(),
            };

            let summary = generated.run_analysis(env, driver);
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
