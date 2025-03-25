//! Config definitions for program inputs and groups of group.

use std::{collections::BTreeMap, fs, path::Path, sync::Arc};

use ce_shell::{Analysis, Input};
use color_eyre::{Result, eyre::Context};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

#[derive(tapi::Tapi, Debug, Default, Clone, Serialize, Deserialize)]
pub struct GroupsConfig {
    #[serde(default)]
    pub ignored_authors: Vec<String>,
    pub groups: Vec<Arc<GroupConfig>>,
}

#[derive(tapi::Tapi, Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProgramsConfig {
    #[serde(default)]
    pub deadlines: IndexMap<Analysis, ProgramsDeadline>,
    #[serde(default)]
    pub envs: IndexMap<Analysis, ProgramsEnvConfig>,
}

#[derive(tapi::Tapi, Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProgramsDeadline {
    pub time: Option<chrono::DateTime<chrono::FixedOffset>>,
}

#[derive(tapi::Tapi, Debug, Default, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProgramsEnvConfig {
    pub programs: Vec<ProgramConfig>,
}

#[derive(tapi::Tapi, Debug, Clone, Serialize, Deserialize)]
pub struct ProgramConfig {
    pub seed: Option<u64>,
    pub input: Option<String>,
    #[serde(default)]
    pub shown: bool,
}

#[derive(tapi::Tapi, Debug, Default, Clone, Serialize, Deserialize)]
pub struct CanonicalProgramsConfig {
    #[serde(default)]
    pub envs: IndexMap<Analysis, CanonicalProgramsEnvConfig>,
}
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ProgramId(usize);
#[derive(tapi::Tapi, Debug, Default, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CanonicalProgramsEnvConfig {
    pub programs: Vec<CanonicalProgramConfig>,
}
#[derive(tapi::Tapi, Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalProgramConfig {
    pub input: String,
    pub shown: bool,
}
impl ProgramsConfig {
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

    pub(crate) fn inputs(
        &self,
    ) -> impl Iterator<Item = (Analysis, impl Iterator<Item = Input> + '_)> + '_ {
        self.envs.iter().map(|(&analysis, p)| {
            (
                analysis,
                p.programs.iter().map(move |p| {
                    let c = p.canonicalize(analysis).unwrap();
                    analysis.input_from_str(&c.input).unwrap()
                }),
            )
        })
    }
}
impl ProgramConfig {
    fn canonicalize(&self, analysis: Analysis) -> Result<CanonicalProgramConfig> {
        Ok(match self {
            ProgramConfig {
                seed: Some(seed),
                input: None,
                ..
            } => CanonicalProgramConfig {
                input: analysis.gen_input_seeded(Some(*seed)).to_string(),
                shown: self.shown,
            },
            ProgramConfig {
                input: Some(input), ..
            } => CanonicalProgramConfig {
                input: input.to_string(),
                shown: self.shown,
            },
            pc => todo!("{pc:?}"),
        })
    }
    // pub fn canonicalize(&self, analysis: Analysis) -> Result<CanonicalProgramConfig> {
    //     let p = self.generated_program(analysis)?;

    //     Ok(CanonicalProgramConfig {
    //         src: p.cmds.to_string(),
    //         input: p.input.to_string(),
    //         shown: self.shown,
    //     })
    // }
}

#[derive(
    tapi::Tapi, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct GroupName(SmolStr);

impl std::fmt::Debug for GroupName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl GroupName {
    pub fn as_str(&self) -> &str {
        self
    }
}

impl std::fmt::Display for GroupName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Deref for GroupName {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(tapi::Tapi, Debug, Default, Clone, Hash, Serialize, Deserialize)]
pub struct GroupConfig {
    pub name: GroupName,
    pub git: Option<SmolStr>,
    pub path: Option<SmolStr>,
    pub run: Option<SmolStr>,
    #[serde(default)]
    pub commit: BTreeMap<Analysis, SmolStr>,
}

pub fn read_programs(programs: impl AsRef<Path>) -> Result<ProgramsConfig> {
    let p = programs.as_ref();
    let src =
        fs::read_to_string(p).wrap_err_with(|| format!("could not read programs at {p:?}"))?;
    let parsed =
        toml::from_str(&src).wrap_err_with(|| format!("error parsing programs from file {p:?}"))?;
    Ok(parsed)
}
pub fn read_groups(groups: impl AsRef<Path>) -> Result<GroupsConfig> {
    let p = groups.as_ref();
    let src = fs::read_to_string(p).wrap_err_with(|| format!("could not read groups at {p:?}"))?;
    let parsed =
        toml::from_str(&src).wrap_err_with(|| format!("error parsing groups from file {p:?}"))?;
    Ok(parsed)
}
