use gcl::{
    ast::{Command, Commands, Flow, Guard, SecurityClass, Target},
    memory::Memory,
};
use indexmap::IndexSet;
use itertools::{chain, Itertools};
use serde::{Deserialize, Serialize};

trait SecurityFlows {
    fn flows(&self) -> IndexSet<Flow<Target>> {
        self.sec(&Default::default())
    }
    fn sec(&self, implicit: &IndexSet<Target>) -> IndexSet<Flow<Target>>;
}

impl SecurityFlows for Commands {
    fn sec(&self, implicit: &IndexSet<Target>) -> IndexSet<Flow<Target>> {
        self.0.iter().flat_map(|c| c.sec(implicit)).collect()
    }
}

impl SecurityFlows for Command {
    fn sec(&self, implicit: &IndexSet<Target>) -> IndexSet<Flow<Target>> {
        match self {
            Command::Assignment(t, a) => chain!(
                implicit.iter().cloned(),
                match t {
                    Target::Variable(_) => Default::default(),
                    Target::Array(_, idx) => idx.fv(),
                },
                a.fv()
            )
            .map(|i| Flow {
                from: i,
                into: t.clone().unit(),
            })
            .collect(),
            Command::Skip => IndexSet::default(),
            Command::If(c) | Command::Loop(c) | Command::EnrichedLoop(_, c) => {
                c.iter()
                    .fold(
                        (implicit.clone(), IndexSet::default()),
                        |(implicit, flows), guard| {
                            let (new_implicit, new_flows) = guard_sec2(guard, &implicit);

                            (
                                implicit.union(&new_implicit).cloned().collect(),
                                flows.union(&new_flows).cloned().collect(),
                            )
                        },
                    )
                    .1
            }
            Command::Annotated(_, c, _) => c.sec(implicit),
            Command::Break => IndexSet::default(),
            Command::Continue => IndexSet::default(),
        }
    }
}

fn guard_sec2(
    guard: &Guard,
    implicit: &IndexSet<Target>,
) -> (IndexSet<Target>, IndexSet<Flow<Target>>) {
    let implicit = implicit.iter().cloned().chain(guard.0.fv()).collect();
    let flows = guard.1.sec(&implicit);
    (implicit, flows)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityLattice {
    allowed: IndexSet<Flow<SecurityClass>>,
}

impl SecurityLattice {
    pub fn new(flows: &[Flow<SecurityClass>]) -> SecurityLattice {
        let mut allowed: IndexSet<Flow<SecurityClass>> = flows.iter().cloned().collect();
        let mut last_len = 0;
        loop {
            let mut to_add: IndexSet<Flow<SecurityClass>> = IndexSet::new();
            for f in &allowed {
                for a in &allowed {
                    // a -> b, b -> c
                    // --------------
                    //     a -> c
                    let new_flow = Flow {
                        from: f.from.clone(),
                        into: a.into.clone(),
                    };
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
    pub fn parse(src: &str) -> color_eyre::Result<SecurityLattice> {
        let flows = gcl::parse::parse_security_lattice(src)?;
        Ok(Self::new(&flows))
    }
    pub fn allows(&self, f: &Flow<SecurityClass>) -> bool {
        f.from == f.into || self.allowed.contains(f)
    }

    fn all_allowed<'a>(
        &'a self,
        classification: &'a Memory<SecurityClass>,
    ) -> impl Iterator<Item = Flow<Target>> + 'a {
        classification
            .iter()
            .cartesian_product(classification.iter())
            .filter_map(|(a, b)| {
                if self.allows(&Flow {
                    from: a.value().clone(),
                    into: b.value().clone(),
                }) {
                    Some(Flow {
                        from: a.target(),
                        into: b.target(),
                    })
                } else {
                    None
                }
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SecurityAnalysisOutput {
    pub actual: Vec<Flow<Target>>,
    pub allowed: Vec<Flow<Target>>,
    pub violations: Vec<Flow<Target>>,
}

impl SecurityAnalysisOutput {
    pub fn run(
        mapping: &Memory<SecurityClass>,
        lattice: &SecurityLattice,
        cmds: &Commands,
    ) -> Self {
        let allowed = lattice.all_allowed(mapping).sorted().dedup().collect_vec();
        let actual = cmds.flows();
        let violations = actual
            .iter()
            .filter(|&flow| !allowed.contains(flow))
            .cloned()
            .sorted()
            .dedup()
            .collect();

        Self {
            actual: actual.into_iter().sorted().collect(),
            allowed,
            violations,
        }
    }
}
