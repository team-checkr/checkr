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
    fn ver_con(&self, c: BExpr) -> BExpr;
}

pub trait Ae {
    fn pretty_print(&self) -> String;
}

pub trait Be {
    fn pretty_print(&self) -> String;
}

pub trait GuardE {
    fn ver_con_if(&self, c: BExpr) -> BExpr;
    fn ver_con_do(&self, i: Predicate, c: BExpr) -> BExpr;
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
        let new_c = cond.clone();
        new_c_v.push(new_c);
        for n in (0..self.0.len()).rev() {
            let new_c = self.0[n].ver_con(new_c_v.last().unwrap().clone());
            new_c_v.push(new_c);
        }
        new_c_v
    }
}

impl Cmd for Command {
    fn ver_con(&self, c: BExpr) -> BExpr {
        match self {
            Command::Assignment(x, e) => {
                if c.contains_var(x) {
                    c.subst_var(x, e).simplify()
                } else {
                    c.clone()
                }
            }
            Command::Skip => c.clone(),
            Command::If(guards) => guards.ver_con_if(c),
            Command::EnrichedLoop(predicate, guards) => guards.ver_con_do(predicate.clone(), c),
            Command::Loop(_) => unimplemented!(),
            Command::Annotated(_, _, _) => unimplemented!(),
            Command::Continue => unimplemented!(),
            Command::Break => unimplemented!(),
        }
    }
}

impl GuardE for Vec<Guard> {
    fn ver_con_if(&self, c: BExpr) -> BExpr {
        let mut wpv = Vec::new();
        for n in (0..self.len()).rev() {
            let cond = c.clone();
            let l = self[n].1.ver_con(cond).last().unwrap().clone();
            let r = self[n].0.clone();
            wpv.push(BExpr::Logic(Box::new(l), LogicOp::And, Box::new(r)));
        }
        let mut wp = wpv.pop().unwrap();
        for iter in wpv {
            wp = BExpr::Logic(Box::new(wp), LogicOp::Or, Box::new(iter));
        }
        wp
    }

    fn ver_con_do(&self, i: Predicate, c: BExpr) -> BExpr {
        let mut wpv = Vec::new();
        for n in (0..self.len()).rev() {
            let cond = c.clone();
            wpv.push(self[n].1.ver_con(cond).last().unwrap().clone());
        }
        // Do some z3 magic on this mf'er here
        // Like do it right here
        // If everything holds (Inv && Done[GC] -> Q) && (Inv && Guard) -> Weakest Pre [body](Inv), then we're good
        i
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
}

#[test]
fn pre_condition_test() {
    let pre = gcl::parse::parse_predicate("n > 5").unwrap();
    let post = gcl::parse::parse_predicate("x>=50").unwrap();
    let src = r#"x:=10+x;
    x:=x*5"#;
    let cmds = gcl::parse::parse_commands(src).unwrap();
    let c = cmds.clone();

    let binding = c.ver_con(post);
    let output = binding.last().unwrap().clone();
    println!("{}", output.simplify().pretty_print());
    assert_eq!(output, pre);
}
