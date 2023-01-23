use rand::{seq::SliceRandom, Rng};

use crate::ast::{
    AExpr, AOp, Array, BExpr, Command, Commands, Guard, LogicOp, RelOp, Target, Variable,
};

pub struct Context {
    fuel: u32,
    recursion_limit: u32,
    negation_limit: u32,
    names: Vec<String>,
}

impl Context {
    pub fn new<R: Rng>(fuel: u32, _rng: &mut R) -> Self {
        Context {
            fuel,
            recursion_limit: fuel,
            negation_limit: fuel,
            names: ["a", "b", "c", "d"].map(Into::into).to_vec(),
        }
    }

    fn use_array(&self) -> bool {
        false
    }

    fn reference<R: Rng>(&mut self, rng: &mut R) -> Target<Box<AExpr>> {
        self.sample(
            rng,
            vec![
                (0.7, box |cx, rng| {
                    Target::Variable(Variable(cx.names.choose(rng).cloned().unwrap()))
                }),
                (if self.use_array() { 0.3 } else { 0.0 }, box |cx, rng| {
                    Target::Array(
                        Array(cx.names.choose(rng).cloned().unwrap().to_uppercase()),
                        box AExpr::gen(cx, rng),
                    )
                }),
            ],
        )
    }

    fn sample<G: Generate<Context = Self>, R: Rng>(
        &mut self,
        rng: &mut R,
        options: Vec<(f32, Box<dyn Fn(&mut Self, &mut R) -> G>)>,
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

impl Generate for Commands {
    type Context = Context;

    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        Commands(cx.many(1, 10, rng))
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
                (1.0, box |cx, rng| {
                    Command::Assignment(Target::gen(cx, rng), AExpr::gen(cx, rng))
                }),
                (0.3, box |cx, rng| Command::If(cx.many(1, 10, rng))),
                (0.3, box |cx, rng| Command::Loop(cx.many(1, 10, rng))),
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

impl Generate for Box<AExpr> {
    type Context = Context;

    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        box AExpr::gen(cx, rng)
    }
}
impl Generate for AExpr {
    type Context = Context;
    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        cx.sample(
            rng,
            vec![
                (0.4, box |_, rng| AExpr::Number(rng.gen_range(-100..=100))),
                (0.8, box |cx, rng| AExpr::Reference(cx.reference(rng))),
                (
                    if cx.recursion_limit == 0 || cx.fuel == 0 {
                        0.0
                    } else {
                        0.9
                    },
                    box |cx, rng| {
                        cx.recursion_limit = cx.recursion_limit.checked_sub(1).unwrap_or_default();
                        AExpr::Binary(
                            box AExpr::gen(cx, rng),
                            AOp::gen(cx, rng),
                            box AExpr::gen(cx, rng),
                        )
                    },
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
                (0.5, box |_, _| AOp::Plus),
                (0.4, box |_, _| AOp::Minus),
                (0.4, box |_, _| AOp::Times),
                (0.1, box |_, _| AOp::Pow),
                (0.3, box |_, _| AOp::Divide),
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
                (0.2, box |_cx, rng| BExpr::Bool(rng.gen())),
                (
                    if cx.recursion_limit == 0 { 0.0 } else { 0.7 },
                    box |cx, rng| {
                        cx.recursion_limit = cx.recursion_limit.checked_sub(1).unwrap_or_default();
                        BExpr::Rel(
                            AExpr::gen(cx, rng),
                            RelOp::gen(cx, rng),
                            AExpr::gen(cx, rng),
                        )
                    },
                ),
                (
                    if cx.recursion_limit == 0 { 0.0 } else { 0.7 },
                    box |cx, rng| {
                        cx.recursion_limit = cx.recursion_limit.checked_sub(1).unwrap_or_default();
                        BExpr::Logic(
                            box BExpr::gen(cx, rng),
                            LogicOp::gen(cx, rng),
                            box BExpr::gen(cx, rng),
                        )
                    },
                ),
                (
                    if cx.negation_limit == 0 { 0.0 } else { 0.4 },
                    box |cx, rng| {
                        cx.negation_limit = cx.negation_limit.checked_sub(1).unwrap_or_default();
                        BExpr::Not(box BExpr::gen(cx, rng))
                    },
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
                (0.3, box |_cx, _rng| RelOp::Eq),
                (0.3, box |_cx, _rng| RelOp::Gt),
                (0.3, box |_cx, _rng| RelOp::Ge),
                (0.3, box |_cx, _rng| RelOp::Lt),
                (0.3, box |_cx, _rng| RelOp::Le),
                (0.3, box |_cx, _rng| RelOp::Ne),
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
                (0.3, box |_cx, _rng| LogicOp::And),
                (0.3, box |_cx, _rng| LogicOp::Land),
                (0.3, box |_cx, _rng| LogicOp::Or),
                (0.3, box |_cx, _rng| LogicOp::Lor),
            ],
        )
    }
}
