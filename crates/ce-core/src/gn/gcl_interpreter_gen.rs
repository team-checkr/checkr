use gcl::ast::{
    AExpr, AOp, Array, BExpr, Command, Commands, Guard, LogicOp, RelOp, Target, Variable,
};
use rand::{
    Rng, SeedableRng,
    rngs::SmallRng,
    seq::{IndexedRandom, SliceRandom},
};

use crate::gn::compiler_gen::{CompilerContext, gen_aexpr, gen_bexpr, gen_target};
type ErasedRng = SmallRng;

type GenFn<G> = Box<dyn Fn(&mut CompilerContext, &mut ErasedRng) -> G>;
type GenOptions<G> = Vec<(f32, GenFn<G>)>;

type GenFnNested<Command> =
    Box<dyn Fn(&mut CompilerContext, &mut ErasedRng, &GenOptionsNested<Command>) -> Command>;
pub struct GenOptionsNested<Command>(pub Vec<(f32, GenFnNested<Command>)>);

pub struct InterpreterContext {
    pub level: u32,
    pub compiler_context: CompilerContext,
}

impl Default for InterpreterContext {
    fn default() -> Self {
        Self {
            level: 1,
            compiler_context: CompilerContext::default(),
        }
    }
}

impl InterpreterContext {
    pub fn new<R: Rng>(level: u32, compiler_context: CompilerContext, _rng: &mut R) -> Self {
        InterpreterContext {
            level,
            compiler_context,
            ..Default::default()
        }
    }
}

impl GenOptionsNested<Commands> {
    pub fn generate(&self, cx: &mut CompilerContext, rng: &mut ErasedRng) -> Commands {
        cx.fuel = cx.fuel.checked_sub(1).unwrap_or_default();

        let mut erng = SmallRng::seed_from_u64(rng.random());

        if cx.fuel == 0 {
            let end_opt: GenOptionsNested<Commands> = lvl_assignment(cx);

            let (_, f) = end_opt.0.choose_weighted(&mut erng, |item| item.0).unwrap();

            return f(cx, &mut erng, &self);
        }

        let (_i, f) = &self.0.choose_weighted(&mut erng, |item| item.0).unwrap();

        let _i = _i + 1.0;

        f(cx, &mut erng, &self)
    }
}

pub fn generate_selective<R: Rng>(cx: &mut InterpreterContext, rng: &mut R) -> Commands {
    let mut cmds: Vec<Command> = Vec::new();
    let mut generation_options: GenOptionsNested<Commands> = GenOptionsNested(vec![]);

    // ? 1 Assignment: state updates (single assignments)

    // return the command immediately if it is level 1,
    // as the generator will normally always generate a sequence of at least 3 commands
    if cx.level == 1 {
        let mut erng = SmallRng::seed_from_u64(rng.random());
        cmds.append(
            &mut lvl_assignment(&mut cx.compiler_context)
                .generate(&mut cx.compiler_context, &mut erng)
                .0,
        );
        return Commands(cmds);
    }

    generation_options
        .0
        .append(&mut lvl_assignment(&mut cx.compiler_context).0);

    if cx.level >= 2 {
        // ? 2 Sequencing: multiple steps ( sequential composition C1 ; C2, always deterministic, no branching, Should always be common afterwards)
        if cx.level == 2 {
            let mut erng = SmallRng::seed_from_u64(rng.random());

            let mut gen_options: GenOptionsNested<Commands> =
                lvl_assignment(&mut cx.compiler_context);
            gen_options
                .0
                .append(&mut lvl_sequencing(&mut cx.compiler_context).0);

            cmds.append(&mut gen_options.generate(&mut cx.compiler_context, &mut erng).0);
        }

        generation_options
            .0
            .append(&mut lvl_sequencing(&mut cx.compiler_context).0);
    }

    if cx.level >= 3 {
        // ? 3 Conditionals: bool branching, execution depends on guards being true, always deterministic. For example: if b1 → C1 [] ... [] bn → Cn fi
        if cx.level == 3 {
            let mut erng = SmallRng::seed_from_u64(rng.random());

            let mut gen_options: GenOptionsNested<Commands> =
                lvl_assignment(&mut cx.compiler_context);
            gen_options.0.append(&mut lvl_conditionals().0);

            cmds.append(&mut gen_options.generate(&mut cx.compiler_context, &mut erng).0);
        }

        generation_options.0.append(&mut lvl_conditionals().0);
    }

    if cx.level >= 4 {
        // ? 4 Stuck: unsolvable programs, guards are all false, or semantics undefined like division by zero
        if cx.level == 4 {
            // as this is before loops they have to be disabled

            let mut erng = SmallRng::seed_from_u64(rng.random());

            let mut gen_options: GenOptionsNested<Commands> =
                lvl_assignment(&mut cx.compiler_context);
            gen_options
                .0
                .append(&mut lvl_stuck(&mut cx.compiler_context).0);

            cmds.append(&mut gen_options.generate(&mut cx.compiler_context, &mut erng).0);
        }

        generation_options
            .0
            .append(&mut lvl_stuck(&mut cx.compiler_context).0);
    }

    if cx.level >= 5 {
        // ? 5 Loops: long execution (execution that may surpass the trace length limit) do GC od introduces iteration, exits when no guards hold. This level will bring potentially infinite execution, and differences between terminated, running, stuck( we have in the code exactly as TerminationState::Running TerminationState::Terminated TerminationState::Stuck
        if cx.level == 5 {
            let mut erng = SmallRng::seed_from_u64(rng.random());

            let mut gen_options: GenOptionsNested<Commands> =
                lvl_assignment(&mut cx.compiler_context);
            gen_options
                .0
                .append(&mut lvl_loops(&mut cx.compiler_context).0);

            cmds.append(&mut gen_options.generate(&mut cx.compiler_context, &mut erng).0);
        }

        generation_options
            .0
            .append(&mut lvl_loops(&mut cx.compiler_context).0);
    }

    if cx.level >= 6 {
        // ? 6 Nondeterminism: multiple valid paths, overlapping guards in if / do (we have also implemented the new nondeterministic path for this one: nexts() choose_random(...)
        if cx.level == 6 {
            let mut erng = SmallRng::seed_from_u64(rng.random());

            let mut gen_options: GenOptionsNested<Commands> =
                lvl_assignment(&mut cx.compiler_context);
            gen_options
                .0
                .append(&mut lvl_nondeterminism(&mut cx.compiler_context).0);

            cmds.append(&mut gen_options.generate(&mut cx.compiler_context, &mut erng).0);
        }

        generation_options
            .0
            .append(&mut lvl_nondeterminism(&mut cx.compiler_context).0);
    }

    // if cx.level >= 7 {
    //     // ? 7 Undefined semantics:
    //     if cx.level == 7 {
    //         let mut erng = SmallRng::seed_from_u64(rng.random());
    //
    //         let mut gen_options: GenOptionsNested<Commands> =
    //             lvl_assignment(&mut cx.compiler_context);
    //         gen_options
    //             .0
    //             .append(&mut lvl_undefined(&mut cx.compiler_context).0);
    //
    //         cmds.append(&mut gen_options.generate(&mut cx.compiler_context, &mut erng).0);
    //     }
    //
    //     generation_options
    //         .0
    //         .append(&mut lvl_undefined(&mut cx.compiler_context).0);
    // }

    if cx.level >= 7 {
        // ? 8 Composition: (all previous levels are guaranteed here)
        if cx.level == 7 {
            let mut erng = SmallRng::seed_from_u64(rng.random());
            cmds.append(
                &mut lvl_assignment(&mut cx.compiler_context)
                    .generate(&mut cx.compiler_context, &mut erng)
                    .0,
            );
            cmds.append(
                &mut lvl_sequencing(&mut cx.compiler_context)
                    .generate(&mut cx.compiler_context, &mut erng)
                    .0,
            );
            cmds.append(
                &mut lvl_conditionals()
                    .generate(&mut cx.compiler_context, &mut erng)
                    .0,
            );
            cmds.append(
                &mut lvl_stuck(&mut cx.compiler_context)
                    .generate(&mut cx.compiler_context, &mut erng)
                    .0,
            );
            cmds.append(
                &mut lvl_loops(&mut cx.compiler_context)
                    .generate(&mut cx.compiler_context, &mut erng)
                    .0,
            );
            cmds.append(
                &mut lvl_nondeterminism(&mut cx.compiler_context)
                    .generate(&mut cx.compiler_context, &mut erng)
                    .0,
            );
            // cmds.append(
            //     &mut lvl_undefined(&mut cx.compiler_context)
            //         .generate(&mut cx.compiler_context, &mut erng)
            //         .0,
            // );
        }

        //generation_options.0.append(&mut lvl_composition().0);
    }

    let min = 3;
    let min: u32 = 0.max((min - cmds.len()).try_into().unwrap());
    let max = cx.compiler_context.fuel.max(min);
    let n = rng.random_range(min..=max);

    for i in 0..n {
        let mut erng = SmallRng::seed_from_u64(rng.random());
        cmds.append(
            &mut generation_options
                .generate(&mut cx.compiler_context, &mut erng)
                .0,
        );

        if cx.compiler_context.fuel <= 0 && i <= min {
            break;
        }
    }

    // so that the guaranteed additions do not always appear as the first value
    cmds.shuffle(rng);

    Commands(cmds)
}

// ? 1 Assignment: state updates (single assignments)
fn lvl_assignment(_cx: &mut CompilerContext) -> GenOptionsNested<Commands> {
    GenOptionsNested(vec![
        (
            4.0,
            Box::new(
                |cx: &mut CompilerContext,
                 rng: &mut ErasedRng,
                 _gnopt: &GenOptionsNested<Commands>| {
                    Commands(vec![gen_assignment(cx, rng)])
                },
            ),
        ),
        (
            4.0,
            Box::new(
                |_cx: &mut CompilerContext,
                 _rng: &mut ErasedRng,
                 _gnopt: &GenOptionsNested<Commands>| Commands(vec![Command::Skip]),
            ),
        ),
    ])
}

// ? 2 Sequencing: multiple steps ( sequential composition C1 ; C2, always deterministic, no branching, Should always be guaranteed afterwards)
fn lvl_sequencing(cx: &mut CompilerContext) -> GenOptionsNested<Commands> {
    GenOptionsNested(vec![
        (
            if cx.fuel >= 2 { 0.3 } else { 0.0 },
            Box::new(
                |cx: &mut CompilerContext,
                 rng: &mut ErasedRng,
                 gnopt: &GenOptionsNested<Commands>| {
                    cx.recursion_limit = cx.recursion_limit.checked_sub(2).unwrap_or_default();

                    let mut seq = gnopt.generate(cx, rng).0;
                    seq.append(&mut gnopt.generate(cx, rng).0);
                    Commands(seq)
                },
            ),
        ),
        (
            if cx.fuel >= 3 { 0.1 } else { 0.0 },
            Box::new(
                |cx: &mut CompilerContext,
                 rng: &mut ErasedRng,
                 gnopt: &GenOptionsNested<Commands>| {
                    cx.recursion_limit = cx.recursion_limit.checked_sub(3).unwrap_or_default();

                    let mut seq = gnopt.generate(cx, rng).0;
                    seq.append(&mut gnopt.generate(cx, rng).0);
                    seq.append(&mut gnopt.generate(cx, rng).0);
                    Commands(seq)
                },
            ),
        ),
    ])
}

// ? 3 Conditionals: bool branching, execution depends on guards being true, always deterministic. For example: if b1 → C1 [] ... [] bn → Cn fi
fn lvl_conditionals() -> GenOptionsNested<Commands> {
    GenOptionsNested(vec![(
        0.5,
        Box::new(
            |cx: &mut CompilerContext, rng: &mut ErasedRng, gnopt: &GenOptionsNested<Commands>| {
                Commands(vec![Command::If(vec![Guard(
                    gen_bexpr(cx, rng),
                    gnopt.generate(cx, rng),
                )])])
            },
        ),
    )])
}

// ? 4 Stuck: unsolvable programs, guards are all false
fn lvl_stuck(_cx: &mut CompilerContext) -> GenOptionsNested<Commands> {
    GenOptionsNested(vec![
        (
            0.5,
            Box::new(
                |cx: &mut CompilerContext,
                 rng: &mut ErasedRng,
                 gnopt: &GenOptionsNested<Commands>| {
                    Commands(vec![Command::If(vec![Guard(
                        gen_bexpr_stuck(cx, rng),
                        gnopt.generate(cx, rng),
                    )])])
                },
            ),
        ),
        (
            0.5,
            Box::new(
                |cx: &mut CompilerContext,
                 rng: &mut ErasedRng,
                 _gnopt: &GenOptionsNested<Commands>| {
                    let name = cx
                        .array_names
                        .choose(rng)
                        .cloned()
                        .unwrap_or_else(|| "A".into());
                    let arr = Target::Array(
                        Array(name),
                        Box::new(AExpr::Number(rng.random_range(-100..=-1))),
                    );

                    Commands(vec![Command::Assignment(arr, gen_aexpr(cx, rng))])
                },
            ),
        ),
    ])
}

// ? 5 Loops: long execution (execution that may surpass the trace length limit) do GC od introduces iteration, exits when no guards hold. This level will bring potentially infinite execution, and differences between terminated, running, stuck( we have in the code exactly as TerminationState::Running TerminationState::Terminated TerminationState::Stuck
fn lvl_loops(cx: &mut CompilerContext) -> GenOptionsNested<Commands> {
    GenOptionsNested(vec![(
        0.5,
        Box::new(
            |cx: &mut CompilerContext, rng: &mut ErasedRng, gnopt: &GenOptionsNested<Commands>| {
                Commands(vec![Command::Loop(vec![Guard(
                    gen_bexpr(cx, rng),
                    gnopt.generate(cx, rng),
                )])])
            },
        ),
    )])
}

// ? 6 Nondeterminism: multiple valid paths, overlapping guards in if / do (we have also implemented the new nondeterministic path for this one: nexts() choose_random(...)
fn lvl_nondeterminism(cx: &mut CompilerContext) -> GenOptionsNested<Commands> {
    GenOptionsNested(vec![
        (
            0.5,
            Box::new(
                |cx: &mut CompilerContext,
                 rng: &mut ErasedRng,
                 gnopt: &GenOptionsNested<Commands>| {
                    Commands(vec![Command::If(gen_multiple_guards(cx, rng, gnopt))])
                },
            ),
        ),
        (
            0.5,
            Box::new(
                |cx: &mut CompilerContext,
                 rng: &mut ErasedRng,
                 gnopt: &GenOptionsNested<Commands>| {
                    Commands(vec![Command::Loop(gen_multiple_guards(cx, rng, gnopt))])
                },
            ),
        ),
    ])
}

// ? 7 Undefined semantics: division by zero
fn lvl_undefined(cx: &mut CompilerContext) -> GenOptionsNested<Commands> {
    GenOptionsNested(vec![
        (
            0.5,
            Box::new(
                |cx: &mut CompilerContext,
                 rng: &mut ErasedRng,
                 gnopt: &GenOptionsNested<Commands>| {
                    Commands(vec![Command::If(gen_undefined_guards(cx, rng, gnopt))])
                },
            ),
        ),
        (
            0.5,
            Box::new(
                |cx: &mut CompilerContext,
                 rng: &mut ErasedRng,
                 gnopt: &GenOptionsNested<Commands>| {
                    Commands(vec![Command::Loop(gen_undefined_guards(cx, rng, gnopt))])
                },
            ),
        ),
    ])
}

// ? helper functions

fn gen_assignment<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> Command {
    Command::Assignment(gen_target(cx, rng), gen_aexpr(cx, rng))
}

pub fn gen_bexpr_stuck<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> BExpr {
    let generation_options: GenOptions<BExpr> = vec![
        (5.0, Box::new(|_, _| BExpr::Bool(false))),
        (
            if cx.negation_limit == 0 { 0.0 } else { 1.0 },
            Box::new(|cx: &mut CompilerContext, _| {
                cx.negation_limit = cx.negation_limit.checked_sub(1).unwrap_or_default();
                BExpr::Not(Box::new(BExpr::Bool(true)))
            }),
        ),
        (
            if cx.negation_limit == 0 { 0.0 } else { 1.0 },
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                cx.negation_limit = cx.negation_limit.checked_sub(1).unwrap_or_default();
                BExpr::Not(Box::new(BExpr::Not(Box::new(gen_bexpr_stuck(cx, rng)))))
            }),
        ),
        (
            0.25,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                cx.negation_limit = cx.negation_limit.checked_sub(1).unwrap_or_default();
                BExpr::Logic(
                    Box::new(gen_bexpr_stuck(cx, rng)),
                    LogicOp::And,
                    Box::new(gen_bexpr(cx, rng)),
                )
            }),
        ),
        (
            0.25,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                cx.negation_limit = cx.negation_limit.checked_sub(1).unwrap_or_default();
                BExpr::Logic(
                    Box::new(gen_bexpr(cx, rng)),
                    LogicOp::And,
                    Box::new(gen_bexpr_stuck(cx, rng)),
                )
            }),
        ),
        (
            0.25,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                cx.negation_limit = cx.negation_limit.checked_sub(1).unwrap_or_default();
                BExpr::Logic(
                    Box::new(gen_bexpr_stuck(cx, rng)),
                    LogicOp::Land,
                    Box::new(gen_bexpr(cx, rng)),
                )
            }),
        ),
        (
            0.25,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                cx.negation_limit = cx.negation_limit.checked_sub(1).unwrap_or_default();
                BExpr::Logic(
                    Box::new(gen_bexpr(cx, rng)),
                    LogicOp::Land,
                    Box::new(gen_bexpr_stuck(cx, rng)),
                )
            }),
        ),
        (
            0.5,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                cx.negation_limit = cx.negation_limit.checked_sub(1).unwrap_or_default();
                BExpr::Logic(
                    Box::new(gen_bexpr_stuck(cx, rng)),
                    LogicOp::Or,
                    Box::new(gen_bexpr_stuck(cx, rng)),
                )
            }),
        ),
        (
            0.5,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                cx.negation_limit = cx.negation_limit.checked_sub(1).unwrap_or_default();
                BExpr::Logic(
                    Box::new(gen_bexpr_stuck(cx, rng)),
                    LogicOp::Lor,
                    Box::new(gen_bexpr_stuck(cx, rng)),
                )
            }),
        ),
        (
            1.0,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                cx.negation_limit = cx.negation_limit.checked_sub(1).unwrap_or_default();
                gen_bexpr_stuck_rel(cx, rng)
            }),
        ),
    ];

    let mut erng = SmallRng::seed_from_u64(rng.random());

    let choice: BExpr = generation_options
        .choose_weighted(&mut erng, |item| item.0)
        .unwrap()
        .1(cx, &mut erng);

    choice
}

pub fn gen_bexpr_stuck_rel<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> BExpr {
    let generation_options: GenOptions<BExpr> = vec![
        (
            1.0,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                cx.negation_limit = cx.negation_limit.checked_sub(1).unwrap_or_default();
                BExpr::Rel(gen_aexpr(cx, rng), RelOp::Eq, gen_aexpr(cx, rng))
            }),
        ),
        (
            1.0,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                cx.negation_limit = cx.negation_limit.checked_sub(1).unwrap_or_default();

                let aexpr = gen_aexpr(cx, rng);

                BExpr::Rel(aexpr.clone(), RelOp::Ne, aexpr)
            }),
        ),
        (
            1.0,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                cx.negation_limit = cx.negation_limit.checked_sub(1).unwrap_or_default();

                let (small, large) = sort_aexpr(gen_aexpr(cx, rng), gen_aexpr(cx, rng));

                BExpr::Rel(small, RelOp::Ge, large)
            }),
        ),
        (
            1.0,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                cx.negation_limit = cx.negation_limit.checked_sub(1).unwrap_or_default();

                let (small, large) = sort_aexpr(gen_aexpr(cx, rng), gen_aexpr(cx, rng));

                BExpr::Rel(small, RelOp::Gt, large)
            }),
        ),
        (
            1.0,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                cx.negation_limit = cx.negation_limit.checked_sub(1).unwrap_or_default();

                let (small, large) = sort_aexpr(gen_aexpr(cx, rng), gen_aexpr(cx, rng));

                BExpr::Rel(large, RelOp::Le, small)
            }),
        ),
        (
            1.0,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                cx.negation_limit = cx.negation_limit.checked_sub(1).unwrap_or_default();

                let (small, large) = sort_aexpr(gen_aexpr(cx, rng), gen_aexpr(cx, rng));

                BExpr::Rel(large, RelOp::Lt, small)
            }),
        ),
    ];

    let mut erng = SmallRng::seed_from_u64(rng.random());

    let choice: BExpr = generation_options
        .choose_weighted(&mut erng, |item| item.0)
        .unwrap()
        .1(cx, &mut erng);

    choice
}

pub fn sort_aexpr(a: AExpr, b: AExpr) -> (AExpr, AExpr) {
    let a_var = aexpr_resolve(a.clone());
    let b_var = aexpr_resolve(b.clone());

    if a_var <= b_var {
        // (smallest, largest)
        (a, b)
    } else {
        (b, a)
    }
}

pub fn aexpr_resolve(a: AExpr) -> i32 {
    match a {
        AExpr::Number(n) => n,
        AExpr::Reference(target) => 0,
        AExpr::Binary(l_aexpr, aop, r_aexpr) => {
            let l_var = aexpr_resolve(l_aexpr.simplify());
            let r_var = aexpr_resolve(r_aexpr.simplify());

            match aop {
                AOp::Plus => l_var + r_var,
                AOp::Minus => l_var - r_var,
                AOp::Times => l_var * r_var,
                AOp::Divide => {
                    if r_var == 0 {
                        return -1;
                    }
                    l_var / r_var
                }
                AOp::Pow => l_var.pow(r_var as u32),
            }
        }
        AExpr::Minus(aexpr) => -aexpr_resolve(aexpr.simplify()),
    }
}

pub fn gen_multiple_guards(
    cx: &mut CompilerContext,
    rng: &mut ErasedRng,
    gnopt: &GenOptionsNested<Commands>,
) -> Vec<Guard> {
    let n = rng.random_range(0..cx.fuel.max(1));

    let guards: Vec<Guard> = (0..n)
        .map(|_| Guard(gen_bexpr(cx, rng), gnopt.generate(cx, rng)))
        .collect();

    guards
}

pub fn gen_undefined_guards(
    cx: &mut CompilerContext,
    rng: &mut ErasedRng,
    gnopt: &GenOptionsNested<Commands>,
) -> Vec<Guard> {
    let n = rng.random_range(0..cx.fuel.max(1));

    let guards: Vec<Guard> = (0..n)
        .map(|_| Guard(gen_bexpr_undefined(cx, rng), gnopt.generate(cx, rng)))
        .collect();

    guards
}

pub fn gen_bexpr_undefined<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> BExpr {
    let generation_options: GenOptions<BExpr> = vec![
        // BExpr::
        // =
        (
            1.0,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                BExpr::Rel(gen_aexpr_op_undefined(cx, rng), RelOp::Eq, AExpr::Number(0))
            }),
        ),
        (
            1.0,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                BExpr::Rel(gen_aexpr_op_undefined(cx, rng), RelOp::Ne, AExpr::Number(0))
            }),
        ),
        // >
        (
            1.0,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                BExpr::Rel(gen_aexpr_op_undefined(cx, rng), RelOp::Gt, AExpr::Number(0))
            }),
        ),
        (
            1.0,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                BExpr::Rel(gen_aexpr_op_undefined(cx, rng), RelOp::Lt, AExpr::Number(0))
            }),
        ),
        // >=
        (
            1.0,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                BExpr::Rel(gen_aexpr_op_undefined(cx, rng), RelOp::Ge, AExpr::Number(0))
            }),
        ),
        (
            1.0,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                BExpr::Rel(gen_aexpr_op_undefined(cx, rng), RelOp::Le, AExpr::Number(0))
            }),
        ),
        // ^
        // &&
        // not
    ];

    let mut erng = SmallRng::seed_from_u64(rng.random());

    let choice: BExpr = generation_options
        .choose_weighted(&mut erng, |item| item.0)
        .unwrap()
        .1(cx, &mut erng);

    choice
}

pub fn gen_aexpr_op_undefined<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> AExpr {
    let generation_options: GenOptions<AExpr> = vec![
        // AExpr::
        // undefined value use
        // +
        (
            0.20,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                cx.recursion_limit = cx.recursion_limit.checked_sub(1).unwrap_or_default();
                AExpr::binary(
                    gen_aexpr_undefined(cx, rng),
                    AOp::Plus,
                    gen_aexpr_undefined(cx, rng),
                )
            }),
        ),
        // -
        (
            0.20,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                cx.recursion_limit = cx.recursion_limit.checked_sub(1).unwrap_or_default();
                AExpr::binary(
                    gen_aexpr_undefined(cx, rng),
                    AOp::Minus,
                    gen_aexpr_undefined(cx, rng),
                )
            }),
        ),
        // *
        (
            0.20,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                cx.recursion_limit = cx.recursion_limit.checked_sub(1).unwrap_or_default();
                AExpr::binary(
                    gen_aexpr_undefined(cx, rng),
                    AOp::Times,
                    gen_aexpr_undefined(cx, rng),
                )
            }),
        ),
        // ^
        (
            0.20,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                cx.recursion_limit = cx.recursion_limit.checked_sub(1).unwrap_or_default();
                AExpr::binary(
                    gen_aexpr_undefined(cx, rng),
                    AOp::Pow,
                    gen_aexpr_undefined(cx, rng),
                )
            }),
        ),
        (
            0.20,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                cx.recursion_limit = cx.recursion_limit.checked_sub(1).unwrap_or_default();
                AExpr::binary(gen_aexpr_undefined(cx, rng), AOp::Divide, AExpr::Number(0))
            }),
        ),
    ];

    let mut erng = SmallRng::seed_from_u64(rng.random());

    let choice: AExpr = generation_options
        .choose_weighted(&mut erng, |item| item.0)
        .unwrap()
        .1(cx, &mut erng);

    choice
}

pub fn gen_aexpr_undefined<R: Rng>(cx: &mut CompilerContext, rng: &mut R) -> AExpr {
    let generation_options: GenOptions<AExpr> = vec![
        (
            0.5,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                let mut var_name = cx.names.choose(rng).cloned().unwrap_or_else(|| "a".into());
                var_name.push_str("0");

                let var_name: String = "test".to_string();

                AExpr::Reference(Target::Variable(Variable(var_name)))
            }),
        ),
        (
            0.5,
            Box::new(|cx: &mut CompilerContext, rng: &mut ErasedRng| {
                let var_name = cx.names.choose(rng).cloned().unwrap_or_else(|| "a".into());

                let var_name: String = "test2".to_string();
                AExpr::Reference(Target::Variable(Variable(var_name)))
            }),
        ),
    ];

    let mut erng = SmallRng::seed_from_u64(rng.random());

    let choice: AExpr = generation_options
        .choose_weighted(&mut erng, |item| item.0)
        .unwrap()
        .1(cx, &mut erng);

    choice
}
