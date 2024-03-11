use std::collections::BTreeSet;

use crate::{
    ast::{Command, CommandKind, Commands, Predicate},
    parse::SourceSpan,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Assertion {
    pub predicate: Predicate,
    pub span: SourceSpan,
}

#[derive(Debug, Default, Clone)]
pub struct Accumulator {
    assertions: BTreeSet<Assertion>,
    predicate_spans: BTreeSet<(Predicate, SourceSpan)>,
}

impl Commands {
    pub fn assertions(&self) -> BTreeSet<Assertion> {
        self.tri(Accumulator::default()).assertions
    }

    fn tri(&self, mut acc: Accumulator) -> Accumulator {
        for c in self.0.iter().rev() {
            acc = c.tri(acc);
        }
        acc
    }
}

impl Command {
    fn tri(&self, mut acc: Accumulator) -> Accumulator {
        for p in self.post_predicates.iter().rev() {
            for (old_p, old_span) in acc.predicate_spans {
                acc.assertions.insert(Assertion {
                    predicate: p.predicate.clone().implies(old_p),
                    span: old_span,
                });
            }
            acc.predicate_spans = [(p.predicate.clone(), p.span)].into();
        }

        // Compute the weakest precondition for the command
        match &self.kind {
            CommandKind::Assignment(t, v) => {
                let mut new = BTreeSet::new();
                for (old_p, span) in acc.predicate_spans {
                    let q = old_p.subst_var(t, v);
                    new.insert((q, span));
                }
                acc.predicate_spans = new;
            }
            CommandKind::Skip => {}
            CommandKind::If(gcs) => {
                acc = gcs
                    .iter()
                    .map(|gc| {
                        let mut q = gc.1.tri(acc.clone());
                        let mut new = BTreeSet::new();
                        for (old_p, span) in q.predicate_spans {
                            new.insert((gc.0.clone().implies(old_p.clone()), span));
                        }
                        q.predicate_spans = new;
                        q
                    })
                    .fold(Accumulator::default(), |mut acc, c| {
                        acc.assertions.extend(c.assertions);
                        acc.predicate_spans.extend(c.predicate_spans);
                        acc
                    });
            }
            CommandKind::Loop(inv, gcs) => {
                // 1. P => I
                // 2. I => wp[GC](I)
                // 3. I && !G => Q
                // -------------------
                // {P} do[I] GC od {Q}

                let mut new = Accumulator::default();

                // 1. P => I
                new.predicate_spans
                    .insert((inv.predicate.clone(), inv.span));

                // 2. I => wp[GC](I)
                for gc in gcs {
                    let q = gc.1.tri(Accumulator {
                        assertions: Default::default(),
                        predicate_spans: [(inv.predicate.clone(), inv.span)].into(),
                    });
                    for (old_p, old_span) in q.predicate_spans {
                        new.assertions.insert(Assertion {
                            predicate: inv.predicate.clone().and(gc.0.clone()).implies(old_p),
                            span: old_span,
                        });
                    }
                    new.assertions.extend(q.assertions);
                }

                // 3. I && !G => Q
                for (old_p, old_span) in acc.predicate_spans {
                    for gc in gcs {
                        new.assertions.insert(Assertion {
                            predicate: inv
                                .predicate
                                .clone()
                                .and(!gc.0.clone())
                                .implies(old_p.clone()),
                            span: old_span,
                        });
                    }
                }

                acc = new;
            }
        }

        for p in self.pre_predicates.iter().rev() {
            for (old_p, old_span) in acc.predicate_spans {
                acc.assertions.insert(Assertion {
                    predicate: p.predicate.clone().implies(old_p),
                    span: old_span,
                });
            }
            acc.predicate_spans = [(p.predicate.clone(), p.span)].into();
        }

        acc
    }
}

impl Assertion {
    pub fn smt(&self) -> impl Iterator<Item = smtlib_lowlevel::ast::Command> {
        let fv = self.predicate.fv();

        fv.into_iter()
            .map(|v| {
                smtlib_lowlevel::ast::Command::DeclareConst(
                    smtlib_lowlevel::lexicon::Symbol(v.name().to_string()),
                    smtlib_lowlevel::ast::Sort::Sort(smtlib_lowlevel::ast::Identifier::Simple(
                        smtlib_lowlevel::lexicon::Symbol("Int".to_string()),
                    )),
                )
            })
            .chain([
                smtlib_lowlevel::ast::Command::Assert((!self.predicate.smt()).into()),
                smtlib_lowlevel::ast::Command::CheckSat,
            ])
    }
}
