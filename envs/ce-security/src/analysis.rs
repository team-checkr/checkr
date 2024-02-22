use std::collections::{BTreeMap, BTreeSet};

use gcl::ast::{Command, Commands, Guard, Target};
use itertools::{chain, Itertools};
use serde::{Deserialize, Serialize};

use crate::{flow, Flow};

#[derive(tapi::Tapi, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "SecurityAnalysis")]
pub struct SecurityLattice {
    pub allowed: BTreeSet<Flow>,
}

impl SecurityLattice {
    pub fn new(rules: &[Flow]) -> Self {
        let mut allowed: BTreeSet<Flow> = rules.iter().cloned().collect();
        let mut last_len = 0;
        loop {
            let mut to_add: BTreeSet<Flow> = BTreeSet::new();
            for f in &allowed {
                for a in &allowed {
                    // a -> b, b -> c
                    // --------------
                    //     a -> c
                    let new_flow = flow(&f.from, &a.into);
                    if f.into == a.from && !allowed.contains(&new_flow) {
                        to_add.insert(new_flow);
                    }
                }
            }

            for f in to_add {
                allowed.insert(f);
            }

            if allowed.len() == last_len {
                break;
            }
            last_len = allowed.len();
        }

        SecurityLattice { allowed }
    }

    pub fn allows(&self, f: &Flow) -> bool {
        f.from == f.into || self.allowed.contains(f)
    }

    pub fn all_allowed<'a>(
        &'a self,
        classification: &'a BTreeMap<Target, String>,
    ) -> impl Iterator<Item = Flow> + 'a {
        classification
            .iter()
            .cartesian_product(classification.iter())
            .filter_map(|(a, b)| {
                if self.allows(&flow(a.1, b.1)) {
                    Some(flow(a.0, b.0))
                } else {
                    None
                }
            })
    }
}

pub(crate) trait Security {
    fn flows(&self) -> BTreeSet<Flow> {
        self.sec(&Default::default())
    }
    fn sec(&self, implicit: &BTreeSet<Target>) -> BTreeSet<Flow>;
}

impl Security for Commands {
    fn sec(&self, implicit: &BTreeSet<Target>) -> BTreeSet<Flow> {
        self.0.iter().flat_map(|c| c.sec(implicit)).collect()
    }
}

impl Security for Command {
    fn sec(&self, implicit: &BTreeSet<Target>) -> BTreeSet<Flow> {
        match self {
            Command::Assignment(t, a) => chain!(
                implicit.iter().cloned(),
                match t {
                    Target::Variable(_) => BTreeSet::default(),
                    Target::Array(_, idx) => idx.fv().into_iter().collect(),
                },
                a.fv()
            )
            .map(|i| flow(i, t.clone().unit()))
            .collect(),
            Command::Skip => BTreeSet::default(),
            Command::If(c) | Command::Loop(c) | Command::EnrichedLoop(_, c) => {
                c.iter()
                    .fold(
                        (implicit.clone(), BTreeSet::default()),
                        |(implicit, flows), guard| {
                            let (new_implicit, new_flows) = guard.sec2(&implicit);

                            (
                                implicit.union(&new_implicit).cloned().collect(),
                                flows.union(&new_flows).cloned().collect(),
                            )
                        },
                    )
                    .1
            }
            Command::Annotated(_, c, _) => c.sec(implicit),
            Command::Break => BTreeSet::default(),
            Command::Continue => BTreeSet::default(),
        }
    }
}

trait Security2 {
    fn sec2(&self, implicit: &BTreeSet<Target>) -> (BTreeSet<Target>, BTreeSet<Flow>);
}

impl Security2 for Guard {
    fn sec2(&self, implicit: &BTreeSet<Target>) -> (BTreeSet<Target>, BTreeSet<Flow>) {
        let implicit = implicit.iter().cloned().chain(self.0.fv()).collect();
        let flows = self.1.sec(&implicit);
        (implicit, flows)
    }
}
