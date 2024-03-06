mod dot;

use std::collections::{BTreeMap, BTreeSet};

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

        if action_bag(&o_g) != action_bag(&t_g) {
            Ok(ValidationResult::Mismatch {
                reason: "the graphs have different structure".to_string(),
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

fn action_bag(
    g: &dot::ParsedGraph,
) -> BTreeMap<(BTreeSet<ActionKind>, BTreeSet<ActionKind>), usize> {
    let mut counts = BTreeMap::new();

    for i in g.graph.node_indices() {
        let outgoing = g
            .graph
            .edges_directed(i, petgraph::Outgoing)
            .map(|e| ActionKind::from(e.weight()))
            .collect::<BTreeSet<_>>();
        let ingoing = g
            .graph
            .edges_directed(i, petgraph::Incoming)
            .map(|e| ActionKind::from(e.weight()))
            .collect::<BTreeSet<_>>();
        let count = counts.entry((outgoing, ingoing)).or_insert(0);
        *count += 1;
    }

    counts
}

impl From<&'_ gcl::pg::Action> for ActionKind {
    fn from(action: &'_ gcl::pg::Action) -> Self {
        match action {
            gcl::pg::Action::Assignment(t, _) => ActionKind::Assignment(t.clone().map_idx(|_| ())),
            gcl::pg::Action::Skip => ActionKind::Skip,
            gcl::pg::Action::Condition(_) => ActionKind::Condition,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum ActionKind {
    Assignment(gcl::ast::Target<()>),
    Skip,
    Condition,
}
