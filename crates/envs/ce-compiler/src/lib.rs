mod dot;

use std::collections::{BTreeMap, BTreeSet};

use ce_core::gn::compiler_gen::{
    CompilerContext, gen_commands_for_level, generate_witness_memories,
};
use ce_core::{Env, Generate, ValidationResult, define_env};
use gcl::{
    ast::{AExpr, Command, Commands, Guard, Target},
    interpreter::InterpreterMemory,
    pg::{Action, Determinism, ProgramGraph},
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
    #[serde(default = "default_level")]
    pub level: u8,
}

fn default_level() -> u8 {
    7
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
            return Ok((
                ValidationResult::Mismatch {
                    reason: diagnose_mismatch(input, &commands, &o_g, &t_g),
                },
                (),
            ));
        }

        // Path fingerprint check — catches structural difference action_bag misses
        match (path_fingerprints(&o_g), path_fingerprints(&t_g)) {
            (Some(o_paths), Some(t_paths)) if o_paths != t_paths => Ok((
                ValidationResult::Mismatch {
                    reason: diagnose_mismatch(input, &commands, &o_g, &t_g),
                },
                (),
            )),
            _ => Ok((ValidationResult::Correct, ())),
        }
    }
}

impl Generate for Input {
    type Context = ();

    fn gn<R: ce_core::rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        gen_input_for_level(7, rng)
    }
}

pub fn gen_input_for_level<R: rand::Rng>(level: u8, rng: &mut R) -> Input {
    let determinism = if level >= 6 {
        *[Determinism::Deterministic, Determinism::NonDeterministic]
            .choose(rng)
            .unwrap()
    } else {
        Determinism::Deterministic
    };
    let commands = gen_commands_for_level(level, &mut CompilerContext::new(10), rng);
    let mut w_rng = <rand::rngs::SmallRng as rand::SeedableRng>::seed_from_u64(0xCEC34);
    let witness_mems = generate_witness_memories(&commands, &mut w_rng);
    Input {
        commands: Stringify::new(commands),
        determinism,
        witness_mems,
        level,
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

//error helpers

fn count_actions<F>(g: &dot::ParsedGraph, pred: F) -> usize
where
    F: Fn(&gcl::pg::Action) -> bool,
{
    g.graph.edge_weights().filter(|a| pred(a)).count()
}

fn commands_have_skip(cmds: &Commands) -> bool {
    cmds.0.iter().any(|c| match c {
        Command::Skip => true,
        Command::If(guards) | Command::Loop(guards) => {
            guards.iter().any(|Guard(_, body)| commands_have_skip(body))
        }
        _ => false,
    })
}

fn commands_have_loop(cmds: &Commands) -> bool {
    cmds.0.iter().any(|c| match c {
        Command::Loop(_) => true,
        Command::If(guards) => guards.iter().any(|Guard(_, body)| commands_have_loop(body)),
        _ => false,
    })
}

fn commands_have_if(cmds: &Commands) -> bool {
    cmds.0.iter().any(|c| match c {
        Command::If(_) => true,
        Command::Loop(guards) => guards.iter().any(|Guard(_, body)| commands_have_if(body)),
        _ => false,
    })
}

fn max_loop_guards(cmds: &Commands) -> usize {
    cmds.0
        .iter()
        .map(|c| match c {
            Command::Loop(guards) => guards.len().max(
                guards
                    .iter()
                    .map(|Guard(_, body)| max_loop_guards(body))
                    .max()
                    .unwrap_or(0),
            ),
            Command::If(guards) => guards
                .iter()
                .map(|Guard(_, body)| max_loop_guards(body))
                .max()
                .unwrap_or(0),
            _ => 0,
        })
        .max()
        .unwrap_or(0)
}

fn max_if_guards(cmds: &Commands) -> usize {
    cmds.0
        .iter()
        .map(|c| match c {
            Command::If(guards) => guards.len().max(
                guards
                    .iter()
                    .map(|Guard(_, body)| max_if_guards(body))
                    .max()
                    .unwrap_or(0),
            ),
            Command::Loop(guards) => guards
                .iter()
                .map(|Guard(_, body)| max_if_guards(body))
                .max()
                .unwrap_or(0),
            _ => 0,
        })
        .max()
        .unwrap_or(0)
}

fn loop_body_has_if(cmds: &Commands) -> bool {
    cmds.0.iter().any(|c| match c {
        Command::Loop(guards) => guards.iter().any(|Guard(_, body)| {
            body.0.iter().any(|c2| matches!(c2, Command::If(_))) || loop_body_has_if(body)
        }),
        Command::If(guards) => guards.iter().any(|Guard(_, body)| loop_body_has_if(body)),
        _ => false,
    })
}

fn if_body_has_loop(cmds: &Commands) -> bool {
    cmds.0.iter().any(|c| match c {
        Command::If(guards) => guards.iter().any(|Guard(_, body)| {
            body.0.iter().any(|c2| matches!(c2, Command::Loop(_))) || if_body_has_loop(body)
        }),
        Command::Loop(guards) => guards.iter().any(|Guard(_, body)| if_body_has_loop(body)),
        _ => false,
    })
}

fn commands_have_array_assignment(cmds: &Commands) -> bool {
    cmds.0.iter().any(|c| match c {
        Command::Assignment(Target::Array(_, _), _) => true,
        Command::If(guards) | Command::Loop(guards) => guards
            .iter()
            .any(|Guard(_, body)| commands_have_array_assignment(body)),
        _ => false,
    })
}

fn aexpr_has_array_ref(e: &AExpr) -> bool {
    match e {
        AExpr::Reference(Target::Array(_, _)) => true,
        AExpr::Reference(Target::Variable(_)) | AExpr::Number(_) => false,
        AExpr::Binary(l, _, r) => aexpr_has_array_ref(l) || aexpr_has_array_ref(r),
        AExpr::Minus(inner) => aexpr_has_array_ref(inner),
    }
}

fn commands_have_array_read(cmds: &Commands) -> bool {
    cmds.0.iter().any(|c| match c {
        Command::Assignment(_, rhs) => aexpr_has_array_ref(rhs),
        Command::If(guards) | Command::Loop(guards) => guards
            .iter()
            .any(|Guard(_, body)| commands_have_array_read(body)),
        _ => false,
    })
}

fn diagnose_mismatch(
    input: &Input,
    commands: &Commands,
    o_g: &dot::ParsedGraph,
    t_g: &dot::ParsedGraph,
) -> String {
    let ref_skips = count_actions(o_g, |a| matches!(a, Action::Skip));
    let stu_skips = count_actions(t_g, |a| matches!(a, Action::Skip));
    let ref_conds = count_actions(o_g, |a| matches!(a, Action::Condition(_)));
    let stu_conds = count_actions(t_g, |a| matches!(a, Action::Condition(_)));

    if commands_have_skip(commands) && ref_skips > 0 && stu_skips == 0 {
        return "skip not implemented".to_string();
    }
    if commands_have_array_assignment(commands) {
        let ref_arr = count_actions(o_g, |a| {
            matches!(a, Action::Assignment(Target::Array(_, _), _))
        });
        let stu_arr = count_actions(t_g, |a| {
            matches!(a, Action::Assignment(Target::Array(_, _), _))
        });
        if ref_arr > 0 && stu_arr == 0 {
            return "array assignment not implemented (A[i] := e)".to_string();
        }
    }
    if commands_have_array_read(commands) {
        let ref_assigns = count_actions(o_g, |a| matches!(a, Action::Assignment(_, _)));
        let stu_assigns = count_actions(t_g, |a| matches!(a, Action::Assignment(_, _)));
        if ref_assigns != stu_assigns {
            return "array read not implemented (x := A[i])".to_string();
        }
    }

    let has_loop = commands_have_loop(commands);
    let has_if = commands_have_if(commands);

    if has_loop && has_if && loop_body_has_if(commands) && ref_conds > stu_conds {
        return "if-fi inside do-od loop not implemented".to_string();
    }

    if has_loop && has_if && if_body_has_loop(commands) && ref_conds > stu_conds {
        return "do-od loop inside if-fi not implemented".to_string();
    }

    if has_loop && ref_conds > stu_conds {
        let n = max_loop_guards(commands);
        if n > 1 {
            return format!("do-od loop with {n} guards not properly compiled");
        }
        return "do-od loop not implemented".to_string();
    }

    if has_if && ref_conds > stu_conds {
        let n = max_if_guards(commands);
        if n > 1 {
            if input.determinism == Determinism::NonDeterministic {
                return format!(
                    "if-fi with {n} guards not properly compiled (non-deterministic mode — check guard conditions are not made exclusive)"
                );
            }
            return format!("if-fi with {n} guards not properly compiled");
        }
        return "if-fi statement not implemented".to_string();
    }

    if o_g.graph.edge_count() == t_g.graph.edge_count() {
        if input.determinism == Determinism::NonDeterministic && (has_loop || has_if) {
            return "non-deterministic compilation incorrect — guard conditions should not be made exclusive in non-deterministic mode".to_string();
        }
        return "graph structure correct but edge semantics differ — check guard conditions or assignment expressions".to_string();
    }

    format!(
        "graph structure mismatch (reference: {} edges, student: {} edges)",
        o_g.graph.edge_count(),
        t_g.graph.edge_count()
    )
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
