use rand::{seq::SliceRandom, Rng};

use crate::ast::{
    AExpr, AOp, Array, BExpr, Command, Commands, Guard, LogicOp, RelOp, Target, Variable,
};

pub struct Context {
    fuel: u32,
    recursion_limit: u32,
    negation_limit: u32,
    no_loops: bool,
    no_division: bool,
    no_unary_minus: bool,
    names: Vec<String>,
}

type GenerationOptions<R, Ctx, G> = Vec<(f32, Box<dyn Fn(&mut Ctx, &mut R) -> G>)>;

impl Context {
    pub fn new<R: Rng>(fuel: u32, _rng: &mut R) -> Self {
        Context {
            fuel,
            recursion_limit: fuel,
            negation_limit: fuel,
            no_loops: false,
            no_division: false,
            no_unary_minus: false,
            names: ["a", "b", "c", "d"].map(Into::into).to_vec(),
        }
    }

    pub fn set_no_loop(&mut self, no_loops: bool) -> &mut Self {
        self.no_loops = no_loops;
        self
    }
    pub fn set_no_division(&mut self, no_division: bool) -> &mut Self {
        self.no_division = no_division;
        self
    }
    pub fn set_no_unary_minus(&mut self, no_unary_minus: bool) -> &mut Self {
        self.no_unary_minus = no_unary_minus;
        self
    }

    fn use_array(&self) -> bool {
        false
    }

    fn reference<R: Rng>(&mut self, rng: &mut R) -> Target<Box<AExpr>> {
        self.sample(
            rng,
            vec![
                (
                    0.7,
                    Box::new(|cx, rng| {
                        Target::Variable(Variable(cx.names.choose(rng).cloned().unwrap()))
                    }),
                ),
                (
                    if self.use_array() { 0.3 } else { 0.0 },
                    Box::new(|cx, rng| {
                        Target::Array(
                            Array(cx.names.choose(rng).cloned().unwrap().to_uppercase()),
                            Box::new(AExpr::gen(cx, rng)),
                        )
                    }),
                ),
            ],
        )
    }

    fn sample<G: Generate<Context = Self>, R: Rng>(
        &mut self,
        rng: &mut R,
        options: GenerationOptions<R, Self, G>,
    ) -> G {
        let f = options.choose_weighted(rng, |o| o.0).unwrap();
        f.1(self, rng)
    }

    pub fn many<G: Generate<Context = Self>, R: Rng>(
        &mut self,
        min: usize,
        max: usize,
        rng: &mut R,
    ) -> Vec<G> {
        let max = max.min(self.fuel as _).max(min);
        let n = rng.gen_range(min..=max);
        if self.fuel < n as _ {
            self.fuel = 0;
        } else {
            self.fuel -= n as u32;
        }
        (0..n).map(|_| G::gen(self, rng)).collect()
    }
}

pub trait Generate {
    type Context;
    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self;
}

impl<T> Generate for Box<T>
where
    T: Generate,
{
    type Context = T::Context;

    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        Box::new(T::gen(cx, rng))
    }
}

impl Generate for Commands {
    type Context = Context;

    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        Commands(cx.many(1, 10, rng))
    }
}

pub fn annotate_cmds<R: Rng>(mut cmds: Commands, rng: &mut R) -> Command {
    use crate::{
        env::{
            sign::{SignAnalysisInput, SignEnv},
            Environment,
        },
        sign::{Memory, Sign, Signs},
    };
    use std::collections::HashSet;

    let input = SignAnalysisInput::gen(&mut cmds, rng);
    let sign_result = SignEnv
        .run(&cmds, &input)
        .expect("the input was just generated, so it should be valid");

    let pre = signs_in(&sign_result.nodes[&sign_result.initial_node]);
    let post = signs_in(&sign_result.nodes[&sign_result.final_node]);

    return Command::Annotated(pre, cmds, post);

    fn signs_in(assignment: &HashSet<Memory<Sign, Signs>>) -> BExpr {
        assignment
            .iter()
            .filter_map(|world| {
                world
                    .variables
                    .iter()
                    .map(|(v, s)| {
                        let v = AExpr::Reference(v.clone().into());
                        let op = match s {
                            Sign::Positive => RelOp::Gt,
                            Sign::Zero => RelOp::Eq,
                            Sign::Negative => RelOp::Lt,
                        };
                        BExpr::Rel(v, op, AExpr::Number(0))
                    })
                    .reduce(|a, b| BExpr::logic(a, LogicOp::And, b))
            })
            .reduce(|a, b| BExpr::logic(a, LogicOp::Or, b))
            .unwrap_or(BExpr::Bool(true))
    }
}

impl Generate for Command {
    type Context = Context;
    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        cx.recursion_limit = 5;
        cx.negation_limit = 3;
        cx.sample(
            rng,
            vec![
                (
                    1.0,
                    Box::new(|cx, rng| {
                        Command::Assignment(Target::gen(cx, rng), AExpr::gen(cx, rng))
                    }),
                ),
                (0.6, Box::new(|cx, rng| Command::If(cx.many(1, 10, rng)))),
                (
                    if cx.no_loops { 0.0 } else { 0.3 },
                    Box::new(|cx, rng| Command::Loop(cx.many(1, 10, rng))),
                ),
            ],
        )
    }
}

impl Generate for Target<Box<AExpr>> {
    type Context = Context;

    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        cx.reference(rng)
    }
}

impl Generate for Guard {
    type Context = Context;

    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        cx.recursion_limit = 5;
        cx.negation_limit = 3;
        Guard(Generate::gen(cx, rng), Commands::gen(cx, rng))
    }
}

impl Generate for AExpr {
    type Context = Context;
    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        cx.sample(
            rng,
            vec![
                (
                    0.4,
                    Box::new(|_, rng| AExpr::Number(rng.gen_range(-100..=100))),
                ),
                (0.8, Box::new(|cx, rng| AExpr::Reference(cx.reference(rng)))),
                (
                    if cx.recursion_limit == 0 || cx.fuel == 0 {
                        0.0
                    } else {
                        0.9
                    },
                    Box::new(|cx, rng| {
                        cx.recursion_limit = cx.recursion_limit.checked_sub(1).unwrap_or_default();
                        AExpr::binary(AExpr::gen(cx, rng), AOp::gen(cx, rng), AExpr::gen(cx, rng))
                    }),
                ),
            ],
        )
    }
}

impl Generate for AOp {
    type Context = Context;

    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        cx.sample(
            rng,
            vec![
                (0.5, Box::new(|_, _| AOp::Plus)),
                (0.4, Box::new(|_, _| AOp::Minus)),
                (0.4, Box::new(|_, _| AOp::Times)),
                (0.1, Box::new(|_, _| AOp::Pow)),
                (
                    if cx.no_division { 0.0 } else { 0.3 },
                    Box::new(|_, _| AOp::Divide),
                ),
            ],
        )
    }
}

impl Generate for BExpr {
    type Context = Context;

    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        cx.sample(
            rng,
            vec![
                (0.2, Box::new(|_cx, rng| BExpr::Bool(rng.gen()))),
                (
                    if cx.recursion_limit == 0 { 0.0 } else { 0.7 },
                    Box::new(|cx, rng| {
                        cx.recursion_limit = cx.recursion_limit.checked_sub(1).unwrap_or_default();
                        BExpr::Rel(
                            AExpr::gen(cx, rng),
                            RelOp::gen(cx, rng),
                            AExpr::gen(cx, rng),
                        )
                    }),
                ),
                (
                    if cx.recursion_limit == 0 { 0.0 } else { 0.7 },
                    Box::new(|cx, rng| {
                        cx.recursion_limit = cx.recursion_limit.checked_sub(1).unwrap_or_default();
                        BExpr::logic(
                            BExpr::gen(cx, rng),
                            LogicOp::gen(cx, rng),
                            BExpr::gen(cx, rng),
                        )
                    }),
                ),
                (
                    if cx.negation_limit == 0 { 0.0 } else { 0.4 },
                    Box::new(|cx, rng| {
                        cx.negation_limit = cx.negation_limit.checked_sub(1).unwrap_or_default();
                        BExpr::Not(Box::new(BExpr::gen(cx, rng)))
                    }),
                ),
            ],
        )
    }
}

impl Generate for RelOp {
    type Context = Context;

    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        cx.sample(
            rng,
            vec![
                (0.3, Box::new(|_cx, _rng| RelOp::Eq)),
                (0.3, Box::new(|_cx, _rng| RelOp::Gt)),
                (0.3, Box::new(|_cx, _rng| RelOp::Ge)),
                (0.3, Box::new(|_cx, _rng| RelOp::Lt)),
                (0.3, Box::new(|_cx, _rng| RelOp::Le)),
                (0.3, Box::new(|_cx, _rng| RelOp::Ne)),
            ],
        )
    }
}
impl Generate for LogicOp {
    type Context = Context;

    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        cx.sample(
            rng,
            vec![
                (0.3, Box::new(|_cx, _rng| LogicOp::And)),
                (0.3, Box::new(|_cx, _rng| LogicOp::Land)),
                (0.3, Box::new(|_cx, _rng| LogicOp::Or)),
                (0.3, Box::new(|_cx, _rng| LogicOp::Lor)),
            ],
        )
    }
}
