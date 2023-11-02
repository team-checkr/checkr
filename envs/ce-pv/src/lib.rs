extern crate env_logger;

extern crate z3;
use ce_core::components::{AnnotatedCommand, GclAnnotatedEditor};

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
    fn def(&self) -> BExpr;
}

pub trait Be {
    fn z3_ast<'ctx>(&self, ctx: &'ctx Context) -> Bool<'ctx>;
    fn def(&self) -> BExpr;
}

pub trait GuardE {
    fn ver_con_if(&self, c: BExpr) -> Vec<BExpr>;
    fn ver_con_do(&self, i: Predicate, c: BExpr) -> PvOutput;
}

pub trait Fun {
    fn z3_ast<'ctx>(&self, ctx: &'ctx Context) -> z3int<'ctx>;
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
        // Checks if user given loop invariant holds for all checks
        // Checks are !((Inv && !Guard) -> Q)   (Q is postcondition for loop)
        // !((Inv && Guard) -> WP[Body][Inv])   (WP[Body][Inv] is weakest precondition for body)
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
                // If one check for the invariant doesnt hold, break
                if res == false {
                    break;
                }
            }
        }
        // Checks if the user given precondition
        // Implies the precondition generated
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
        let input = props.input().clone();
        let input2 = props.input().clone();

        cx.render(rsx!(StandardLayout {
            input: cx.render(rsx!(GclAnnotatedEditor {
                command: AnnotatedCommand {
                    pre: input.pre.clone(),
                    cmds: input.cmds.clone(),
                    post: input.post.clone()
                },
                on_change: move |cmds: AnnotatedCommand| props.set_input(PvInput {
                    pre: cmds.pre,
                    post: cmds.post,
                    cmds: cmds.cmds,
                }),
            })),
            output: cx.render(rsx!(div {
                class: "grid place-items-center",
                div {
                    props.with_result(cx, |res| cx.render(rsx!(div {
                        class: "grid place-items-center text-xl divide-y font-mono",
                        pre {
                            for (cmd, cond) in intersperse_conds(&input2.cmds, &res.reference().conds) {
                                cx.render(rsx!(div {
                                    class: "flex text-sm flex-col",
                                    span { cmd }
                                    if let Some(cond) = cond {
                                        cx.render(rsx!(span { class: "text-xs text-orange-500", " {{ " cond " }}" }))
                                    }
                                }))
                            }
                        }
                        div {
                            for cond in res.reference().conds.iter().rev() {
                                cx.render(rsx!(div {
                                    class: "grid place-items-center text-sm",
                                    span { cond.to_string() }
                                }))
                            }
                        }
                        div {
                            for cond in res.reference().checks.iter().rev() {
                                cx.render(rsx!(div {
                                    class: "grid place-items-center text-sm",
                                    span { cond.to_string() }
                                }))
                            }
                        }
                    })))
                }
            })),
        }))
    }
}

fn intersperse_conds(commands: &Commands, conds: &[BExpr]) -> Vec<(String, Option<String>)> {
    let mut buf = Vec::new();

    let mut idx = 0;

    for l in commands.to_string().lines() {
        if l.ends_with("fi") || l.ends_with("fi ;") || l.ends_with("od") || l.ends_with("od ;") {
            idx += 1;
            buf.push((l.to_string(), None));
        } else {
            buf.push((l.to_string(), Some(conds[idx].to_string())));
        }
    }

    buf
}

impl Generate for PvInput {
    type Context = ();

    fn gen<R: rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        let cmds = Commands::gen(&mut Default::default(), rng);

        Self {
            pre: BExpr::Bool(true),
            post: BExpr::Bool(true),
            cmds,
        }
    }
}

impl Cmds for Commands {
    // Generate verification conditions
    // for Sequence of commands
    fn ver_con(&self, cond: BExpr) -> PvOutput {
        let new_c = cond.clone();
        let mut op = PvOutput {
            conds: Vec::new(),
            checks: Vec::new(),
        };
        let mut cs = Vec::new();
        cs.push(new_c);
        for n in (0..self.0.len()).rev() {
            let mut p = self.0[n].ver_con(cs.last().unwrap().clone());
            cs.append(&mut p.conds);
            op.checks.append(&mut p.checks)
        }
        for i in 0..cs.len() {
            op.conds.push(
                BExpr::Logic(Box::new(cs[i].clone()), LogicOp::And, Box::new(cs[i].def()))
                    .simplify(),
            );
        }
        op
    }
}

impl Cmd for Command {
    // Generate Hoare triple
    // for single command
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
            Command::Loop(_) | Command::Annotated(_, _, _) | Command::Continue | Command::Break => {
                tracing::warn!("Loop, Annotated, Continue, Break, are not implemented")
            }
        };
        op
    }
}

impl GuardE for Vec<Guard> {
    // Generate Hoare triples
    // for if fi
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
    // Generate Hoare triples
    // for do od loops
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
    // Generate AST that Z3 can understand
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
                AOp::Pow => ast::Int::power(&l.z3_ast(&ctx), &r.z3_ast(&ctx)).to_int(),
            },
            AExpr::Function(f) => f.z3_ast(&ctx),
        }
    }

    // Add extra definitions to predicate
    // if function isn't defined for all numbers
    fn def(&self) -> BExpr {
        match self {
            AExpr::Binary(l, op, r) => match op {
                AOp::Divide => {
                    if r.find_vars().len() > 0 {
                        BExpr::Rel(*r.clone(), RelOp::Ne, AExpr::Number(0))
                    } else {
                        BExpr::Bool(true)
                    }
                }
                _ => BExpr::Logic(Box::new(l.def()), LogicOp::And, Box::new(r.def())),
            },
            AExpr::Minus(m) => m.def(),
            AExpr::Function(f) => match f {
                Function::Division(_, r) => {
                    if r.find_vars().len() > 0 {
                        BExpr::Rel(*r.clone(), RelOp::Ne, AExpr::Number(0))
                    } else {
                        BExpr::Bool(true)
                    }
                }
                Function::Fib(n) => {
                    if n.find_vars().len() > 0 {
                        BExpr::Rel(*n.clone(), RelOp::Ge, AExpr::Number(0))
                    } else {
                        BExpr::Bool(true)
                    }
                }
                Function::Fac(n) => {
                    if n.find_vars().len() > 0 {
                        BExpr::Rel(*n.clone(), RelOp::Ge, AExpr::Number(0))
                    } else {
                        BExpr::Bool(true)
                    }
                }
                _ => BExpr::Bool(true),
            },
            _ => BExpr::Bool(true),
        }
    }
}

impl Be for BExpr {
    // Generate AST that Z3 can understand
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

    // Add extra definitions to predicate
    // if function isn't defined for all numbers
    fn def(&self) -> BExpr {
        match self {
            BExpr::Bool(_) => self.clone(),
            BExpr::Rel(l, _, r) => BExpr::Logic(Box::new(l.def()), LogicOp::And, Box::new(r.def())),
            BExpr::Logic(l, op, r) => BExpr::Logic(Box::new(l.def()), *op, Box::new(r.def())),
            BExpr::Not(b) => BExpr::Not(Box::new(b.def())),
            BExpr::Quantified(_, _, _) => unimplemented!(),
        }
    }
}

impl Fun for Function {
    fn z3_ast<'ctx>(&self, ctx: &'ctx Context) -> z3int<'ctx> {
        match self {
            Function::Division(n, d) => n.z3_ast(&ctx).div(&d.z3_ast(&ctx)),
            Function::Min(x, y) => {
                let min = RecFuncDecl::new(
                    &ctx,
                    "min",
                    &[&Sort::int(&ctx), &Sort::int(&ctx)],
                    &Sort::int(&ctx),
                );
                let a = ast::Int::new_const(&ctx, "a");
                let b = ast::Int::new_const(&ctx, "b");
                let cond: ast::Bool = a.le(&b);
                let body = cond.ite(&a, &b);
                min.add_def(&[&a, &b], &body);
                min.apply(&[&x.z3_ast(&ctx), &y.z3_ast(&ctx)])
                    .as_int()
                    .unwrap()
            }
            Function::Max(x, y) => {
                let max = RecFuncDecl::new(
                    &ctx,
                    "max",
                    &[&Sort::int(&ctx), &Sort::int(&ctx)],
                    &Sort::int(&ctx),
                );
                let a = ast::Int::new_const(&ctx, "a");
                let b = ast::Int::new_const(&ctx, "b");
                let cond: ast::Bool = a.ge(&b);
                let body = cond.ite(&a, &b);
                max.add_def(&[&a, &b], &body);
                max.apply(&[&x.z3_ast(&ctx), &y.z3_ast(&ctx)])
                    .as_int()
                    .unwrap()
            }
            Function::Count(_, _) => todo!(),
            Function::LogicalCount(_, _) => todo!(),
            Function::Length(_) => todo!(),
            Function::LogicalLength(_) => todo!(),
            Function::Fac(x) => {
                let fac = RecFuncDecl::new(&ctx, "fac", &[&Sort::int(&ctx)], &Sort::int(&ctx));
                let n = ast::Int::new_const(&ctx, "n");
                let n_m_1 = ast::Int::sub(&ctx, &[&n, &ast::Int::from_i64(&ctx, 1)]);
                let fac_n_m_1 = fac.apply(&[&n_m_1]);
                let cond: ast::Bool = n.le(&ast::Int::from_i64(&ctx, 0));
                let body = cond.ite(
                    &ast::Int::from_i64(&ctx, 1),
                    &ast::Int::mul(&ctx, &[&n, &fac_n_m_1.as_int().unwrap()]),
                );
                fac.add_def(&[&n], &body);
                fac.apply(&[&x.z3_ast(&ctx)]).as_int().unwrap()
            }
            Function::Fib(x) => {
                let fib = RecFuncDecl::new(&ctx, "fib", &[&Sort::int(&ctx)], &Sort::int(&ctx));
                let n = ast::Int::new_const(&ctx, "n");
                let n_minus_1 = ast::Int::sub(&ctx, &[&n, &ast::Int::from_i64(&ctx, 1)]);
                let fib_of_n_minus_1 = fib.apply(&[&n_minus_1]);
                let n_minus_2 = ast::Int::sub(&ctx, &[&n, &ast::Int::from_i64(&ctx, 2)]);
                let fib_of_n_minus_2 = fib.apply(&[&n_minus_2]);
                let cond: ast::Bool = n.lt(&ast::Int::from_i64(&ctx, 2));
                let body = cond.ite(
                    &n,
                    &ast::Int::add(
                        &ctx,
                        &[
                            &fib_of_n_minus_1.as_int().unwrap(),
                            &fib_of_n_minus_2.as_int().unwrap(),
                        ],
                    ),
                );
                fib.add_def(&[&n], &body);
                fib.apply(&[&x.z3_ast(&ctx)]).as_int().unwrap()
            }
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
        ValidationResult::CorrectTerminated => assert!(true),
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

#[test]
fn pre_condition_test9() {
    let pr = gcl::parse::parse_predicate("n>=0").unwrap();
    let po = gcl::parse::parse_predicate("x>=0").unwrap();
    let src = r#"
        x:=n+1;
        x:=1/x
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
fn pre_condition_test10() {
    let pr = gcl::parse::parse_predicate("n>=0").unwrap();
    let po = gcl::parse::parse_predicate("r=fib(n)").unwrap();
    let src = r#"
        r:=0;
        i:=0;
        s:=1;
        do {(0<=i && i<=n) && r=fib(i) && s=fib(i+1)} i!=n ->
            t:=s;
            s:=r+s;
            r:=t;
            i:=i+1
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
fn pre_condition_test11() {
    let pr = gcl::parse::parse_predicate("true").unwrap();
    let po = gcl::parse::parse_predicate("z=min(x,y) && w=max(x,y)").unwrap();
    let src = r#"
    if x<y ->
        z:=x;
        w:=y
    [] x>= y ->
        z:=y;
        w:=x
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
