#![allow(non_snake_case)]

mod semantics;

use std::collections::BTreeSet;

use ce_core::{
    Env, EnvError, Generate, ValidationResult, define_env,
    rand::{self, seq::IndexedRandom},
};
use gcl::{
    ast::{Commands, Target, TargetDef},
    memory::Memory,
    pg::{
        Determinism, Node, ProgramGraph,
        analysis::{FiFo, mono_analysis},
    },
};
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
pub use semantics::{Bools, Sign, SignAnalysis, SignMemory, Signs};
use serde::{Deserialize, Serialize};
use stdx::stringify::Stringify;

define_env!(SignEnv);

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "SignAnalysis")]
pub struct Input {
    pub commands: Stringify<Commands>,
    pub determinism: Determinism,
    pub assignment: SignMemory,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[tapi(path = "SignAnalysis")]
pub struct Output {
    pub initial_node: String,
    pub final_node: String,
    pub nodes: IndexMap<String, IndexSet<SignMemory>>,
    pub dot: String,
}

impl Env for SignEnv {
    type Input = Input;

    type Output = Output;

    type Meta = BTreeSet<TargetDef>;

    fn meta(input: &Self::Input) -> Self::Meta {
        if let Ok(commands) = input.commands.try_parse() {
            commands.fv().into_iter().map(|t| t.def()).collect()
        } else {
            Default::default()
        }
    }

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        let pg =
            ProgramGraph::new(
                input.determinism,
                &input.commands.try_parse().map_err(
                    ce_core::EnvError::invalid_input_for_program("failed to parse commands"),
                )?,
            );

        for t in pg.fv() {
            match t {
                Target::Variable(var) => {
                    if input.assignment.get_var(&var).is_none() {
                        return Err(EnvError::InvalidInputForProgram {
                            message: format!("variable `{var}` was not in the given input"),
                            source: None,
                        });
                    }
                }
                Target::Array(arr, _) => {
                    if input.assignment.get_arr(&arr).is_none() {
                        return Err(EnvError::InvalidInputForProgram {
                            message: format!("array `{arr}` was not in the given input"),
                            source: None,
                        });
                    }
                }
            }
        }

        let nodes = mono_analysis::<_, FiFo>(
            SignAnalysis {
                assignment: input.assignment.clone(),
            },
            &pg,
        )
        .facts
        .into_iter()
        .map(|(k, v)| (format!("{k}"), v))
        .collect();
        Ok(Output {
            initial_node: Node::Start.to_string(),
            final_node: Node::End.to_string(),
            nodes,
            dot: pg.dot(),
        })
    }

    fn validate(
        input: &Self::Input,
        output: &Self::Output,
    ) -> ce_core::Result<ce_core::ValidationResult> {
        let reference = Self::run(input)?;

        let mut pool = reference.nodes.values().collect_vec();

        for (n, o) in &output.nodes {
            if let Some(idx) = pool.iter().position(|r| *r == o) {
                pool.remove(idx);
            } else {
                tracing::error!(not_in_reference = format!("{o:?}"), "damn...");
                return Ok(ValidationResult::Mismatch {
                    reason: format!(
                        "Produced world which did not exist in reference: {n:?} ~> {o:?}"
                    ),
                });
            }
        }

        if pool.is_empty() {
            Ok(ValidationResult::Correct)
        } else {
            tracing::error!(missing = format!("{pool:?}"), "oh no...");
            Ok(ValidationResult::Mismatch {
                reason: "Reference had world which was not present".to_string(),
            })
        }
    }
}

impl Generate for Input {
    type Context = ();

    fn gn<R: rand::Rng>(_cx: &mut Self::Context, mut rng: &mut R) -> Self {
        let commands = Commands::gn(&mut Default::default(), rng);
        let assignment: SignMemory = Memory::from_targets_with(
            commands.fv(),
            &mut rng,
            |rng, _| Generate::gn(&mut (), rng),
            |rng, _| Generate::gn(&mut (), rng),
        )
        .into();

        let determinism = *[Determinism::Deterministic, Determinism::NonDeterministic]
            .choose(rng)
            .unwrap();

        Input {
            commands: Stringify::new(commands),
            assignment,
            determinism,
        }
    }
}

impl Generate for Sign {
    type Context = ();

    fn gn<R: rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        *[Sign::Positive, Sign::Zero, Sign::Negative]
            .choose(rng)
            .unwrap()
    }
}
impl Generate for Signs {
    type Context = ();

    fn gn<R: rand::Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        [Sign::gn(cx, rng)].into_iter().collect()
    }
}
