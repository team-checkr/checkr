mod dot;

use std::collections::BTreeMap;

use ce_core::{define_env, Env, Generate, ValidationResult};
use gcl::{
    ast::Commands,
    pg::{Determinism, ProgramGraph},
    stringify::Stringify,
};
use serde::{Deserialize, Serialize};

define_env!(CompilerEnv);

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "Compiler")]
pub struct Input {
    pub commands: Stringify<Commands>,
    pub determinism: Determinism,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "Compiler")]
pub struct Output {
    pub dot: String,
}

impl Env for CompilerEnv {
    type Input = Input;

    type Output = Output;

    type Meta = ();

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        let dot =
            ProgramGraph::new(
                input.determinism,
                &input.commands.try_parse().map_err(
                    ce_core::EnvError::invalid_input_for_program("failed to parse commands"),
                )?,
            )
            .dot();
        Ok(Output { dot })
    }

    fn validate(input: &Self::Input, output: &Self::Output) -> ce_core::Result<ValidationResult> {
        let t_g = match dot::dot_to_petgraph(&output.dot) {
            Ok(t_g) => t_g,
            Err(err) => {
                return Ok(ValidationResult::Mismatch {
                    reason: format!("failed to parse dot: {err}"),
                })
            }
        };
        let o_g =
            dot::dot_to_petgraph(&Self::run(input)?.dot).expect("we always produce valid dot");

        if degree_bag(&o_g) != degree_bag(&t_g) {
            Ok(ValidationResult::Mismatch {
                reason: "invalid node degree bag".to_string(),
            })
        } else {
            Ok(ValidationResult::CorrectTerminated)
        }
    }
}

impl Generate for Input {
    type Context = ();

    fn gen<R: ce_core::rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        Input {
            commands: Stringify::new(Commands::gen(&mut Default::default(), rng)),
            determinism: Determinism::NonDeterministic,
        }
    }
}

fn degree_bag(g: &dot::ParsedGraph) -> BTreeMap<(usize, usize), usize> {
    let mut counts = BTreeMap::new();

    for i in g.graph.node_indices() {
        let out_degree = g.graph.neighbors_directed(i, petgraph::Outgoing).count();
        let in_degree = g.graph.neighbors_directed(i, petgraph::Incoming).count();
        let count = counts.entry((out_degree, in_degree)).or_insert(0);
        *count += 1;
    }

    counts
}
