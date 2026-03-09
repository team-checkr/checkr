use ce_core::{Env, Generate, ValidationResult, define_env, rand};
use gcl::{
    ast::{AExpr, BExpr, Command, Commands, Guard, LogicOp, Target, Variable},
    interpreter::InterpreterMemory,
    pg::Node,
    semantics::SemanticsContext,
};
use serde::{Deserialize, Serialize};
use stdx::stringify::Stringify;
use indexmap::IndexSet;

define_env!(BiGCLEnv);

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "BiGCL")]
pub struct Input {
    commands: Stringify<Commands>,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "BiGCL")]
pub struct Output {
    binary: Stringify<Commands>,
}

impl Env for BiGCLEnv {
    type Input = Input;

    type Output = Output;

    type Meta = ();

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        let cmd = input
                    .commands
                    .try_parse()
                    .map_err(ce_core::EnvError::invalid_input_for_program(
                        "failed to parse commands",
                    ))?;
                    let mut fv = Ctx::new(cmd.fv().into_iter().map(|t|t.name().to_string()).collect());
        Ok(Output {
            binary: Stringify::new(
                cmd
                    .binify(&mut fv),
            ),
        })
    }

    fn validate(input: &Self::Input, output: &Self::Output) -> ce_core::Result<ValidationResult> {
        let (o_cmds, t_cmds) = match (input.commands.try_parse(), output.binary.try_parse()) {
            (Ok(ours), Ok(theirs)) => (ours, theirs),
            (Err(err), _) | (_, Err(err)) => {
                return Ok(ValidationResult::Mismatch {
                    reason: format!("failed to parse output: {err:?}"),
                });
            }
        };

        if !t_cmds.is_binary() {
            return Ok(ValidationResult::Mismatch {
                reason: "the output program is not of binary form".to_string(),
            });
        }

        Ok(check_programs_for_semantic_equivalence(&o_cmds, &t_cmds))
    }
}

impl Generate for Input {
    type Context = ();

    fn gn<R: rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        Self {
            commands: Stringify::new(Commands::gn(&mut Default::default(), rng)),
        }
    }
}

#[derive(Debug)]
pub struct Ctx {
    next_id: u32,
    fv: IndexSet<String>,
}

impl Ctx {
    pub fn new(fv: IndexSet<String>) -> Ctx {
        Ctx {
            next_id: 0,
            fv,
        }
    }
    fn fresh<T>(&mut self) -> Target<T> {
        loop {
            let id = self.next_id;
            self.next_id += 1;
            let name = format!("tmp{id}_");
            if self.fv.contains(&name) {continue;}
            return Target::Variable(Variable(name))
        }
    }
}

trait IsBinary {
    fn is_binary(&self) -> bool;
}
trait IsAtomic {
    fn is_atomic(&self) -> bool;
}

pub trait Binify {
    type Output;
    fn binify(&self, ctx: &mut Ctx) -> Self::Output;
}

trait Bitify {
    type Output;
    fn bitify(&self, ctx: &mut Ctx, target: &Target<Box<AExpr>>) -> Self::Output;
}

impl Binify for Commands {
    type Output = Commands;
    fn binify(&self, ctx: &mut Ctx) -> Commands {
        Commands(self.0.iter().flat_map(|c| c.binify(ctx).0).collect())
    }
}
impl Binify for Command {
    type Output = Commands;
    fn binify(&self, ctx: &mut Ctx) -> Commands {
        match self {
            Command::Assignment(target, x) => {
                let (target_cmds, target) = target.binify(ctx);
                let (x_cmds, x) = x.binify(ctx);
                target_cmds
                    .concat(&x_cmds)
                    .extend(Command::Assignment(target.clone(), x))
            }
            Command::Skip => Commands([Command::Skip].to_vec()),
            Command::If(guards) => guards.iter().rfold(
                Commands(
                    // NOTE: if all branches fail, we divide by zero to indicate
                    // stuck. Perhaps for the futrure we want to have a "false
                    // -> skip" branch
                    [Command::Assignment(
                        Target::Variable(Variable("stuck_".to_string())),
                        AExpr::Binary(
                            Box::new(AExpr::Number(1)),
                            gcl::ast::AOp::Divide,
                            Box::new(AExpr::Number(0)),
                        ),
                    )]
                    .to_vec(),
                ),
                |else_, Guard(b, c)| {
                    let tmp = ctx.fresh();
                    let pre = b.bitify(ctx, &tmp);
                    let g = BExpr::Rel(
                        AExpr::Reference(tmp.clone()),
                        gcl::ast::RelOp::Eq,
                        AExpr::Number(1),
                    );
                    pre.extend(Command::If(
                        [
                            Guard(g.clone(), c.binify(ctx)),
                            Guard(BExpr::Not(Box::new(g)), else_),
                        ]
                        .to_vec(),
                    ))
                },
            ),
            Command::Loop(guards) => match guards.as_slice() {
                [] => Commands([Command::Loop([].to_vec())].to_vec()),
                [Guard(b, c)] => {
                    let tmp = ctx.fresh();
                    let cmds = b.bitify(ctx, &tmp);
                    cmds.clone().extend(Command::Loop(
                        [Guard(
                            BExpr::Rel(
                                AExpr::Reference(tmp.clone()),
                                gcl::ast::RelOp::Eq,
                                AExpr::Number(1),
                            ),
                            c.binify(ctx).concat(&cmds),
                        )]
                        .to_vec(),
                    ))
                }
                _ => Commands(
                    [Command::Loop(
                        [Guard(
                            any(guards),
                            Commands([Command::If(guards.clone())].to_vec()),
                        )]
                        .to_vec(),
                    )]
                    .to_vec(),
                )
                .binify(ctx),
            },
        }
    }
}

fn any(guards: &[Guard]) -> BExpr {
    guards
        .iter()
        .map(|Guard(b, _c)| b.clone())
        .reduce(|a, b| BExpr::logic(a, LogicOp::Or, b))
        .unwrap_or(BExpr::Bool(false))
}

fn set_n(target: Target<Box<AExpr>>, n: i32) -> Commands {
    Commands([Command::Assignment(target, AExpr::Number(n))].to_vec())
}

impl Bitify for BExpr {
    type Output = Commands;

    fn bitify(&self, ctx: &mut Ctx, target: &Target<Box<AExpr>>) -> Self::Output {
        match self {
            BExpr::Bool(true) => set_n(target.clone(), 1),
            BExpr::Bool(false) => set_n(target.clone(), 0),
            BExpr::Rel(l, op, r) => {
                let (l_cmds, l) = l.binify(ctx);
                let (r_cmds, r) = r.binify(ctx);
                let cmds = l_cmds.concat(&r_cmds);
                let t = BExpr::Rel(l, *op, r);
                let f = BExpr::Not(Box::new(t.clone()));
                cmds.extend(Command::If(
                    [
                        Guard(t, set_n(target.clone(), 1)),
                        Guard(f, set_n(target.clone(), 0)),
                    ]
                    .to_vec(),
                ))
            }
            BExpr::Logic(l, op, r) => {
                match op {
                    LogicOp::And => Commands(
                        [Command::If(
                            [
                                Guard(
                                    l.as_ref().clone(),
                                    Commands(
                                        [Command::If(
                                            [
                                                Guard(r.as_ref().clone(), set_n(target.clone(), 1)),
                                                Guard(
                                                    BExpr::Not(r.clone()),
                                                    set_n(target.clone(), 0),
                                                ),
                                            ]
                                            .to_vec(),
                                        )]
                                        .to_vec(),
                                    ),
                                ),
                                Guard(BExpr::Not(l.clone()), set_n(target.clone(), 0)),
                            ]
                            .to_vec(),
                        )]
                        .to_vec(),
                    )
                    .binify(ctx),
                    LogicOp::Land => {
                        let l_tmp = ctx.fresh();
                        let r_tmp = ctx.fresh();
                        let l_cmds = l.bitify(ctx, &l_tmp);
                        let r_cmds = r.bitify(ctx, &r_tmp);
                        l_cmds.concat(&r_cmds).extend(Command::Assignment(
                            target.clone(),
                            AExpr::Binary(
                                Box::new(AExpr::Reference(l_tmp)),
                                gcl::ast::AOp::Times,
                                Box::new(AExpr::Reference(r_tmp)),
                            ),
                        ))
                    }
                    LogicOp::Or => Commands(
                        [Command::If(
                            [
                                Guard(l.as_ref().clone(), set_n(target.clone(), 1)),
                                Guard(
                                    BExpr::Not(l.clone()),
                                    Commands(
                                        [Command::If(
                                            [
                                                Guard(r.as_ref().clone(), set_n(target.clone(), 1)),
                                                Guard(
                                                    BExpr::Not(r.clone()),
                                                    set_n(target.clone(), 0),
                                                ),
                                            ]
                                            .to_vec(),
                                        )]
                                        .to_vec(),
                                    ),
                                ),
                            ]
                            .to_vec(),
                        )]
                        .to_vec(),
                    )
                    .binify(ctx),
                    LogicOp::Lor => {
                        let l_tmp = ctx.fresh();
                        let r_tmp = ctx.fresh();
                        let l_cmds = l.bitify(ctx, &l_tmp);
                        let r_cmds = r.bitify(ctx, &r_tmp);
                        l_cmds
                            .concat(&r_cmds)
                            // t := l + r
                            .extend(Command::Assignment(
                                target.clone(),
                                AExpr::Binary(
                                    Box::new(AExpr::Reference(l_tmp)),
                                    gcl::ast::AOp::Plus,
                                    Box::new(AExpr::Reference(r_tmp)),
                                ),
                            ))
                            // t := t + 1
                            .extend(Command::Assignment(
                                target.clone(),
                                AExpr::Binary(
                                    Box::new(AExpr::Reference(target.clone())),
                                    gcl::ast::AOp::Plus,
                                    Box::new(AExpr::Number(1)),
                                ),
                            ))
                            // t := t / 2
                            .extend(Command::Assignment(
                                target.clone(),
                                AExpr::Binary(
                                    Box::new(AExpr::Reference(target.clone())),
                                    gcl::ast::AOp::Divide,
                                    Box::new(AExpr::Number(2)),
                                ),
                            ))
                    }
                }
            }
            BExpr::Not(b) => {
                let cmds = b.bitify(ctx, target);
                cmds.extend(Command::Assignment(
                    target.clone(),
                    AExpr::Binary(
                        Box::new(AExpr::Number(1)),
                        gcl::ast::AOp::Minus,
                        Box::new(AExpr::Reference(target.clone())),
                    ),
                ))
            }
        }
    }
}
impl Binify for Target<Box<AExpr>> {
    type Output = (Commands, Target<Box<AExpr>>);

    fn binify(&self, ctx: &mut Ctx) -> Self::Output {
        match self {
            Target::Variable(v) => (Commands([].to_vec()), Target::Variable(v.clone())),
            Target::Array(a, idx) => {
                let (cmds, idx) = idx.binify(ctx);
                (cmds, Target::Array(a.clone(), Box::new(idx)))
            }
        }
    }
}
impl Binify for AExpr {
    type Output = (Commands, AExpr);

    fn binify(&self, ctx: &mut Ctx) -> Self::Output {
        match self {
            AExpr::Number(n) => (Commands([].to_vec()), AExpr::Number(*n)),
            AExpr::Reference(target) => {
                let (cmds, target) = target.binify(ctx);
                (cmds, AExpr::Reference(target.clone()))
            }
            AExpr::Binary(l, op, r) => {
                let (l_cmds, l) = l.binify(ctx);
                let (r_cmds, r) = r.binify(ctx);
                let mut cmds = l_cmds.concat(&r_cmds);
                let fresh = ctx.fresh();
                let cmd = Command::Assignment(
                    fresh.clone(),
                    AExpr::Binary(Box::new(l), *op, Box::new(r)),
                );
                cmds.0.push(cmd);
                (cmds, AExpr::Reference(fresh))
            }
            AExpr::Minus(x) => {
                let (mut cmds, x) = x.binify(ctx);
                let fresh = ctx.fresh();
                let cmd = Command::Assignment(fresh.clone(), AExpr::Minus(Box::new(x)));
                cmds.0.push(cmd);
                (cmds, AExpr::Reference(fresh))
            }
        }
    }
}

fn check_programs_for_semantic_equivalence(p1: &Commands, p2: &Commands) -> ValidationResult {
    let pg1 = gcl::pg::ProgramGraph::new(gcl::pg::Determinism::Deterministic, p1);
    let pg2 = gcl::pg::ProgramGraph::new(gcl::pg::Determinism::Deterministic, p2);

    let n_samples = 10;
    let n_steps = 1000;

    let mut rng = <rand::rngs::SmallRng as rand::SeedableRng>::seed_from_u64(0xCEC34);

    for _ in 0..n_samples {
        let assignment = generate_input_assignment(p2, &mut rng);

        let mut node1 = Node::Start;
        let mut mem1 = assignment.clone();
        let mut node2 = Node::Start;
        let mut mem2 = assignment.clone();
        let mut term1 = false;
        let mut term2 = false;

        for _ in 0..n_steps {
            if !term1 && let Some(next) = node1.next(&pg1, &mem1) {
                node1 = next.0;
                mem1 = next.1;
            } else {
                term1 = true;
            }
            if !term2 && let Some(next) = node2.next(&pg2, &mem2) {
                node2 = next.0;
                mem2 = next.1;
            } else {
                term2 = true;
            }
        }

        match (term1, term2) {
            (true, true) => {
                if mem1.agrees_on(&p1.fv(), &mem2) {
                    // NOTE: nothing more to do!
                } else {
                    return ValidationResult::Mismatch {
                        reason: format!("final memories differ:\n{:?}\n{:?}", mem1, mem2),
                    };
                }
            }
            (true, false) => {
                return ValidationResult::Unknown {
                    reason: "output program did not terminate".to_string(),
                };
            }
            (false, true) => {
                return ValidationResult::Unknown {
                    reason: "input program did not terminate".to_string(),
                };
            }
            (false, false) => {
                return ValidationResult::Unknown {
                    reason: "input and output program did not terminate".to_string(),
                };
            }
        }
    }

    ValidationResult::Correct
}

fn generate_input_assignment(
    commands: &gcl::ast::Commands,
    mut rng: &mut impl rand::Rng,
) -> InterpreterMemory {
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
}

impl IsBinary for Commands {
    fn is_binary(&self) -> bool {
        self.0.iter().all(|c| c.is_binary())
    }
}

impl IsBinary for Target<Box<AExpr>> {
    fn is_binary(&self) -> bool {
        match self {
            Target::Variable(_) => true,
            Target::Array(_, idx) => idx.is_atomic(),
        }
    }
}
impl IsBinary for Command {
    fn is_binary(&self) -> bool {
        match self {
            Command::Assignment(t, a) => t.is_binary() && a.is_binary(),
            Command::Skip => true,
            Command::If(guards) => {
                if let [Guard(a, _), Guard(BExpr::Not(b), _)] = guards.as_slice()
                    && a == &**b
                    && a.is_binary()
                {
                    true
                } else {
                    false
                }
            }
            Command::Loop(guards) => {
                if let [Guard(a, _)] = guards.as_slice() && a.is_binary() {
                    true
                } else {
                    false
                }
            }
        }
    }
}

impl IsBinary for AExpr {
    fn is_binary(&self) -> bool {
        match self {
            AExpr::Number(_) | AExpr::Reference(_) => true,
            AExpr::Binary(l, _, r) => l.is_atomic() && r.is_atomic(),
            AExpr::Minus(x) => x.is_atomic(),
        }
    }
}

impl IsAtomic for AExpr {
    fn is_atomic(&self) -> bool {
        match self {
            AExpr::Number(_) | AExpr::Reference(_) => true,
            AExpr::Binary(_, _, _) | AExpr::Minus(_) => false,
        }
    }
}

impl IsBinary for BExpr {
    fn is_binary(&self) -> bool {
        match self {
            BExpr::Bool(_) => true,
            BExpr::Rel(l, _, r) => l.is_atomic() && r.is_atomic(),
            BExpr::Logic(_, _, _) => false,
            BExpr::Not(_) => false,
        }
    }
}
