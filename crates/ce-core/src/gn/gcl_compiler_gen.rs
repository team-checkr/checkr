/// Separate test-case generator for the compiler environment.
use gcl::ast::{
    AExpr, AOp, Array, BExpr, Command, Commands, Guard, LogicOp, RelOp, Target, Variable,
};
use rand::{Rng, SeedableRng, rngs::SmallRng, seq::IndexedRandom};

type ErasedRng = SmallRng;

type GenFn<G> = Box<dyn Fn(&mut CompilerContext, &mut ErasedRng) -> G>;
type GenOptions<G> = Vec<(f32, GenFn<G>)>;

pub struct CompilerContext {
    pub fuel: u32,
    pub recursion_limit: u32,
    pub negation_limit: u32,
    pub no_arrays: bool,
    pub names: Vec<String>,
    pub array_names: Vec<String>,
    pub level: u8,
}

impl Default for CompilerContext {
    fn default() -> Self {
        Self {
            fuel: 10,
            recursion_limit: Default::default(),
            negation_limit: Default::default(),
            no_arrays: Default::default(),
            names: ["a", "b", "c", "d"].map(Into::into).to_vec(),
            array_names: ["A", "B", "C"].map(Into::into).to_vec(),
            level: 7,
        }
    }
}

impl CompilerContext {
    pub fn new(fuel: u32) -> Self {
        CompilerContext {
            fuel,
            recursion_limit: fuel,
            negation_limit: fuel,
            ..Default::default()
        }
    }

    pub fn set_no_arrays(&mut self, no_arrays: bool) -> &mut Self {
        self.no_arrays = no_arrays;
        self
    }

    fn use_array(&self) -> bool {
        !self.no_arrays && !self.array_names.is_empty()
    }

    fn many<G, R: Rng>(
        &mut self,
        min: usize,
        max: usize,
        rng: &mut R,
        f: fn(&mut CompilerContext, &mut R) -> G,
    ) -> Vec<G> {
        let max = max.min(self.fuel as _).max(min);
        let n = rng.random_range(min..=max);
        if self.fuel < n as _ {
            self.fuel = 0;
        } else {
            self.fuel -= n as u32;
        }
        (0..n).map(|_| f(self, rng)).collect()
    }

    fn many_erased<G>(
        &mut self,
        min: usize,
        max: usize,
        rng: &mut ErasedRng,
        f: fn(&mut CompilerContext, &mut ErasedRng) -> G,
    ) -> Vec<G> {
        self.many(min, max, rng, f)
    }

    fn sample<G>(&mut self, rng: &mut ErasedRng, options: GenOptions<G>) -> G {
        let f = options.choose_weighted(rng, |o| o.0).unwrap();
        f.1(self, rng)
    }
}

fn bridge<R: Rng>(rng: &mut R) -> ErasedRng {
    SmallRng::seed_from_u64(rng.random())
}

// Scenario catalog to weigh random selections

#[derive(Clone, Copy)]
enum Scenario {
    SimpleAssignment,
    SequentialAssignments,
    Skip,
    SimpleIf,
    MultiGuardIf2,
    MultiGuardIf3,
    SimpleDo,
    DoNGuards2,
    DoNGuards3,
    DoNGuards4,
    NestedIfInDo,
    NestedDoInIf,
    ArrayAssignment,
    ArrayRead,
    NonDeterministicOverlapping,
    VariableReuse,
    VariableAsIndex,
    UnaryMinus,
    Random,
}

const CATALOG: &[(f32, Scenario)] = &[
    (1.0, Scenario::SimpleAssignment),
    (1.0, Scenario::UnaryMinus),
    (1.5, Scenario::SequentialAssignments),
    (1.0, Scenario::Skip),
    (2.0, Scenario::SimpleIf),
    (2.0, Scenario::MultiGuardIf2),
    (1.0, Scenario::MultiGuardIf3),
    (2.0, Scenario::SimpleDo),
    (1.5, Scenario::DoNGuards2),
    (1.0, Scenario::DoNGuards3),
    (0.5, Scenario::DoNGuards4),
    (2.0, Scenario::NestedIfInDo),
    (2.0, Scenario::NestedDoInIf),
    (2.0, Scenario::ArrayAssignment),
    (2.0, Scenario::ArrayRead),
    (2.0, Scenario::NonDeterministicOverlapping),
    (1.0, Scenario::VariableReuse),
    (1.0, Scenario::VariableAsIndex),
    (3.0, Scenario::Random),
];

fn scenario_level(s: Scenario) -> u8 {
    match s {
        Scenario::SimpleAssignment => 1,
        Scenario::UnaryMinus => 1,
        Scenario::SequentialAssignments => 2,
        Scenario::Skip => 2,
        Scenario::SimpleIf => 3,
        Scenario::MultiGuardIf2 => 3,
        Scenario::MultiGuardIf3 => 3,
        Scenario::SimpleDo => 4,
        Scenario::DoNGuards2 => 4,
        Scenario::DoNGuards3 => 4,
        Scenario::DoNGuards4 => 4,
        Scenario::NestedIfInDo => 4,
        Scenario::NestedDoInIf => 4,
        Scenario::ArrayAssignment => 5,
        Scenario::ArrayRead => 5,
        Scenario::VariableAsIndex => 5,
        Scenario::NonDeterministicOverlapping => 6,
        Scenario::VariableReuse => 6,
        Scenario::Random => 7,
    }
}

fn dispatch_scenario<R: Rng>(scenario: Scenario, cx: &mut CompilerContext, rng: &mut R) -> Commands {
    match scenario {
        Scenario::SimpleAssignment => gen_simple_assignment(cx, rng),
        Scenario::UnaryMinus => gen_unary_minus(cx, rng),
        Scenario::SequentialAssignments => gen_sequential_assignments(cx, rng),
        Scenario::Skip => gen_skip_program(cx, rng),
        Scenario::SimpleIf => gen_simple_if(cx, rng),
        Scenario::MultiGuardIf2 => gen_multi_guard_if(cx, rng, 2),
        Scenario::MultiGuardIf3 => gen_multi_guard_if(cx, rng, 3),
        Scenario::SimpleDo => gen_simple_do(cx, rng),
        Scenario::DoNGuards2 => gen_do_n_guards(cx, rng, 2),
        Scenario::DoNGuards3 => gen_do_n_guards(cx, rng, 3),
        Scenario::DoNGuards4 => gen_do_n_guards(cx, rng, 4),
        Scenario::NestedIfInDo => gen_nested_if_in_do(cx, rng),
        Scenario::NestedDoInIf => gen_nested_do_in_if(cx, rng),
        Scenario::ArrayAssignment => gen_array_assignment(cx, rng),
        Scenario::ArrayRead => gen_array_read(cx, rng),
        Scenario::NonDeterministicOverlapping => gen_non_deterministic_overlapping(cx, rng),
        Scenario::VariableReuse => gen_variable_reuse(cx, rng),
        Scenario::VariableAsIndex => gen_variable_as_index(cx, rng),
        Scenario::Random => gen_random_commands(cx, rng),
    }
}

pub fn gen_commands<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Commands {
    if cx.level < 5 {
        cx.no_arrays = true;
    }
    let catalog: Vec<(f32, Scenario)> = CATALOG
        .iter()
        .filter(|(_, s)| {
            if cx.level >= 7 {
                true
            } else {
                scenario_level(*s) == cx.level
            }
        })
        .cloned()
        .collect();
    let scenario = catalog.choose_weighted(rng, |item| item.0).unwrap().1;
    dispatch_scenario(scenario, cx, rng)
}

pub fn gen_commands_for_level<R: Rng>(level: u8, cx: &mut CompilerContext, rng: &mut R) -> Commands {
    cx.level = level;
    gen_commands(cx, rng)
}

fn gen_simple_assignment<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Commands {
    Commands(vec![Command::Assignment(gen_target(cx, rng), gen_aexpr(cx, rng))])
}

fn gen_unary_minus<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Commands {
    let var = cx.names.choose(rng).cloned().unwrap_or_else(|| "a".into());
    let inner = AExpr::Reference(Target::Variable(Variable(var)));
    let rhs = AExpr::Minus(Box::new(inner));
    let target = gen_target(cx, rng);
    Commands(vec![Command::Assignment(target, rhs)])
}

fn gen_sequential_assignments<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Commands {
    let n = rng.random_range(2..=4usize);
    Commands((0..n).map(|_| Command::Assignment(gen_target(cx, rng), gen_aexpr(cx, rng))).collect())
}

fn gen_random_commands<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Commands {
    Commands(cx.many(1, 10, rng, gen_command))
}

fn gen_skip_program<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Commands {
    let mut cmds: Vec<Command> = Vec::new();
    if rng.random_bool(0.5) {
        cmds.push(Command::Assignment(gen_target(cx, rng), gen_aexpr(cx, rng)));
    }
    cmds.push(Command::Skip);
    if rng.random_bool(0.3) {
        cmds.push(Command::Assignment(gen_target(cx, rng), gen_aexpr(cx, rng)));
    }
    Commands(cmds)
}

fn gen_simple_if<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Commands {
    let body = Commands(vec![Command::Assignment(
        gen_target(cx, rng),
        gen_aexpr(cx, rng),
    )]);
    let guard = Guard(gen_bexpr(cx, rng), body);
    Commands(vec![Command::If(vec![guard])])
}

fn gen_multi_guard_if<R: Rng>(cx: &mut CompilerContext, rng: &mut R, n: usize) -> Commands {
    let guards: Vec<Guard> = (0..n)
        .map(|_| {
            let body = Commands(vec![Command::Assignment(
                gen_target(cx, rng),
                gen_aexpr(cx, rng),
            )]);
            Guard(gen_bexpr(cx, rng), body)
        })
        .collect();
    Commands(vec![Command::If(guards)])
}

fn gen_simple_do<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Commands {
    let body = Commands(vec![Command::Assignment(
        gen_target(cx, rng),
        gen_aexpr(cx, rng),
    )]);
    let guard = Guard(gen_bexpr(cx, rng), body);
    Commands(vec![Command::Loop(vec![guard])])
}
fn gen_do_n_guards<R: Rng>(cx: &mut CompilerContext, rng: &mut R, n: usize) -> Commands {
    let guards: Vec<Guard> = (0..n)
        .map(|_| {
            let body = Commands(vec![Command::Assignment(
                gen_target(cx, rng),
                gen_aexpr(cx, rng),
            )]);
            Guard(gen_bexpr(cx, rng), body)
        })
        .collect();
    Commands(vec![Command::Loop(guards)])
}

fn gen_nested_if_in_do<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Commands {
    let inner_body = Commands(vec![Command::Assignment(
        gen_target(cx, rng),
        gen_aexpr(cx, rng),
    )]);
    let inner_guard = Guard(gen_bexpr(cx, rng), inner_body);
    let inner_if = Command::If(vec![inner_guard]);
    let outer_body = Commands(vec![inner_if]);
    let outer_guard = Guard(gen_bexpr(cx, rng), outer_body);
    Commands(vec![Command::Loop(vec![outer_guard])])
}

fn gen_nested_do_in_if<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Commands {
    let inner_body = Commands(vec![Command::Assignment(
        gen_target(cx, rng),
        gen_aexpr(cx, rng),
    )]);
    let inner_guard = Guard(gen_bexpr(cx, rng), inner_body);
    let inner_do = Command::Loop(vec![inner_guard]);
    let outer_body = Commands(vec![inner_do]);
    let outer_guard = Guard(gen_bexpr(cx, rng), outer_body);
    Commands(vec![Command::If(vec![outer_guard])])
}

fn gen_array_assignment<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Commands {
    let arr_name = cx
        .array_names
        .choose(rng)
        .cloned()
        .unwrap_or_else(|| "A".into());
    let idx = gen_aexpr(cx, rng);
    let rhs = gen_aexpr(cx, rng);
    let guaranteed = Command::Assignment(Target::Array(Array(arr_name), Box::new(idx)), rhs);
    Commands(vec![guaranteed])
}

fn gen_array_read<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Commands {
    let arr_name = cx
        .array_names
        .choose(rng)
        .cloned()
        .unwrap_or_else(|| "A".into());
    let idx = gen_aexpr(cx, rng);
    let rhs = AExpr::Reference(Target::Array(Array(arr_name), Box::new(idx)));
    let scalar_var = cx.names.choose(rng).cloned().unwrap_or_else(|| "x".into());
    let guaranteed = Command::Assignment(Target::Variable(Variable(scalar_var)), rhs);
    Commands(vec![guaranteed])
}

fn gen_non_deterministic_overlapping<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Commands {
    let var_name = cx.names.choose(rng).cloned().unwrap_or_else(|| "a".into());
    let var_ref = AExpr::Reference(Target::Variable(Variable(var_name)));
    let b1 = BExpr::Rel(var_ref.clone(), RelOp::Gt, AExpr::Number(0));
    let b2 = BExpr::Rel(var_ref.clone(), RelOp::Lt, AExpr::Number(10));
    let g1 = Guard(
        b1,
        Commands(vec![Command::Assignment(
            gen_target(cx, rng),
            gen_aexpr(cx, rng),
        )]),
    );
    let g2 = Guard(
        b2,
        Commands(vec![Command::Assignment(
            gen_target(cx, rng),
            gen_aexpr(cx, rng),
        )]),
    );
    Commands(vec![Command::If(vec![g1, g2])])
}

fn gen_variable_reuse<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Commands {
    let var_name = cx.names.choose(rng).cloned().unwrap_or_else(|| "a".into());
    let make_target = || Target::Variable(Variable(var_name.clone()));
    let g1 = Guard(
        gen_bexpr(cx, rng),
        Commands(vec![Command::Assignment(make_target(), gen_aexpr(cx, rng))]),
    );
    let g2 = Guard(
        gen_bexpr(cx, rng),
        Commands(vec![Command::Assignment(make_target(), gen_aexpr(cx, rng))]),
    );
    Commands(vec![Command::If(vec![g1, g2])])
}

fn gen_variable_as_index<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Commands {
    let var_name = cx
        .array_names
        .choose(rng)
        .cloned()
        .unwrap_or_else(|| "a".into());
    let arr_name = cx
        .array_names
        .choose(rng)
        .cloned()
        .unwrap_or_else(|| "A".into());
    let idx = AExpr::Reference(Target::Variable(Variable(var_name.clone())));
    let arr_ass = Command::Assignment(
        Target::Array(Array(arr_name), Box::new(idx)),
        gen_aexpr(cx, rng),
    );
    let scalar_assign =
        Command::Assignment(Target::Variable(Variable(var_name)), gen_aexpr(cx, rng));

    Commands(vec![arr_ass, scalar_assign])
}

pub fn gen_command<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Command {
    cx.recursion_limit = 5;
    cx.negation_limit = 3;
    let mut erng = bridge(rng);
    cx.sample(
        &mut erng,
        vec![
            (
                1.0,
                Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                    Command::Assignment(gen_target(cx, rng), gen_aexpr(cx, rng))
                }),
            ),
            // skip is a real compiler edge — include it at a low weight
            (
                0.3,
                Box::new(|_cx: &mut CompilerContext, _rng: &mut ErasedRng| Command::Skip),
            ),
            (
                0.6,
                Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                    Command::If(cx.many_erased(1, 10, rng, gen_guard_erased))
                }),
            ),
            (
                0.3,
                Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                    Command::Loop(cx.many_erased(1, 10, rng, gen_guard_erased))
                }),
            ),
        ],
    )
}

pub fn gen_target<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Target<Box<AExpr>> {
    gen_reference(cx, rng)
}

pub fn gen_guard<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Guard {
    cx.recursion_limit = 5;
    cx.negation_limit = 3;
    Guard(gen_bexpr(cx, rng), gen_commands(cx, rng))
}

pub fn gen_aexpr<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> AExpr {
    let mut erng = bridge(rng);
    cx.sample(
        &mut erng,
        vec![
            (
                0.4,
                Box::new(|_cx: &mut CompilerContext, rng: &mut ErasedRng| {
                    AExpr::Number(rng.random_range(-100..=100))
                }),
            ),
            (
                if cx.names.is_empty() && cx.array_names.is_empty() {
                    0.0
                } else {
                    0.8
                },
                Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                    AExpr::Reference(gen_reference(cx, rng))
                }),
            ),
            (
                if cx.recursion_limit == 0 || cx.fuel == 0 {
                    0.0
                } else {
                    0.9
                },
                Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                    cx.recursion_limit = cx.recursion_limit.checked_sub(1).unwrap_or_default();
                    AExpr::binary(gen_aexpr(cx, rng), gen_aop(cx, rng), gen_aexpr(cx, rng))
                }),
            ),
            // Unary minus: -expr
            (
                if cx.recursion_limit == 0 || cx.fuel == 0 {
                    0.0
                } else {
                    0.4
                },
                Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                    cx.recursion_limit = cx.recursion_limit.checked_sub(1).unwrap_or_default();
                    AExpr::Minus(Box::new(gen_aexpr(cx, rng)))
                }),
            ),
        ],
    )
}

pub fn gen_aop<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> AOp {
    let mut erng = bridge(rng);
    cx.sample(
        &mut erng,
        vec![
            (
                0.5,
                Box::new(|_cx: &mut CompilerContext, _rng: &mut ErasedRng| AOp::Plus),
            ),
            (
                0.4,
                Box::new(|_cx: &mut CompilerContext, _rng: &mut ErasedRng| AOp::Minus),
            ),
            (
                0.4,
                Box::new(|_cx: &mut CompilerContext, _rng: &mut ErasedRng| AOp::Times),
            ),
            (
                0.1,
                Box::new(|_cx: &mut CompilerContext, _rng: &mut ErasedRng| AOp::Pow),
            ),
            (
                0.3,
                Box::new(|_cx: &mut CompilerContext, _rng: &mut ErasedRng| AOp::Divide),
            ),
        ],
    )
}

pub fn gen_bexpr<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> BExpr {
    let mut erng = bridge(rng);
    cx.sample(
        &mut erng,
        vec![
            (
                0.2,
                Box::new(|_cx: &mut CompilerContext, rng: &mut ErasedRng| {
                    BExpr::Bool(rng.random())
                }),
            ),
            (
                if cx.recursion_limit == 0 { 0.0 } else { 0.7 },
                Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                    cx.recursion_limit = cx.recursion_limit.checked_sub(1).unwrap_or_default();
                    BExpr::Rel(gen_aexpr(cx, rng), gen_relop(cx, rng), gen_aexpr(cx, rng))
                }),
            ),
            (
                if cx.recursion_limit == 0 { 0.0 } else { 0.7 },
                Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                    cx.recursion_limit = cx.recursion_limit.checked_sub(1).unwrap_or_default();
                    BExpr::logic(gen_bexpr(cx, rng), gen_logicop(cx, rng), gen_bexpr(cx, rng))
                }),
            ),
            (
                if cx.negation_limit == 0 { 0.0 } else { 0.4 },
                Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                    cx.negation_limit = cx.negation_limit.checked_sub(1).unwrap_or_default();
                    BExpr::Not(Box::new(gen_bexpr(cx, rng)))
                }),
            ),
        ],
    )
}

pub fn gen_relop<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> RelOp {
    let mut erng = bridge(rng);
    cx.sample(
        &mut erng,
        vec![
            (
                0.3,
                Box::new(|_cx: &mut CompilerContext, _rng: &mut ErasedRng| RelOp::Eq),
            ),
            (
                0.3,
                Box::new(|_cx: &mut CompilerContext, _rng: &mut ErasedRng| RelOp::Gt),
            ),
            (
                0.3,
                Box::new(|_cx: &mut CompilerContext, _rng: &mut ErasedRng| RelOp::Ge),
            ),
            (
                0.3,
                Box::new(|_cx: &mut CompilerContext, _rng: &mut ErasedRng| RelOp::Lt),
            ),
            (
                0.3,
                Box::new(|_cx: &mut CompilerContext, _rng: &mut ErasedRng| RelOp::Le),
            ),
            (
                0.3,
                Box::new(|_cx: &mut CompilerContext, _rng: &mut ErasedRng| RelOp::Ne),
            ),
        ],
    )
}

pub fn gen_logicop<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> LogicOp {
    let mut erng = bridge(rng);
    cx.sample(
        &mut erng,
        vec![
            (
                0.3,
                Box::new(|_cx: &mut CompilerContext, _rng: &mut ErasedRng| LogicOp::And),
            ),
            (
                0.3,
                Box::new(|_cx: &mut CompilerContext, _rng: &mut ErasedRng| LogicOp::Land),
            ),
            (
                0.3,
                Box::new(|_cx: &mut CompilerContext, _rng: &mut ErasedRng| LogicOp::Or),
            ),
            (
                0.3,
                Box::new(|_cx: &mut CompilerContext, _rng: &mut ErasedRng| LogicOp::Lor),
            ),
        ],
    )
}

fn gen_guard_erased(cx: &mut CompilerContext, rng: &mut ErasedRng) -> Guard {
    gen_guard(cx, rng)
}

fn gen_reference<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Target<Box<AExpr>> {
    let mut erng = bridge(rng);
    cx.sample(
        &mut erng,
        vec![
            (
                if cx.names.is_empty() { 0.0 } else { 0.7 },
                Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                    Target::Variable(Variable(cx.names.choose(rng).cloned().unwrap()))
                }),
            ),
            (
                if cx.use_array() { 0.3 } else { 0.0 },
                Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                    let name = cx
                        .array_names
                        .choose(rng)
                        .cloned()
                        .unwrap_or_else(|| "A".into());
                    Target::Array(Array(name), Box::new(gen_aexpr(cx, rng)))
                }),
            ),
        ],
    )
}

pub fn collect_guards(cmds: &Commands) -> Vec<BExpr> {
    let mut result = Vec::new();
    for cmd in &cmds.0 {
        match cmd {
            Command::If(guards) | Command::Loop(guards) => {
                for Guard(b, body) in guards {
                    result.push(b.clone());
                    result.extend(collect_guards(body));
                }
            }
            _ => {}
        }
    }
    result
}

pub fn generate_witness_memories<R: Rng>(
    commands: &Commands,
    rng: &mut R,
) -> Vec<gcl::interpreter::InterpreterMemory> {
    use gcl::interpreter::InterpreterMemory;
    use gcl::memory::Memory;

    let guards = collect_guards(commands);
    let fv = commands.fv();
    let mut witnesses = Vec::new();

    for guard in &guards {
        let mut true_found = false;
        let mut false_found = false;

        for _ in 0..100 {
            if true_found && false_found {
                break;
            }
            let mem = Memory::from_targets_with(
                fv.clone(),
                &mut *rng,
                |rng, _| rng.random_range(-10..=10),
                |rng, _| {
                    let len = rng.random_range(5..=10);
                    (0..len).map(|_| rng.random_range(-10..=10)).collect()
                },
            );
            let interp_mem = InterpreterMemory {
                variables: mem.variables,
                arrays: mem.arrays,
            };
            match guard.semantics(&interp_mem) {
                Ok(true) if !true_found => {
                    true_found = true;
                    witnesses.push(interp_mem);
                }
                Ok(false) if !false_found => {
                    false_found = true;
                    witnesses.push(interp_mem);
                }
                _ => {}
            }
        }
    }

    witnesses
}
