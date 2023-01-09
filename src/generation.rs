use rand::{seq::SliceRandom, Rng};

use crate::ast::{AExpr, AOp, Array, BExpr, Command, Guard, RelOp, Variable};

pub struct Context {
    fuel: u32,
    names: Vec<String>,
}

impl Context {
    pub fn new<R: Rng>(fuel: u32, _rng: &mut R) -> Self {
        Context {
            fuel,
            names: ["a", "b", "c"].map(Into::into).to_vec(),
        }
    }

    fn array_name<R: Rng>(&self, rng: &mut R) -> String {
        self.names.choose(rng).cloned().unwrap().to_uppercase()
    }

    fn var_name<R: Rng>(&self, rng: &mut R) -> String {
        self.names.choose(rng).cloned().unwrap()
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

impl Generate for Command {
    type Context = Context;
    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        cx.sample(
            rng,
            vec![
                (0.7, box |cx, rng| {
                    Command::Assignment(Variable::gen(cx, rng), AExpr::gen(cx, rng))
                }),
                (0.3, box |cx, rng| {
                    Command::ArrayAssignment(Array::gen(cx, rng), AExpr::gen(cx, rng))
                }),
                (0.3, box |cx, rng| Command::If(cx.many(1, 10, rng))),
                (0.3, box |cx, rng| Command::Loop(cx.many(1, 10, rng))),
            ],
        )
    }
}

impl Generate for Guard {
    type Context = Context;

    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        Guard(Generate::gen(cx, rng), cx.many(1, 10, rng))
    }
}

impl Generate for Array {
    type Context = Context;
    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        Array(cx.array_name(rng), box AExpr::gen(cx, rng))
    }
}

impl Generate for Variable {
    type Context = Context;
    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        Variable(cx.var_name(rng))
    }
}

impl Generate for AExpr {
    type Context = Context;
    fn gen<R: Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        cx.sample(
            rng,
            vec![
                (0.4, box |_, rng| AExpr::Number(rng.gen_range(-100..=100))),
                (0.7, box |cx, rng| AExpr::Variable(cx.var_name(rng))),
                // box |cx, rng| AExpr::Array(Array::gen(cx, rng)),
                (if cx.fuel == 0 { 0.0 } else { 0.5 }, box |cx, rng| {
                    AExpr::Binary(
                        box AExpr::gen(cx, rng),
                        AOp::gen(cx, rng),
                        box AExpr::gen(cx, rng),
                    )
                }),
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
                (0.7, box |cx, rng| {
                    BExpr::Rel(
                        AExpr::gen(cx, rng),
                        RelOp::gen(cx, rng),
                        AExpr::gen(cx, rng),
                    )
                }),
                (0.7, box |cx, rng| {
                    BExpr::And(box BExpr::gen(cx, rng), box BExpr::gen(cx, rng))
                }),
                (0.4, box |cx, rng| BExpr::Not(box BExpr::gen(cx, rng))),
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
            ],
        )
    }
}
