mod dot;

use std::collections::{BTreeMap, BTreeSet};

use ce_core::gn::compiler_gen::{CompilerContext, gen_commands, generate_witness_memories};
use ce_core::{Env, Generate, ValidationResult, define_env};
use gcl::{
    ast::Commands,
    interpreter::InterpreterMemory,
    pg::{Determinism, ProgramGraph},
};
use itertools::Itertools;
use petgraph::visit::EdgeRef;
use rand::{Rng, seq::IndexedRandom};
use serde::{Deserialize, Serialize};
use stdx::stringify::Stringify;

define_env!(CompilerEnv);

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "Compiler")]
pub struct Input {
    pub commands: Stringify<Commands>,
    pub determinism: Determinism,
    #[serde(default)]
    pub witness_mems: Vec<InterpreterMemory>,
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

    type Annotation = ();

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

    fn validate(
        input: &Self::Input,
        output: &Self::Output,
    ) -> ce_core::Result<(ValidationResult, ())> {
        let commands =
            input
                .commands
                .try_parse()
                .map_err(ce_core::EnvError::invalid_input_for_program(
                    "failed to parse commands",
                ))?;
        let o_dot = ProgramGraph::new(input.determinism, &commands).dot();

        let sample_mems = if input.witness_mems.is_empty() {
            let mut rng = <rand::rngs::SmallRng as rand::SeedableRng>::seed_from_u64(0xCEC34);
            (0..10)
                .map(|_| {
                    let initial_memory = gcl::memory::Memory::from_targets_with(
                        commands.fv(),
                        &mut rng,
                        |rng, _| rng.random_range(-10..=10),
                        |rng, _| {
                            let len = rng.random_range(5..=10);
                            (0..len).map(|_| rng.random_range(-10..=10)).collect()
                        },
                    );
                    InterpreterMemory {
                        variables: initial_memory.variables,
                        arrays: initial_memory.arrays,
                    }
                })
                .collect_vec()
        } else {
            input.witness_mems.clone()
        };

        let t_g = match dot::dot_to_petgraph(&output.dot) {
            Ok(t_g) => t_g,
            Err(err) => {
                return Ok((
                    ValidationResult::Mismatch {
                        reason: format!("failed to parse dot: {err}"),
                    },
                    (),
                ));
            }
        };
        let o_g = dot::dot_to_petgraph(&o_dot).expect("we always produce valid dot");

        if action_bag(&o_g, &sample_mems) != action_bag(&t_g, &sample_mems) {
            Ok((
                ValidationResult::Mismatch {
                    reason: "the graphs have different structure".to_string(),
                },
                (),
            ))
        } else {
            Ok((ValidationResult::Correct, ()))
        }
    }
}

impl Generate for Input {
    type Context = ();

    fn gn<R: ce_core::rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        let determinism = *[Determinism::Deterministic, Determinism::NonDeterministic]
            .choose(rng)
            .unwrap();
        let commands = gen_commands(&mut CompilerContext::default(), rng);
        let mut w_rng = <rand::rngs::SmallRng as rand::SeedableRng>::seed_from_u64(0xCEC34);
        let witness_mems = generate_witness_memories(&commands, &mut w_rng);
        Input {
            commands: Stringify::new(commands),
            determinism,
            witness_mems,
        }
    }
}

fn action_bag(
    g: &dot::ParsedGraph,
    mems: &[InterpreterMemory],
) -> BTreeMap<[BTreeSet<Fingerprint>; 2], usize> {
    let mut counts = BTreeMap::new();

    for i in g.graph.node_indices() {
        let id = [petgraph::Incoming, petgraph::Outgoing].map(|dir| {
            g.graph
                .edges_directed(i, dir)
                .map(|e| fingerprint(e.weight(), mems))
                .collect()
        });
        *counts.entry(id).or_insert(0) += 1;
    }

    counts
}

const MAX_PATHS: usize = 512;

fn path_fingerprints(g: &dot::ParsedGraph) -> Option<BTreeSet<Vec<String>>> {
    let start = g.node_mapping.get("qStart")?;
    let mut all_paths = BTreeSet::new();
    let mut stack: Vec<(
        petgraph::graph::NodeIndex,
        Vec<String>,
        BTreeSet<petgraph::graph::NodeIndex>,
    )> = vec![(*start, vec![], BTreeSet::new())];
    while let Some((node, path, visited)) = stack.pop() {
        if all_paths.len() >= MAX_PATHS {
            return None;
        }
        let outgoing: Vec<_> = g.graph.edges(node).collect();
        if outgoing.is_empty() {
            all_paths.insert(path);
        } else {
            for edge in outgoing {
                let target = edge.target();
                if !visited.contains(&target) {
                    let mut new_path = path.clone();
                    new_path.push(edge.weight().to_string());
                    let mut new_visited = visited.clone();
                    new_visited.insert(node);
                    stack.push((target, new_path, new_visited));
                }
            }
        }
    }
    Some(all_paths)
}

type Fingerprint = (ActionKind, Vec<Option<InterpreterMemory>>);
fn fingerprint(a: &gcl::pg::Action, mems: &[InterpreterMemory]) -> Fingerprint {
    (
        a.into(),
        mems.iter().map(|mem| a.semantics(mem).ok()).collect(),
    )
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

#[test]
fn point4_oracle_memory_states() {
    use gcl::ast::Commands;
    use rand::SeedableRng;

    let commands: Commands = "if a > 5 -> x := 1 [] a > 2 -> x := 0 fi".parse().unwrap();

    let mut rng = rand::rngs::SmallRng::seed_from_u64(0xCEC34);
    for i in 0..10 {
        let mem = gcl::memory::Memory::from_targets_with(
            commands.fv(),
            &mut rng,
            |rng, _| rng.random_range(-10..=10),
            |rng, _| {
                let len = rng.random_range(5..=10);
                (0..len)
                    .map(|_| rng.random_range(-10..=10))
                    .collect::<Vec<_>>()
            },
        );
        println!("Sample {i}: {:?}", mem.variables);
    }
}

#[test]
fn witness_memories_cover_guards() {
    use ce_core::gn::compiler_gen::{
        CompilerContext, collect_guards, gen_commands, generate_witness_memories,
    };
    use rand::SeedableRng;

    let mut rng = rand::rngs::SmallRng::seed_from_u64(42);
    for _ in 0..20 {
        let mut cx = CompilerContext::default();
        cx.set_no_arrays(true);
        let commands = gen_commands(&mut cx, &mut rng);
        let guards = collect_guards(&commands);
        let witnesses = generate_witness_memories(&commands, &mut rng);

        for guard in &guards {
            match guard {
                gcl::ast::BExpr::Bool(_) => continue,
                _ => {
                    let any_evaluates = witnesses.iter().any(|mem| guard.semantics(mem).is_ok());
                    assert!(any_evaluates, "No witness evaluates guard: {:?}", guard);
                }
            }
        }
    }
}

#[test]
fn validate_uses_witness_mems_not_fixed_seed() {
    use ce_core::{Env, Generate};
    use rand::SeedableRng;

    let mut rng = rand::rngs::SmallRng::seed_from_u64(99);
    let input = Input::gn(&mut (), &mut rng);

    assert!(
        !input.witness_mems.is_empty(),
        "Generated input should have witness memories"
    );

    let output = CompilerEnv::run(&input).unwrap();
    let result = CompilerEnv::validate(&input, &output).unwrap();
    assert_eq!(result, (ce_core::ValidationResult::Correct, ()));
}

#[test]
fn path_fingerprints_differ_for_det_vs_nondet() {
    use gcl::ast::Commands;
    use gcl::pg::{Determinism, ProgramGraph};

    let commands: Commands = "if a > 0 -> x := 1 [] a < 10 -> x := 2 fi".parse().unwrap();

    let det_dot = ProgramGraph::new(Determinism::Deterministic, &commands).dot();
    let nondet_dot = ProgramGraph::new(Determinism::NonDeterministic, &commands).dot();

    let det_g = dot::dot_to_petgraph(&det_dot).unwrap();
    let nondet_g = dot::dot_to_petgraph(&nondet_dot).unwrap();

    let det_paths = path_fingerprints(&det_g);
    let nondet_paths = path_fingerprints(&nondet_g);

    assert!(
        det_paths.is_some(),
        "det graph should have qStart and paths"
    );
    assert!(
        nondet_paths.is_some(),
        "nondet graph should have qStart and paths"
    );
    assert_ne!(
        det_paths.unwrap(),
        nondet_paths.unwrap(),
        "Path fingerprints should differ between det and nondet compilation of overlapping guards"
    );
}

#[test]
fn path_fingerprints_match_for_correct_graph() {
    use ce_core::{Env, Generate};
    use rand::SeedableRng;

    let mut rng = rand::rngs::SmallRng::seed_from_u64(123);
    for _ in 0..10 {
        let input = Input::gn(&mut (), &mut rng);
        let output = CompilerEnv::run(&input).unwrap();
        let result = CompilerEnv::validate(&input, &output).unwrap();
        assert_eq!(
            result,
            (ce_core::ValidationResult::Correct, ()),
            "Reference output should validate as correct. Commands: {:?}",
            input.commands
        );
    }
}
