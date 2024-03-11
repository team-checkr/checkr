use std::collections::BTreeSet;

use itertools::Itertools;

use crate::{
    ast::{Command, CommandKind, Commands, Predicate},
    parse::SourceSpan,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Source {
    pub span: SourceSpan,
    pub text: Option<String>,
    pub related: Option<(String, SourceSpan)>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Assertion {
    pub predicate: Predicate,
    pub source: Source,
}

#[derive(Debug, Default, Clone)]
pub struct Accumulator {
    assertions: BTreeSet<Assertion>,
    predicate_spans: BTreeSet<(Predicate, Source)>,
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

    pub fn is_fully_annotated(&self) -> bool {
        let before = self.0.first().is_some_and(|x| !x.pre_predicates.is_empty());
        let after = self.0.last().is_some_and(|x| !x.post_predicates.is_empty());
        let in_between = self
            .0
            .iter()
            .tuple_windows()
            .all(|(a, b)| !a.post_predicates.is_empty() || !b.pre_predicates.is_empty());
        let inside = self.0.iter().all(|c| c.is_fully_annotated());
        before && after && in_between && inside
    }
}

impl Command {
    fn tri(&self, mut acc: Accumulator) -> Accumulator {
        for p in self.post_predicates.iter().rev() {
            for (old_p, old_src) in acc.predicate_spans {
                acc.assertions.insert(Assertion {
                    predicate: p.predicate.clone().implies(old_p),
                    source: old_src,
                });
            }
            acc.predicate_spans = [(
                p.predicate.clone(),
                Source {
                    span: p.span,
                    text: Some(format!("{:?} doesn't hold", p.predicate)),
                    related: None,
                },
            )]
            .into();
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
                        let mut q = gc.cmds.tri(acc.clone());
                        let mut new = BTreeSet::new();
                        for (old_p, span) in q.predicate_spans {
                            new.insert((gc.guard.clone().implies(old_p.clone()), span));
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
                new.predicate_spans.insert((
                    inv.predicate.clone(),
                    Source {
                        span: inv.span,
                        text: Some("invariant doesn't hold initially".to_string()),
                        related: None,
                    },
                ));

                // 2. I => wp[GC](I)
                for gc in gcs {
                    let q = gc.cmds.tri(Accumulator {
                        assertions: Default::default(),
                        predicate_spans: [(
                            inv.predicate.clone(),
                            Source {
                                span: inv.span,
                                text: Some(format!(
                                    "invariant doesn't hold at end of the branch guarded by `{}`",
                                    gc.guard
                                )),
                                related: Some((
                                    "invariant doesn't hold for this branch".to_string(),
                                    gc.guard_span,
                                )),
                            },
                        )]
                        .into(),
                    });
                    for (old_p, old_src) in q.predicate_spans {
                        new.assertions.insert(Assertion {
                            predicate: inv.predicate.clone().and(gc.guard.clone()).implies(old_p),
                            source: old_src,
                        });
                    }
                    new.assertions.extend(q.assertions);
                }

                // 3. I && !G => Q
                for (old_p, old_src) in acc.predicate_spans {
                    if let Some(not_done) =
                        gcs.iter().map(|gc| gc.guard.clone()).reduce(|a, b| a.or(b))
                    {
                        new.assertions.insert(Assertion {
                            predicate: inv.predicate.clone().and(!not_done).implies(old_p.clone()),
                            source: old_src,
                        });
                    } else {
                        new.assertions.insert(Assertion {
                            predicate: inv.predicate.clone().implies(old_p.clone()),
                            source: old_src,
                        });
                    }
                }

                acc = new;
            }
        }

        for p in self.pre_predicates.iter().rev() {
            acc.predicate_spans = [(
                p.predicate.clone(),
                Source {
                    span: p.span,
                    text: Some(format!("{:?} doesn't hold", p.predicate)),
                    related: None,
                },
            )]
            .into();
        }

        acc
    }
    pub fn is_fully_annotated(&self) -> bool {
        match &self.kind {
            CommandKind::Assignment(_, _) | CommandKind::Skip => true,
            CommandKind::If(gcs) | CommandKind::Loop(_, gcs) => {
                gcs.iter().all(|gc| gc.cmds.is_fully_annotated())
            }
        }
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
