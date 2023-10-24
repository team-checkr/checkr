extern crate env_logger;

extern crate z3;
use dioxus::html::form;
use gcl::ast::Function;
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

pub trait Fun {
    fn pretty_print(&self) -> String;
    fn z3_ast<'ctx>(&self, ctx: &'ctx Context) -> z3int<'ctx>;
    fn ver_con(&self, c: BExpr) -> Vec<BExpr>;
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
        let mut qv = Vec::new();
        for n in (0..self.len()).rev() {
            let cond = i.clone();
            let wp = &mut self[n].1.ver_con(cond);
            let mut wpb = wp.last().clone().unwrap();
            wpv.append(&mut wp.clone());
            qv.push(wpb.clone());
        }
        // Do some z3 magic on this mf'er here
        // Like do it right here
        // If everything holds (Inv && Done[GC] -> Q) && (Inv && Guard) -> Weakest Pre [body](Inv), then we're good
        let _ = env_logger::try_init();

        for n in (0..self.len()).rev() {
            let cfg = Config::new();
            let ctx = Context::new(&cfg);
            let solver = Solver::new(&ctx);
            let inv = i.z3_ast(&ctx);
            let and = &ast::Bool::and(&ctx, &[&inv, &self[n].0.z3_ast(&ctx).not()]);
            let imp = &ast::Bool::implies(&and, &c.z3_ast(&ctx)).not();
            solver.assert(&and);

            let result1 = solver.check();
            match result1 {
                SatResult::Sat => {
                    solver.assert(&imp);
                    let result = solver.check();
                    match result {
                        SatResult::Sat => panic!("Invariant and Done[GC] does not imply Q"),
                        SatResult::Unsat => println!("Invariant and Done[GC] implies Q"),
                        SatResult::Unknown => panic!("Z3 is confused"),
                    }
                }
                SatResult::Unsat => println!("Invariant and Done[GC] is unsatisfiable"),
                SatResult::Unknown => panic!("Z3 is confused"),
            }
            let cfg = Config::new();
            let ctx = Context::new(&cfg);
            let solver = Solver::new(&ctx);
            let inv = i.z3_ast(&ctx);
            let and = &ast::Bool::and(&ctx, &[&inv, &self[n].0.z3_ast(&ctx)]);
            let imp = &ast::Bool::implies(&and, &qv[n].z3_ast(&ctx)).not();
            println!("{}", qv[n].pretty_print());
            solver.assert(&and);
            let result2 = solver.check();

            match result2 {
                SatResult::Sat => {
                    solver.pop(1);
                    solver.assert(&imp);
                    let result = solver.check();
                    match result {
                        SatResult::Sat => {
                            panic!("Invariant and Guard does not imply Weakest Pre [body](Inv)")
                        }
                        SatResult::Unsat => {
                            println!("Invariant and Guard implies Weakest Pre [body](Inv)")
                        }
                        SatResult::Unknown => panic!("Z3 is confused"),
                    }
                }

                SatResult::Unsat => println!("Invariant and Guard is unsatisfiable"),
                SatResult::Unknown => panic!("Z3 is confused"),
            }
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
    fn pretty_print(&self) -> String {
        match self {
            Function::Division(n, d) => format!("({}/{})", n.pretty_print(), d.pretty_print()),
            Function::Min(x, y) => format!("min({}, {})", x.pretty_print(), y.pretty_print()),
            Function::Max(x, y) => format!("max({}, {})", x.pretty_print(), y.pretty_print()),
            Function::Count(a, e) => todo!(),
            Function::LogicalCount(a, e) => todo!(),
            Function::Length(a) => todo!(),
            Function::LogicalLength(a) => todo!(),
            Function::Fac(n) => format!("fac({})", n.pretty_print()),
            Function::Fib(n) => format!("fib({})", n.pretty_print()),
        }
    }

    fn ver_con(&self, c: BExpr) -> Vec<BExpr> {
        match self {
            Function::Division(n, d) => todo!(),
            Function::Min(x, y) => todo!(),
            Function::Max(x, y) => todo!(),
            Function::Count(a, e) => unimplemented!(),
            Function::LogicalCount(a, e) => unimplemented!(),
            Function::Length(a) => unimplemented!(),
            Function::LogicalLength(a) => unimplemented!(),
            Function::Fac(n) => todo!(),
            Function::Fib(n) => todo!(),
        }
    }

    fn z3_ast<'ctx>(&self, ctx: &'ctx Context) -> z3int<'ctx> {
        match self {
            Function::Division(n, d) => n.z3_ast(&ctx).div(&d.z3_ast(&ctx)),
            Function::Min(x, y) => todo!(),
            Function::Max(x, y) => todo!(),
            Function::Count(a, e) => todo!(),
            Function::LogicalCount(a, e) => todo!(),
            Function::Length(a) => todo!(),
            Function::LogicalLength(a) => todo!(),
            Function::Fac(n) => todo!(),
            Function::Fib(n) => todo!(),
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
    let pre = gcl::parse::parse_predicate("n > 0").unwrap();
    let post = gcl::parse::parse_predicate("n>2").unwrap();
    let src = r#"
        n:=n+1;
        n:=n+1
    "#;
    let cmds = gcl::parse::parse_commands(src).unwrap();
    let c = cmds.clone();

    let _ = env_logger::try_init();
    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let binding = c.ver_con(post);
    let pre_new = binding.last().unwrap().clone().z3_ast(&ctx);
    let pre = pre.z3_ast(&ctx);
    for n in (0..binding.len()).rev() {
        println!("{}", binding[n].pretty_print());
    }
    solver.assert(&ast::Bool::implies(&pre, &pre_new).not());
    let result = match solver.check() {
        SatResult::Sat => false,
        SatResult::Unsat => true,
        SatResult::Unknown => false,
    };
    assert!(result);
}

#[test]
fn pre_condition_test2() {
    let pre = gcl::parse::parse_predicate("x>=0 && y>=0").unwrap();
    let post = gcl::parse::parse_predicate("z=5").unwrap();
    let src = r#"
        x:=3;
        y:=2;
        z:=x+y
    "#;
    let cmds = gcl::parse::parse_commands(src).unwrap();
    let c = cmds.clone();

    let _ = env_logger::try_init();
    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let binding = c.ver_con(post);
    let pre_new = binding.last().unwrap().clone().z3_ast(&ctx);
    let pre = pre.z3_ast(&ctx);
    for n in (0..binding.len()).rev() {
        println!("{}", binding[n].pretty_print());
    }
    solver.assert(&ast::Bool::implies(&pre, &pre_new).not());
    let result = match solver.check() {
        SatResult::Sat => false,
        SatResult::Unsat => true,
        SatResult::Unknown => false,
    };
    assert!(result);
}

#[test]
fn pre_condition_test3() {
    let pre = gcl::parse::parse_predicate("x>=0 && y<0").unwrap();
    let post = gcl::parse::parse_predicate("x=y").unwrap();
    let src = r#"
        if (x<y) -> x:=y
        [] (x>y) -> y:=x
        [] (x=y) -> skip
        fi
    "#;
    let cmds = gcl::parse::parse_commands(src).unwrap();
    let c = cmds.clone();

    let _ = env_logger::try_init();
    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let binding = c.ver_con(post);
    let pre_new = binding.last().unwrap().clone().z3_ast(&ctx);
    let pre = pre.z3_ast(&ctx);
    for n in (0..binding.len()).rev() {
        println!("{}", binding[n].pretty_print());
    }
    solver.assert(&ast::Bool::implies(&pre, &pre_new).not());
    let result = match solver.check() {
        SatResult::Sat => false,
        SatResult::Unsat => true,
        SatResult::Unknown => false,
    };
    assert!(result);
}

#[test]
fn pre_condition_test4() {
    let pre = gcl::parse::parse_predicate("true").unwrap();
    let post = gcl::parse::parse_predicate("M=res*N+m").unwrap();
    let src = r#"
        res:=0;
        m:=M;
        do {M=res*N+m} m>=N ->
        m:=m-N;
        res:=res+1
        od
    "#;
    let cmds = gcl::parse::parse_commands(src).unwrap();
    let c = cmds.clone();

    let _ = env_logger::try_init();
    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let binding = c.ver_con(post);
    let pre_new = binding.last().unwrap().clone().z3_ast(&ctx);
    let pre = pre.z3_ast(&ctx);
    for n in (0..binding.len()).rev() {
        println!("{}", binding[n].pretty_print());
    }
    solver.assert(&ast::Bool::implies(&pre, &pre_new).not());
    let result = match solver.check() {
        SatResult::Sat => false,
        SatResult::Unsat => true,
        SatResult::Unknown => false,
    };
    assert!(result);
}

#[test]
fn pre_condition_test5() {
    let pre = gcl::parse::parse_predicate("true").unwrap();
    let post = gcl::parse::parse_predicate("n=0").unwrap();
    let src = r#"
        n:=12;
        do {n>=0} n>0 ->
        n:=n-1
        od
    "#;
    let cmds = gcl::parse::parse_commands(src).unwrap();
    let c = cmds.clone();

    let _ = env_logger::try_init();
    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let binding = c.ver_con(post);
    let pre_new = binding.last().unwrap().clone().z3_ast(&ctx);
    let pre = pre.z3_ast(&ctx);
    for n in (0..binding.len()).rev() {
        println!("{}", binding[n].pretty_print());
    }
    solver.assert(&ast::Bool::implies(&pre, &pre_new).not());
    let result = match solver.check() {
        SatResult::Sat => false,
        SatResult::Unsat => true,
        SatResult::Unknown => false,
    };
    assert!(result);
}

#[test]
fn pre_condition_test6() {
    let pre = gcl::parse::parse_predicate("true").unwrap();
    let post = gcl::parse::parse_predicate("n=1").unwrap();
    let src = r#"
        n:=1024;
        do {n>=1} n>1 ->
        n:=n/2
        od
    "#;
    let cmds = gcl::parse::parse_commands(src).unwrap();
    let c = cmds.clone();

    let _ = env_logger::try_init();
    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let binding = c.ver_con(post);
    let pre_new = binding.last().unwrap().clone().z3_ast(&ctx);
    let pre = pre.z3_ast(&ctx);
    for n in (0..binding.len()).rev() {
        println!("{}", binding[n].pretty_print());
    }
    solver.assert(&ast::Bool::implies(&pre, &pre_new).not());
    let result = match solver.check() {
        SatResult::Sat => false,
        SatResult::Unsat => true,
        SatResult::Unknown => false,
    };
    assert!(result);
}

#[test]
fn pre_condition_test7() {
    let pre = gcl::parse::parse_predicate("true").unwrap();
    let post = gcl::parse::parse_predicate("M=res*N+m").unwrap();
    let src = r#"
        res:=0;
        m:=M;
        do {M=res*N+m} m>=N ->
        m:=m-N;
        res:=res+1
        od
    "#;
    let cmds = gcl::parse::parse_commands(src).unwrap();
    let c = cmds.clone();

    let _ = env_logger::try_init();
    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let binding = c.ver_con(post);
    let pre_new = binding.last().unwrap().clone().z3_ast(&ctx);
    let pre = pre.z3_ast(&ctx);
    for n in (0..binding.len()).rev() {
        println!("{}", binding[n].pretty_print());
    }
    solver.assert(&ast::Bool::implies(&pre, &pre_new).not());
    let result = match solver.check() {
        SatResult::Sat => false,
        SatResult::Unsat => true,
        SatResult::Unknown => false,
    };
    assert!(result);
}

#[test]
fn pre_condition_test8() {
    let pre = gcl::parse::parse_predicate("0<=y").unwrap();
    let post = gcl::parse::parse_predicate("x=y").unwrap();
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
    let cmds = gcl::parse::parse_commands(src).unwrap();
    let c = cmds.clone();

    let _ = env_logger::try_init();
    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let binding = c.ver_con(post);
    let pre_new = binding.last().unwrap().clone().z3_ast(&ctx);
    let pre = pre.z3_ast(&ctx);
    for n in (0..binding.len()).rev() {
        println!("{}", binding[n].pretty_print());
    }
    solver.assert(&ast::Bool::implies(&pre, &pre_new).not());
    let result = match solver.check() {
        SatResult::Sat => false,
        SatResult::Unsat => true,
        SatResult::Unknown => false,
    };
    assert!(result);
}
