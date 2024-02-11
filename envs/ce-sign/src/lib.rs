#![allow(non_snake_case)]

mod semantics;

use ce_core::{
    define_env,
    rand::{self, seq::SliceRandom},
    Env, EnvError, Generate, ValidationResult,
};
use gcl::{
    ast::{Array, Commands, Target, Variable},
    pg::{
        analysis::{mono_analysis, FiFo},
        Determinism, Node, ProgramGraph,
    },
    stringify::Stringify,
};
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

pub use semantics::{Bools, Sign, SignAnalysis, SignMemory, Signs};

define_env!(SignEnv);

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignInput {
    pub commands: Stringify<Commands>,
    pub determinism: Determinism,
    pub assignment: SignMemory,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignOutput {
    pub initial_node: String,
    pub final_node: String,
    pub nodes: IndexMap<String, IndexSet<SignMemory>>,
}

impl Env for SignEnv {
    type Input = SignInput;

    type Output = SignOutput;

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        let pg = ProgramGraph::new(input.determinism, input.commands.inner());

        for t in pg.fv() {
            match t {
                Target::Variable(var) => {
                    if input.assignment.get_var(&var).is_none() {
                        return Err(EnvError::InvalidInputForProgram {
                            message: format!("variable `{var}` was not in the given input"),
                        });
                    }
                }
                Target::Array(arr, _) => {
                    if input.assignment.get_arr(&arr).is_none() {
                        return Err(EnvError::InvalidInputForProgram {
                            message: format!("array `{arr}` was not in the given input"),
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
        Ok(SignOutput {
            initial_node: Node::Start.to_string(),
            final_node: Node::End.to_string(),
            nodes,
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
            Ok(ValidationResult::CorrectTerminated)
        } else {
            tracing::error!(missing = format!("{pool:?}"), "oh no...");
            Ok(ValidationResult::Mismatch {
                reason: "Reference had world which was not present".to_string(),
            })
        }
    }
}

impl SignInput {
    fn set_sign(&self, var: &Variable, sign: Sign) -> SignInput {
        let mut new = self.clone();
        new.assignment.variables.insert(var.clone(), sign);
        new
    }
    fn set_signs(&self, arr: &Array, signs: Signs) -> SignInput {
        let mut new = self.clone();
        new.assignment.arrays.insert(arr.clone(), signs);
        new
    }
}

impl SignInput {
    fn gen_from_commands<R: rand::Rng>(rng: &mut R, commands: Commands) -> SignInput {
        let assignment = SignMemory::from_targets_with(
            commands.fv(),
            rng,
            |rng, _| Generate::gen(&mut (), rng),
            |rng, _| Generate::gen(&mut (), rng),
        );

        SignInput {
            commands: Stringify::new(commands),
            assignment,
            determinism: Determinism::Deterministic,
        }
    }
}

impl Generate for SignInput {
    type Context = ();

    fn gen<R: rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        let commands = Commands::gen(&mut Default::default(), rng);
        SignInput::gen_from_commands(rng, commands)
    }
}

impl Generate for Sign {
    type Context = ();

    fn gen<R: rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        *[Sign::Positive, Sign::Zero, Sign::Negative]
            .choose(rng)
            .unwrap()
    }
}
impl Generate for Signs {
    type Context = ();

    fn gen<R: rand::Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        [Sign::gen(cx, rng)].into_iter().collect()
    }
}
