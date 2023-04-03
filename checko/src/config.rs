//! Config definitions for program inputs and groups of group.

use checkr::{env::Analysis, GeneratedProgram};
use color_eyre::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GroupsConfig {
    pub groups: Vec<GroupConfig>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProgramsConfig {
    #[serde(default)]
    pub envs: IndexMap<Analysis, ProgramsEnvConfig>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProgramsEnvConfig {
    pub programs: Vec<ProgramConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramConfig {
    pub seed: Option<u64>,
    pub src: Option<String>,
    pub input: Option<String>,
    #[serde(default)]
    pub shown: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CanonicalProgramsConfig {
    #[serde(default)]
    pub envs: IndexMap<Analysis, CanonicalProgramsEnvConfig>,
}
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ProgramId(usize);
impl CanonicalProgramsConfig {
    pub(crate) fn get(&self, analysis: Analysis, input: ProgramId) -> &CanonicalProgramConfig {
        &self.envs[&analysis].programs[input.0]
    }
}

impl CanonicalProgramConfig {
    pub fn generated_program(&self, analysis: Analysis) -> Result<GeneratedProgram> {
        let builder = analysis.setup_generation();
        Ok(builder.from_cmds_and_input(
            checkr::parse::parse_commands(&self.src).unwrap(),
            analysis.input_from_str(&self.input)?,
        ))
    }
}
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CanonicalProgramsEnvConfig {
    pub programs: Vec<CanonicalProgramConfig>,
}
impl CanonicalProgramsEnvConfig {
    pub(crate) fn programs(&self) -> impl Iterator<Item = (ProgramId, &CanonicalProgramConfig)> {
        self.programs
            .iter()
            .enumerate()
            .map(|(idx, p)| (ProgramId(idx), p))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalProgramConfig {
    pub src: String,
    pub input: String,
    pub shown: bool,
}
impl ProgramsConfig {
    pub fn extend(&mut self, other: Self) {
        for (analysis, env) in other.envs {
            self.envs
                .entry(analysis)
                .or_default()
                .programs
                .extend_from_slice(&env.programs);
        }
    }
    pub fn canonicalize(&self) -> Result<CanonicalProgramsConfig> {
        let envs = self
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

        Ok(CanonicalProgramsConfig { envs })
    }
}
impl ProgramConfig {
    fn generated_program(&self, analysis: Analysis) -> Result<GeneratedProgram> {
        Ok(match self {
            ProgramConfig {
                seed: Some(seed),
                src: None,
                input: None,
                ..
            } => analysis.setup_generation().seed(Some(*seed)).build(),
            ProgramConfig {
                seed: Some(seed),
                src: Some(src),
                input: None,
                ..
            } => {
                let builder = analysis.setup_generation().seed(Some(*seed));
                builder.from_cmds(checkr::parse::parse_commands(src).unwrap())
            }
            ProgramConfig {
                src: Some(src),
                input: Some(input),
                ..
            } => {
                let builder = analysis.setup_generation();
                builder.from_cmds_and_input(
                    checkr::parse::parse_commands(src).unwrap(),
                    analysis.input_from_str(input)?,
                )
            }
            _ => todo!(),
        })
    }
    pub fn canonicalize(&self, analysis: Analysis) -> Result<CanonicalProgramConfig> {
        let p = self.generated_program(analysis)?;

        Ok(CanonicalProgramConfig {
            src: p.cmds.to_string(),
            input: p.input.to_string(),
            shown: self.shown,
        })
    }
}

#[derive(Debug, Default, Clone, Hash, Serialize, Deserialize)]
pub struct GroupConfig {
    pub name: String,
    pub git: String,
}
