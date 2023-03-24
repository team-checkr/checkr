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
    pub envs: IndexMap<Analysis, CanonicalProgramsEnvConfig>,
}
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CanonicalProgramsEnvConfig {
    pub programs: Vec<CanonicalProgramConfig>,
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
}
impl ProgramConfig {
    pub fn generated_program(&self, analysis: Analysis) -> Result<GeneratedProgram> {
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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GroupConfig {
    pub name: String,
    pub git: String,
}
