extern crate env_logger;

extern crate z3;
use ce_core::components::GclEditor;

use dioxus::html::tr;
use gcl::ast::Function;

use z3::ast::{Bool, Int as z3int};
use z3::*;

use ce_core::{
    components::StandardLayout, define_env, rand, Env, Generate, RenderProps, ValidationResult,
};
use dioxus::prelude::*;
use gcl::ast::{AExpr, AOp, BExpr, Command, Commands, Guard, LogicOp, Predicate, RelOp};

use serde::{Deserialize, Serialize};
define_env!(PvEnv);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PvInput {
    pre: Predicate,
    post: Predicate,
    cmds: Commands,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PvOutput {
    conds: Vec<BExpr>,
    checks: Vec<BExpr>,
}

pub trait Cmds {
    fn ver_con(&self, cond: BExpr) -> PvOutput;
}

pub trait Cmd {
    fn ver_con(&self, c: BExpr) -> PvOutput;
}

pub trait Ae {
    fn z3_ast<'ctx>(&self, ctx: &'ctx Context) -> z3int<'ctx>;
}

pub trait Be {
    fn z3_ast<'ctx>(&self, ctx: &'ctx Context) -> Bool<'ctx>;
}

pub trait GuardE {
    fn ver_con_if(&self, c: BExpr) -> Vec<BExpr>;
    fn ver_con_do(&self, i: Predicate, c: BExpr) -> PvOutput;
}

pub trait Fun {
    fn z3_ast<'ctx>(&self, ctx: &'ctx Context) -> z3int<'ctx>;
    fn ver_con(&self, c: BExpr) -> Vec<BExpr>;
}

impl Env for PvEnv {
    type Input = PvInput;

    type Output = PvOutput;

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        let post = input.post.clone();
        let cmds = input.cmds.clone();
        let op = cmds.ver_con(post);
        Ok(PvOutput {
            conds: op.conds,
            checks: op.checks,
        })
    }

    fn validate(input: &Self::Input, output: &Self::Output) -> ce_core::Result<ValidationResult> {
        let _ = env_logger::try_init();
        let mut res = true;
        if output.checks.len() > 0 {
            for check in output.checks.clone() {
                let cfg = Config::new();
                let ctx = Context::new(&cfg);
                let solver = Solver::new(&ctx);
                solver.assert(&check.z3_ast(&ctx));
                res = match solver.check() {
                    SatResult::Sat => false,
                    SatResult::Unsat => true,
                    SatResult::Unknown => false,
                };

                if res == false {
                    break;
                }
            }
        }
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let solver = Solver::new(&ctx);
        let pre = input.pre.clone().z3_ast(&ctx);
        let pre_new = output.conds.last().unwrap().clone().z3_ast(&ctx);
        solver.assert(&ast::Bool::implies(&pre, &pre_new).not());
        match solver.check() {
            SatResult::Sat => Ok(ValidationResult::IncorretPostcondition),
            SatResult::Unsat => match res {
                true => Ok(ValidationResult::CorrectTerminated),
                false => Ok(ValidationResult::IncorrectInvariant),
            },
            SatResult::Unknown => Ok(ValidationResult::CannotBeValidated),
        }
    }

    fn render<'a>(cx: &'a ScopeState, props: &'a RenderProps<'a, Self>) -> Element<'a> {
        cx.render(rsx!(StandardLayout {
            input: cx.render(rsx!(GclEditor {
                commands: props.input().cmds.clone(),
                on_change: move |cmds| props.set_input(PvInput {
                    pre: BExpr::Bool(true),
                    post: BExpr::Bool(true),
                    cmds
                }),
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
            pre: BExpr::Bool(true),
            post: BExpr::Bool(true),
            cmds: cmds,
        }
    }
}

impl Cmds for Commands {
    //Fix this at some point, so we can run unit tests
    fn ver_con(&self, cond: BExpr) -> PvOutput {
        let new_c = cond.clone();
        let mut op = PvOutput {
            conds: Vec::new(),
            checks: Vec::new(),
        };
        op.conds.push(new_c);
        for n in (0..self.0.len()).rev() {
            let mut p = self.0[n].ver_con(op.conds.last().unwrap().clone());
            op.conds.append(&mut p.conds);
            op.checks.append(&mut p.checks)
        }
        op
    }
}

impl Cmd for Command {
    fn ver_con(&self, c: BExpr) -> PvOutput {
        let mut op = PvOutput {
            conds: Vec::new(),
            checks: Vec::new(),
        };

        match self {
            Command::Assignment(x, e) => {
                if c.contains_var(x) {
                    op.conds.push(c.subst_var(x, e).reduce().simplify())
                } else {
                    op.conds.push(c.clone().reduce().simplify())
                }
            }
            Command::Skip => op.conds.push(c.clone().reduce().simplify()),
            Command::If(guards) => op.conds.append(&mut guards.ver_con_if(c)),
            Command::EnrichedLoop(predicate, guards) => {
                let mut p = guards.ver_con_do(predicate.clone(), c);
                op.conds.append(&mut p.conds);
                op.checks.append(&mut p.checks);
            }
            Command::Loop(_) => unimplemented!(),
            Command::Annotated(_, _, _) => unimplemented!(),
            Command::Continue => unimplemented!(),
            Command::Break => unimplemented!(),
        };
        op
    }
}

impl GuardE for Vec<Guard> {
    fn ver_con_if(&self, c: BExpr) -> Vec<BExpr> {
        let mut wpv = Vec::new();
        let mut wpvl = Vec::new();
        for n in (0..self.len()).rev() {
            let cond = c.clone();
            let mut left = self[n].1.ver_con(cond);
            let l = left.conds.last().unwrap().clone();
            wpvl.append(&mut left.conds);

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

    fn ver_con_do(&self, i: Predicate, c: BExpr) -> PvOutput {
        let mut wpv = Vec::new();
        let mut qv = Vec::new();
        let mut op = PvOutput {
            conds: Vec::new(),
            checks: Vec::new(),
        };
        for n in (0..self.len()).rev() {
            let cond = i.clone();
            let wp = &mut self[n].1.ver_con(cond);
            let wpb = wp.conds.last().clone().unwrap();
            wpv.append(&mut wp.conds.clone());
            qv.push(wpb.clone());
            let and = BExpr::Logic(
                Box::new(i.clone()),
                LogicOp::And,
                Box::new(BExpr::Not(Box::new(self[n].0.clone()))),
            );
            let imp = BExpr::Not(Box::new(BExpr::Logic(
                Box::new(and),
                LogicOp::Implies,
                Box::new(c.clone()),
            )));
            op.checks.push(imp);
            let and = BExpr::Logic(
                Box::new(i.clone()),
                LogicOp::And,
                Box::new(self[n].0.clone()),
            );
            let imp = BExpr::Not(Box::new(BExpr::Logic(
                Box::new(and),
                LogicOp::Implies,
                Box::new(wpb.clone()),
            )));
            op.checks.push(imp);
        }

        wpv.push(i.clone());
        op.conds.append(&mut wpv);
        op
    }
}

impl Ae for AExpr {
    fn z3_ast<'ctx>(&self, ctx: &'ctx Context) -> z3int<'ctx> {
        match self {
            AExpr::Number(n) => ast::Int::from_i64(&ctx, *n),
            AExpr::Reference(x) => ast::Int::new_const(&ctx, x.name()),
            AExpr::Minus(m) => -m.z3_ast(&ctx),
            AExpr::Binary(l, op, r) => match op {
                AOp::Plus => ast::Int::add(&ctx, &[&l.z3_ast(&ctx), &r.z3_ast(&ctx)]),
                AOp::Minus => ast::Int::sub(&ctx, &[&l.z3_ast(&ctx), &r.z3_ast(&ctx)]),
                AOp::Times => ast::Int::mul(&ctx, &[&l.z3_ast(&ctx), &r.z3_ast(&ctx)]),
                AOp::Divide => ast::Int::div(&l.z3_ast(&ctx), &r.z3_ast(&ctx)),
                AOp::Pow => todo!(),
            },
            AExpr::Function(_f) => todo!(),
        }
    }
}

impl Be for BExpr {
    fn z3_ast<'ctx>(&self, ctx: &'ctx Context) -> Bool<'ctx> {
        match self {
            BExpr::Bool(b) => Bool::from_bool(&ctx, *b),
            BExpr::Rel(l, op, r) => match op {
                RelOp::Eq => ast::Bool::and(
                    &ctx,
                    &[
                        &l.z3_ast(&ctx).ge(&r.z3_ast(&ctx)),
                        &l.z3_ast(&ctx).le(&r.z3_ast(&ctx)),
                    ],
                ),
                RelOp::Ge => l.z3_ast(&ctx).ge(&r.z3_ast(&ctx)),
                RelOp::Gt => l.z3_ast(&ctx).gt(&r.z3_ast(&ctx)),
                RelOp::Le => l.z3_ast(&ctx).le(&r.z3_ast(&ctx)),
                RelOp::Lt => l.z3_ast(&ctx).lt(&r.z3_ast(&ctx)),
                RelOp::Ne => ast::Bool::and(
                    &ctx,
                    &[
                        &l.z3_ast(&ctx).ge(&r.z3_ast(&ctx)),
                        &l.z3_ast(&ctx).le(&r.z3_ast(&ctx)),
                    ],
                )
                .not(),
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

impl Fun for Function {
    fn ver_con(&self, _c: BExpr) -> Vec<BExpr> {
        match self {
            Function::Division(_, _) => todo!(),
            Function::Min(_, _) => todo!(),
            Function::Max(_, _) => todo!(),
            Function::Count(_, _) => unimplemented!(),
            Function::LogicalCount(_, _) => unimplemented!(),
            Function::Length(_) => unimplemented!(),
            Function::LogicalLength(_) => unimplemented!(),
            Function::Fac(_) => todo!(),
            Function::Fib(_) => todo!(),
        }
    }

    fn z3_ast<'ctx>(&self, ctx: &'ctx Context) -> z3int<'ctx> {
        match self {
            Function::Division(n, d) => n.z3_ast(&ctx).div(&d.z3_ast(&ctx)),
            Function::Min(_, _) => todo!(),
            Function::Max(_, _) => todo!(),
            Function::Count(_, _) => todo!(),
            Function::LogicalCount(_, _) => todo!(),
            Function::Length(_) => todo!(),
            Function::LogicalLength(_) => todo!(),
            Function::Fac(_) => todo!(),
            Function::Fib(_) => todo!(),
        }
    }
}

/////////////////////////////////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////
/////////////////////////////////// TESTS ///////////////////////////////////
/////////////////////////////////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////

#[test]
fn pre_condition_test1() {
    let pr = gcl::parse::parse_predicate("n>0").unwrap();
    let po = gcl::parse::parse_predicate("n>2").unwrap();
    let src = r#"
    n:=n+1;
    n:=n+1
    "#;
    let coms = gcl::parse::parse_commands(src).unwrap();
    let inp = PvInput {
        pre: pr,
        post: po.clone(),
        cmds: coms.clone(),
    };
    let out = coms.ver_con(po);
    for n in (0..out.conds.len()).rev() {
        println!("{}", out.conds[n]);
    }
    match PvEnv::validate(&inp, &out).unwrap() {
        ValidationResult::CorrectTerminated => assert!(!true),
        _ => assert!(false),
    }
}

#[test]
fn pre_condition_test2() {
    let pr = gcl::parse::parse_predicate("x>=0 && y>=0").unwrap();
    let po = gcl::parse::parse_predicate("z=5").unwrap();
    let src = r#"
        x:=3;
        y:=2;
        z:=x+y
    "#;
    let coms = gcl::parse::parse_commands(src).unwrap();
    let inp = PvInput {
        pre: pr,
        post: po.clone(),
        cmds: coms.clone(),
    };
    let out = coms.ver_con(po);
    for n in (0..out.conds.len()).rev() {
        println!("{}", out.conds[n]);
    }
    match PvEnv::validate(&inp, &out).unwrap() {
        ValidationResult::CorrectTerminated => assert!(true),
        _ => assert!(false),
    }
}

#[test]
fn pre_condition_test3() {
    let pr = gcl::parse::parse_predicate("x>=0 && y<0").unwrap();
    let po = gcl::parse::parse_predicate("x=y").unwrap();
    let src = r#"
        if (x<y) -> x:=y
        [] (x>y) -> y:=x
        [] (x=y) -> skip
        fi
    "#;
    let coms = gcl::parse::parse_commands(src).unwrap();
    let inp = PvInput {
        pre: pr,
        post: po.clone(),
        cmds: coms.clone(),
    };
    let out = coms.ver_con(po);
    for n in (0..out.conds.len()).rev() {
        println!("{}", out.conds[n]);
    }
    match PvEnv::validate(&inp, &out).unwrap() {
        ValidationResult::CorrectTerminated => assert!(true),
        _ => assert!(false),
    }
}

#[test]
fn pre_condition_test4() {
    let pr = gcl::parse::parse_predicate("true").unwrap();
    let po = gcl::parse::parse_predicate("M=res*N+m").unwrap();
    let src = r#"
        res:=0;
        m:=M;
        do {M=res*N+m} m>=N ->
        m:=m-N;
        res:=res+1
        od
    "#;
    let coms = gcl::parse::parse_commands(src).unwrap();
    let inp = PvInput {
        pre: pr,
        post: po.clone(),
        cmds: coms.clone(),
    };
    let out = coms.ver_con(po);
    for n in (0..out.conds.len()).rev() {
        println!("{}", out.conds[n]);
    }
    match PvEnv::validate(&inp, &out).unwrap() {
        ValidationResult::CorrectTerminated => assert!(true),
        _ => assert!(false),
    }
}

#[test]
fn pre_condition_test5() {
    let pr = gcl::parse::parse_predicate("true").unwrap();
    let po = gcl::parse::parse_predicate("n=0").unwrap();
    let src = r#"
        n:=12;
        do {n>=0} n>0 ->
        n:=n-1
        od
    "#;
    let coms = gcl::parse::parse_commands(src).unwrap();
    let inp = PvInput {
        pre: pr,
        post: po.clone(),
        cmds: coms.clone(),
    };
    let out = coms.ver_con(po);
    for n in (0..out.conds.len()).rev() {
        println!("{}", out.conds[n]);
    }
    match PvEnv::validate(&inp, &out).unwrap() {
        ValidationResult::CorrectTerminated => assert!(true),
        _ => assert!(false),
    }
}

#[test]
fn pre_condition_test6() {
    let pr = gcl::parse::parse_predicate("true").unwrap();
    let po = gcl::parse::parse_predicate("n=1").unwrap();
    let src = r#"
        n:=1024;
        do {n>=1} n>1 ->
        n:=n/2
        od
    "#;
    let coms = gcl::parse::parse_commands(src).unwrap();
    let inp = PvInput {
        pre: pr,
        post: po.clone(),
        cmds: coms.clone(),
    };
    let out = coms.ver_con(po);
    for n in (0..out.conds.len()).rev() {
        println!("{}", out.conds[n]);
    }
    match PvEnv::validate(&inp, &out).unwrap() {
        ValidationResult::CorrectTerminated => assert!(true),
        _ => assert!(false),
    }
}

#[test]
fn pre_condition_test7() {
    let pr = gcl::parse::parse_predicate("0<=y").unwrap();
    let po = gcl::parse::parse_predicate("x=y").unwrap();
    let src = r#"
        z:=10;
        x:=0;
        do {x<=y} x<y ->
            if (y<z) -> y:=z
            [] (y>=z) -> skip
            fi;
            x:=x+1
        od
    "#;
    let coms = gcl::parse::parse_commands(src).unwrap();
    let inp = PvInput {
        pre: pr,
        post: po.clone(),
        cmds: coms.clone(),
    };
    let out = coms.ver_con(po);
    for n in (0..out.conds.len()).rev() {
        println!("{}", out.conds[n]);
    }
    match PvEnv::validate(&inp, &out).unwrap() {
        ValidationResult::CorrectTerminated => assert!(true),
        _ => assert!(false),
    }
}

#[test]
fn pre_condition_test8() {
    let pr = gcl::parse::parse_predicate("true").unwrap();
    let po = gcl::parse::parse_predicate("x=y").unwrap();
    let src = r#"
        if x>y ->
            do {x>=y} x>y ->
                y:=y+1
            od
        [] x<y -> 
            do {x<=y} x<y ->
                x:= x+1
            od
        [] x=y ->
            skip
        fi
    "#;
    let coms = gcl::parse::parse_commands(src).unwrap();
    let inp = PvInput {
        pre: pr,
        post: po.clone(),
        cmds: coms.clone(),
    };
    let out = coms.ver_con(po);
    for n in (0..out.conds.len()).rev() {
        println!("{}", out.conds[n]);
    }
    match PvEnv::validate(&inp, &out).unwrap() {
        ValidationResult::CorrectTerminated => assert!(true),
        _ => assert!(false),
    }
}
