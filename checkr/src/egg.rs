use egg::{define_language, rewrite as rw, FromOpError, Id, RecExpr, RecExprParseError};

use crate::ast::{AExpr, AOp, Array, BExpr, Function, LogicOp, Quantifier, Target, Variable};

type Runner = egg::Runner<Gcl, ()>;
type Rewrite = egg::Rewrite<Gcl, ()>;

define_language! {
    pub enum Gcl {
        // Quantifiers
        "exists" = Exists([Id; 2]),
        "forall" = Forall([Id; 2]),
        // AExpr
        "+" = Add([Id; 2]),
        "-" = Sub([Id; 2]),
        "*" = Mul([Id; 2]),
        "^" = Pow([Id; 2]),
        Number(i64),
        Variable(Variable),
        Array(Array, Id),
        // Functions
        "division" = Division([Id; 2]),
        "min" = Min([Id; 2]),
        "max" = Max([Id; 2]),
        Count(String, Id),
        LogicalCount(String, Id),
        Length(String),
        LogicalLength(String),
        "fac" = Fac(Id),
        "fib" = Fib(Id),
        // BExpr
        Bool(bool),
        // - Rel
        "<" = Lt([Id; 2]),
        "<=" = Le([Id; 2]),
        ">" = Gt([Id; 2]),
        ">=" = Ge([Id; 2]),
        "=" = Eq([Id; 2]),
        "!=" = Ne([Id; 2]),
        // - Logic
        "||" = Or([Id; 2]),
        "|" = LOr([Id; 2]),
        "&&" = And([Id; 2]),
        "&" = LAnd([Id; 2]),
        "==>" = Implies([Id; 2]),
        // - Rest
        "!" = Not(Id),
    }
}

fn make_rules() -> Vec<Rewrite> {
    vec![
        rw!("comm-add";  "(+ ?a ?b)"        => "(+ ?b ?a)"),
        rw!("comm-mul";  "(* ?a ?b)"        => "(* ?b ?a)"),
        rw!("assoc-add"; "(+ ?a (+ ?b ?c))" => "(+ (+ ?a ?b) ?c)"),
        rw!("assoc-mul"; "(* ?a (* ?b ?c))" => "(* (* ?a ?b) ?c)"),
        // rw!("sub-canon"; "(- ?a ?b)" => "(+ ?a (* -1 ?b))"),
        // rw!("canon-sub"; "(+ ?a (* -1 ?b))"   => "(- ?a ?b)"),
        // rw!("zero-add"; "(+ ?a 0)" => "?a"),
        // rw!("zero-mul"; "(* ?a 0)" => "0"),
        // rw!("one-mul";  "(* ?a 1)" => "?a"),
        // rw!("add-zero"; "?a" => "(+ ?a 0)"),
        // rw!("mul-one";  "?a" => "(* ?a 1)"),
        // rw!("cancel-sub"; "(- ?a ?a)" => "0"),
        // rw!("distribute"; "(* ?a (+ ?b ?c))"        => "(+ (* ?a ?b) (* ?a ?c))"),
        // rw!("factor"    ; "(+ (* ?a ?b) (* ?a ?c))" => "(* ?a (+ ?b ?c))"),
        // rw!("pow-mul"; "(* (^ ?a ?b) (^ ?a ?c))" => "(^ ?a (+ ?b ?c))"),
        // rw!("pow0"; "(^ ?x 0)" => "1"
        //     if is_not_zero("?x")),
        // rw!("pow1"; "(^ ?x 1)" => "?x"),
        // rw!("pow2"; "(^ ?x 2)" => "(* ?x ?x)"),
        // rw!("pow-recip"; "(^ ?x -1)" => "(/ 1 ?x)"
        //     if is_not_zero("?x")),
        // rw!("recip-mul-div"; "(* ?x (/ 1 ?x))" => "1" if is_not_zero("?x")),
        // rw!("desugar-ne"; "(!= ?a ?b)" => "(! (= ?a ?b))"),
        // rw!("desugar-lq"; "(<= ?a ?b)" => "(| (= ?a ?b) (< ?a ?b))"),
        // rw!("desugar-gq"; "(>= ?a ?b)" => "(! (< ?a ?b))"),
        // rw!("desugar-ge"; "(> ?a ?b)" => "(! (<= ?a ?b))"),
        // rw!("desugar-and-land"; "(&& ?a ?b)" => "(& ?a ?b)"),
        // rw!("desugar-or-lor"; "(|| ?a ?b)" => "(| ?a ?b)"),
        // rw!("desugar-imp"; "(==> ?a ?b)" => "(| (! ?a) ?b)"),
        rw!("comm-or";  "(|| ?a ?b)"        => "(|| ?b ?a)"),
        rw!("comm-lor";  "(| ?a ?b)"        => "(| ?b ?a)"),
        rw!("comm-and";  "(& ?a ?b)"        => "(& ?b ?a)"),
        rw!("comm-land";  "(&& ?a ?b)"        => "(&& ?b ?a)"),
        rw!("assoc-or"; "(|| ?a (|| ?b ?c))" => "(|| (|| ?a ?b) ?c)"),
        rw!("assoc-lor"; "(| ?a (| ?b ?c))" => "(| (| ?a ?b) ?c)"),
        rw!("assoc-and"; "(&& ?a (&& ?b ?c))" => "(&& (&& ?a ?b) ?c)"),
        rw!("assoc-land"; "(& ?a (& ?b ?c))" => "(& (& ?a ?b) ?c)"),
    ]
}

pub trait IntoEgg {
    fn egg(&self) -> String;
    fn rec_expr(&self) -> Result<RecExpr<Gcl>, RecExprParseError<FromOpError>> {
        self.egg().parse()
    }
    fn id(&self, runner: &mut Runner) -> Result<Id, RecExprParseError<FromOpError>> {
        Ok(runner.egraph.add_expr(&self.rec_expr()?))
    }
}

impl IntoEgg for AExpr {
    fn egg(&self) -> String {
        match self {
            AExpr::Number(n) => format!("{n}"),
            AExpr::Reference(t) => match t {
                Target::Variable(v) => format!("{v}"),
                Target::Array(arr, idx) => format!("(array {arr} {})", idx.egg()),
            },
            AExpr::Binary(lhs, AOp::Divide, rhs) => {
                format!("(division {} {})", lhs.egg(), rhs.egg())
            }
            AExpr::Binary(lhs, op, rhs) => format!("({op} {} {})", lhs.egg(), rhs.egg()),
            AExpr::Minus(e) => format!("(- 0 {})", e.egg()),
            AExpr::Function(fun) => fun.egg(),
        }
    }
}

impl IntoEgg for Function {
    fn egg(&self) -> String {
        match self {
            Function::Division(a, b) => format!("(division {} {})", a.egg(), b.egg()),
            Function::Min(a, b) => format!("(min {} {})", a.egg(), b.egg()),
            Function::Max(a, b) => format!("(max {} {})", a.egg(), b.egg()),
            Function::Count(a, b) => format!("(count {} {})", a, b.egg()),
            Function::LogicalCount(a, b) => format!("(count {} {})", a, b.egg()),
            Function::Length(x) => format!("(length {})", x),
            Function::LogicalLength(x) => format!("(length {})", x),
            Function::Fac(x) => format!("(fac {})", x.egg()),
            Function::Fib(x) => format!("(fib {})", x.egg()),
        }
    }
}

impl IntoEgg for BExpr {
    fn egg(&self) -> String {
        match self {
            BExpr::Bool(b) => format!("{b}"),
            BExpr::Rel(l, op, r) => format!("({op} {} {})", l.egg(), r.egg()),
            BExpr::Logic(l, op, r) => format!("({op} {} {})", l.egg(), r.egg()),
            BExpr::Not(b) => format!("(! {})", b.egg()),
            BExpr::Quantified(q, x, b) => format!("({q} {x} {})", b.egg()),
        }
    }
}

impl BExpr {
    pub fn renumber_quantifiers(&self) -> BExpr {
        self.renumber_quantifiers_inner(&mut 0)
    }
    pub fn renumber_quantifiers_inner(&self, count: &mut u64) -> BExpr {
        match self
            .semantics(&Default::default())
            .map(BExpr::Bool)
            .unwrap_or_else(|_| self.clone())
        {
            BExpr::Bool(b) => BExpr::Bool(b),
            BExpr::Rel(l, op, r) => BExpr::Rel(l.simplify(), op, r.simplify()),
            BExpr::Logic(l, op, r) => {
                let l = l.renumber_quantifiers_inner(count);
                let r = r.renumber_quantifiers_inner(count);

                match (l, op, r) {
                    (BExpr::Bool(true), LogicOp::And, x) | (x, LogicOp::And, BExpr::Bool(true)) => {
                        x
                    }
                    (BExpr::Bool(false), LogicOp::And, _)
                    | (_, LogicOp::And, BExpr::Bool(false)) => BExpr::Bool(false),
                    (BExpr::Bool(false), LogicOp::Or, x) | (x, LogicOp::Or, BExpr::Bool(false)) => {
                        x
                    }
                    (BExpr::Bool(true), LogicOp::Or, _) | (_, LogicOp::Or, BExpr::Bool(true)) => {
                        BExpr::Bool(true)
                    }
                    (l, op, r) => BExpr::logic(l, op, r),
                }
            }
            BExpr::Not(x) => {
                let x = x.renumber_quantifiers_inner(count);
                match x {
                    BExpr::Bool(b) => BExpr::Bool(!b),
                    x => BExpr::Not(Box::new(x)),
                }
            }
            BExpr::Quantified(q, t, e) => {
                let x = Target::Variable(Variable(format!("_f{count}")));
                *count += 1;
                BExpr::Quantified(
                    q,
                    x.clone().unit(),
                    Box::new(
                        e.subst_var(&t, &AExpr::Reference(x))
                            .renumber_quantifiers_inner(count),
                    ),
                )
            }
        }
    }
}

pub struct EquivChecker {
    rules: Vec<Rewrite>,
    runner: Runner,
}
impl EquivChecker {
    pub fn register(&mut self, ex: &impl IntoEgg) -> RecExpr<Gcl> {
        let expr = ex.rec_expr().unwrap();
        self.runner.egraph.add_expr(&expr);
        expr
    }
    pub fn run(&mut self) {
        self.runner = std::mem::take(&mut self.runner).run(&self.rules);
    }
    pub fn are_equivalent(&self, x: &RecExpr<Gcl>, y: &RecExpr<Gcl>) -> bool {
        !self.runner.egraph.equivs(x, y).is_empty()
    }
}

impl Default for EquivChecker {
    fn default() -> Self {
        EquivChecker {
            rules: make_rules(),
            runner: Runner::default(),
        }
    }
}

#[test]
fn egg() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let a = AExpr::binary(AExpr::Number(12), AOp::Plus, AExpr::Number(0));
    assert_eq!(a.egg(), "(+ 12 0)");
    let a = a.rec_expr()?;

    let b = AExpr::binary(AExpr::Number(1), AOp::Times, AExpr::Number(12));
    assert_eq!(b.egg(), "(* 1 12)");
    let b = b.rec_expr()?;

    let mut runner = Runner::default(); //.with_explanations_enabled();

    runner.egraph.add_expr(&a);
    runner.egraph.add_expr(&b);

    let runner = runner.run(&make_rules());

    assert_eq!(runner.egraph.equivs(&a, &b).len(), 1);

    Ok(())
}

#[test]
fn egg_quantifiers() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let mut checker = EquivChecker::default();

    let forall = BExpr::Quantified(
        Quantifier::Forall,
        Target::Variable("x".parse().unwrap()),
        Box::new(BExpr::Bool(true)),
    );
    let forall_expr = checker.register(&forall);
    assert_eq!(forall_expr.to_string(), "(forall x true)");
    let forall_re: RecExpr<Gcl> = forall_expr.to_string().parse()?;
    assert_eq!(forall_expr, forall_re);

    let exists = BExpr::Quantified(
        Quantifier::Exists,
        Target::Variable("x".parse().unwrap()),
        Box::new(BExpr::Bool(true)),
    );
    let exists_expr = checker.register(&exists);
    assert_eq!(exists_expr.to_string(), "(exists x true)");
    let exists_re: RecExpr<Gcl> = exists_expr.to_string().parse()?;
    assert_eq!(exists_expr, exists_re);

    checker.run();

    assert!(!checker.are_equivalent(&forall_expr, &exists_expr));

    Ok(())
}
