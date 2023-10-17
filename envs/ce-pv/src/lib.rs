extern crate env_logger;

extern crate z3;
use std::convert::TryInto;
use std::ops::Add;
use std::time::Duration;
use z3::ast::{Array as z3array, Ast, Bool, Int as z3int, BV};
use z3::*;

use ce_core::{
    components::StandardLayout, define_env, rand, Env, Generate, RenderProps, ValidationResult,
};
use dioxus::prelude::{SvgAttributes, *};
use gcl::{
    ast::{
        AExpr, AOp, Array, BExpr, Command, Commands, Guard, Int, LogicOp, Predicate, RelOp, Target,
        Variable,
    },
    parse::parse_commands,
};

use serde::{Deserialize, Serialize};
define_env!(PvEnv);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PvInput {
    pre: Predicate,
    post: Predicate,
    cmds: Commands,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PvOutput {}

pub trait Cmds {
    fn ver_con(&self, cond: BExpr) -> Vec<BExpr>;
}

pub trait Cmd {
    fn ver_con(&self, c: BExpr) -> Vec<BExpr>;
}

pub trait Ae {
    fn pretty_print(&self) -> String;
    fn z3_ast<'ctx>(&self, ctx: &'ctx Context) -> z3int<'ctx>;
}

pub trait Be {
    fn pretty_print(&self) -> String;
    fn z3_ast<'ctx>(&self, ctx: &'ctx Context) -> Bool<'ctx>;
}

pub trait GuardE {
    fn ver_con_if(&self, c: BExpr) -> Vec<BExpr>;
    fn ver_con_do(&self, i: Predicate, c: BExpr) -> Vec<BExpr>;
}

impl Env for PvEnv {
    type Input = PvInput;

    type Output = PvOutput;

    fn run(_input: &Self::Input) -> ce_core::Result<Self::Output> {
        Ok(PvOutput {})
    }

    fn validate(_input: &Self::Input, _output: &Self::Output) -> ce_core::Result<ValidationResult> {
        Ok(ValidationResult::CorrectTerminated)
    }

    fn render<'a>(cx: &'a ScopeState, _props: &'a RenderProps<'a, Self>) -> Element<'a> {
        cx.render(rsx!(StandardLayout {
            input: cx.render(rsx!(div {
                class: "grid place-items-center",
                "Input goes here"
            })),
            output: cx.render(rsx!(div {
                class: "grid place-items-center",
                "Output goes here"
            })),
        }))
    }
}

impl Generate for PvInput {
    type Context = ();

    fn gen<R: rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        let cmds = Commands::gen(&mut Default::default(), rng);

        Self {
            pre: todo!(),
            post: todo!(),
            cmds: todo!(),
        }
    }
}

impl Cmds for Commands {
    //Fix this at some point, so we can run unit tests
    fn ver_con(&self, cond: BExpr) -> Vec<BExpr> {
        let mut new_c_v = Vec::new();
        let mut new_c = cond.clone();
        new_c_v.push(new_c);
        for n in (0..self.0.len()).rev() {
            let mut new_c = self.0[n].ver_con(new_c_v.last().unwrap().clone());
            new_c_v.append(&mut new_c);
        }
        new_c_v
    }
}

impl Cmd for Command {
    fn ver_con(&self, c: BExpr) -> Vec<BExpr> {
        let mut new_c_v = Vec::new();
        match self {
            Command::Assignment(x, e) => {
                if c.contains_var(x) {
                    new_c_v.push(c.subst_var(x, e).simplify())
                } else {
                    new_c_v.push(c.clone())
                }
            }
            Command::Skip => new_c_v.push(c.clone()),
            Command::If(guards) => new_c_v.append(&mut guards.ver_con_if(c)),
            Command::EnrichedLoop(predicate, guards) => {
                new_c_v.append(&mut guards.ver_con_do(predicate.clone(), c))
            }
            Command::Loop(_) => unimplemented!(),
            Command::Annotated(_, _, _) => unimplemented!(),
            Command::Continue => unimplemented!(),
            Command::Break => unimplemented!(),
        };
        new_c_v
    }
}

impl GuardE for Vec<Guard> {
    fn ver_con_if(&self, c: BExpr) -> Vec<BExpr> {
        let mut wpv = Vec::new();
        let mut wpvl = Vec::new();
        for n in (0..self.len()).rev() {
            let cond = c.clone();
            let mut left = self[n].1.ver_con(cond);
            let l = left.last().unwrap().clone();
            wpvl.append(&mut left);

            let r = self[n].0.clone();
            wpv.push(BExpr::Logic(Box::new(l), LogicOp::And, Box::new(r)));
        }
        let mut wp = Vec::new();
        let mut wpif = wpv.pop().unwrap();
        for iter in wpv {
            wpif = BExpr::Logic(Box::new(wpif.clone()), LogicOp::Or, Box::new(iter));
        }

        wp.append(&mut wpvl);
        wp.push(wpif);
        wp
    }

    fn ver_con_do(&self, i: Predicate, c: BExpr) -> Vec<BExpr> {
        let mut wpv = Vec::new();
        let mut done = Vec::new();
        let mut qv = Vec::new();
        for n in (0..self.len()).rev() {
            let cond = i.clone();
            let wp = &mut self[n].1.ver_con(cond);
            wpv.append(&mut wp.clone());
            qv.push(wp[0].clone());
            done.push(self[n].0.clone());
        }
        // Do some z3 magic on this mf'er here
        // Like do it right here
        // If everything holds (Inv && Done[GC] -> Q) && (Inv && Guard) -> Weakest Pre [body](Inv), then we're good
        let _ = env_logger::try_init();
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let inv = i.z3_ast(&ctx);
        let solver = Solver::new(&ctx);

        for n in (0..self.len()).rev() {
            solver.assert(
                &ast::Bool::implies(
                    &ast::Bool::and(&ctx, &[&inv, &done[n].z3_ast(&ctx).not()]),
                    &c.z3_ast(&ctx),
                )
                .not(),
            );
            solver.assert(
                &ast::Bool::implies(
                    &ast::Bool::and(&ctx, &[&inv, &self[n].0.z3_ast(&ctx)]),
                    &qv[n].z3_ast(&ctx),
                )
                .not(),
            );
        }
        let result = solver.check();
        match result {
            SatResult::Sat => panic!("Invariant does not hold"),
            SatResult::Unsat => println!("Invariant holds"),
            SatResult::Unknown => panic!("Z3 is confused"),
        }
        wpv.push(i.clone());
        wpv
    }
}

impl Ae for AExpr {
    fn pretty_print(&self) -> String {
        match self {
            AExpr::Number(n) => format!("{}", n),
            AExpr::Reference(x) => format!("{}", x),
            AExpr::Minus(m) => format!("-{}", m.pretty_print()),
            AExpr::Binary(l, op, r) => match op {
                AOp::Plus => format!("({} + {})", l.pretty_print(), r.pretty_print()),
                AOp::Minus => format!("({} - {})", l.pretty_print(), r.pretty_print()),
                AOp::Times => format!("({} * {})", l.pretty_print(), r.pretty_print()),
                AOp::Divide => format!("({} / {})", l.pretty_print(), r.pretty_print()),
                AOp::Pow => format!("({} ^ {})", l.pretty_print(), r.pretty_print()),
            },
            AExpr::Function(_) => todo!(),
        }
    }

    fn z3_ast<'ctx>(&self, ctx: &'ctx Context) -> z3int<'ctx> {
        match self {
            AExpr::Number(n) => {
                let mut a: i64;
                let a = *n;
                ast::Int::from_i64(&ctx, a)
            }
            AExpr::Reference(x) => ast::Int::new_const(&ctx, x.name()),
            AExpr::Minus(m) => -m.z3_ast(&ctx),
            AExpr::Binary(l, op, r) => match op {
                AOp::Plus => ast::Int::add(&ctx, &[&l.z3_ast(&ctx), &r.z3_ast(&ctx)]),
                AOp::Minus => ast::Int::sub(&ctx, &[&l.z3_ast(&ctx), &r.z3_ast(&ctx)]),
                AOp::Times => ast::Int::mul(&ctx, &[&l.z3_ast(&ctx), &r.z3_ast(&ctx)]),
                AOp::Divide => ast::Int::div(&l.z3_ast(&ctx), &r.z3_ast(&ctx)),
                AOp::Pow => todo!(),
            },
            AExpr::Function(f) => todo!(),
        }
    }
}

impl Be for BExpr {
    fn pretty_print(&self) -> String {
        match self {
            BExpr::Logic(l, op, r) => match op {
                LogicOp::And => format!("({} && {})", l.pretty_print(), r.pretty_print()),
                LogicOp::Or => format!("({} || {})", l.pretty_print(), r.pretty_print()),
                LogicOp::Land => format!("({} & {})", l.pretty_print(), r.pretty_print()),
                LogicOp::Lor => format!("({} | {})", l.pretty_print(), r.pretty_print()),
                LogicOp::Implies => format!("({} -> {})", l.pretty_print(), r.pretty_print()),
            },
            BExpr::Rel(l, op, r) => match op {
                RelOp::Eq => format!("({} = {})", l.pretty_print(), r.pretty_print()),
                RelOp::Ne => format!("({} != {})", l.pretty_print(), r.pretty_print()),
                RelOp::Gt => format!("({} > {})", l.pretty_print(), r.pretty_print()),
                RelOp::Ge => format!("({} >= {})", l.pretty_print(), r.pretty_print()),
                RelOp::Lt => format!("({} < {})", l.pretty_print(), r.pretty_print()),
                RelOp::Le => format!("({} <= {})", l.pretty_print(), r.pretty_print()),
            },
            BExpr::Not(b) => format!("!({})", b.pretty_print()),
            BExpr::Bool(b) => format!("{}", b),
            BExpr::Quantified(_, _, _) => unimplemented!(),
        }
    }
    fn z3_ast<'ctx>(&self, ctx: &'ctx Context) -> Bool<'ctx> {
        match self {
            //BExpr::Bool(b) => ast::Bool::<'a>::from_bool(&ctx, b.clone()),
            BExpr::Bool(b) => todo!(),
            BExpr::Rel(l, op, r) => match op {
                RelOp::Eq => ast::Bool::from_bool(&ctx, l.z3_ast(&ctx).eq(&r.z3_ast(&ctx))),
                RelOp::Ge => l.z3_ast(&ctx).ge(&r.z3_ast(&ctx)),
                RelOp::Gt => l.z3_ast(&ctx).gt(&r.z3_ast(&ctx)),
                RelOp::Le => l.z3_ast(&ctx).le(&r.z3_ast(&ctx)),
                RelOp::Lt => l.z3_ast(&ctx).lt(&r.z3_ast(&ctx)),
                RelOp::Ne => ast::Bool::from_bool(&ctx, !l.z3_ast(&ctx).eq(&r.z3_ast(&ctx))),
            },
            BExpr::Logic(l, op, r) => match op {
                LogicOp::And => ast::Bool::and(&ctx, &[&l.z3_ast(&ctx), &r.z3_ast(&ctx)]),
                LogicOp::Implies => ast::Bool::implies(&l.z3_ast(&ctx), &r.z3_ast(&ctx)),
                LogicOp::Land => ast::Bool::and(&ctx, &[&l.z3_ast(&ctx), &r.z3_ast(&ctx)]),
                LogicOp::Lor => ast::Bool::or(&ctx, &[&l.z3_ast(&ctx), &r.z3_ast(&ctx)]),
                LogicOp::Or => ast::Bool::or(&ctx, &[&l.z3_ast(&ctx), &r.z3_ast(&ctx)]),
            },
            BExpr::Not(b) => b.z3_ast(&ctx).not(),
            BExpr::Quantified(_, _, _) => unimplemented!(),
        }
    }
}

#[test]
fn pre_condition_test() {
    let pre = gcl::parse::parse_predicate("n > 5").unwrap();
    let post = gcl::parse::parse_predicate("n=1").unwrap();
    let src = r#"
    n :=1024;
    do {n>=10} n>1 ->
        n:=n/2
    od
    "#;
    let cmds = gcl::parse::parse_commands(src).unwrap();
    let c = cmds.clone();

    let binding = c.ver_con(post);
    let output = binding.last().unwrap().clone();
    for n in (0..binding.len()).rev() {
        println!("{}", binding[n].pretty_print());
    }

    assert_eq!(output, pre);
}
