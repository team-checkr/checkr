use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use serde::{Deserialize, Serialize};

use crate::{
    ast::{Array, Command, Commands, Guard, Variable},
    parse::{self, ParseError},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Flow<T>(pub T, pub T);
impl<T> Flow<T> {
    fn map<S>(&self, f: impl Fn(&T) -> S) -> Flow<S> {
        Flow(f(&self.0), f(&self.1))
    }
}

impl<T> Display for Flow<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -> {}", self.0, self.1)
    }
}

impl Commands {
    pub fn flows(&self) -> HashSet<Flow<Variable>> {
        self.sec(&Default::default())
    }
    fn sec(&self, implicit: &HashSet<Variable>) -> HashSet<Flow<Variable>> {
        self.0.iter().flat_map(|c| c.sec(implicit)).collect()
    }
}

impl Command {
    fn sec(&self, implicit: &HashSet<Variable>) -> HashSet<Flow<Variable>> {
        match self {
            Command::Assignment(Variable(x), a) => implicit
                .iter()
                .cloned()
                .chain(a.fv())
                .map(|i| Flow(i, Variable(x.clone())))
                .collect(),
            Command::Skip => HashSet::default(),
            Command::If(c) | Command::Loop(c) => {
                c.iter()
                    .fold(
                        (implicit.clone(), HashSet::default()),
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
            Command::ArrayAssignment(Array(arr, idx), a) => implicit
                .iter()
                .cloned()
                .chain(a.fv())
                .chain(idx.fv())
                // TODO: Should this really be variable?
                .map(|i| Flow(i, Variable(arr.clone())))
                .collect(),
            Command::Break => HashSet::default(),
            Command::Continue => HashSet::default(),
        }
    }
}

impl Guard {
    fn sec2(&self, implicit: &HashSet<Variable>) -> (HashSet<Variable>, HashSet<Flow<Variable>>) {
        let implicit = implicit.iter().cloned().chain(self.0.fv()).collect();
        let flows = self.1.sec(&implicit);
        (implicit, flows)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SecurityClass(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityLattice {
    allowed: HashSet<Flow<SecurityClass>>,
}

impl SecurityLattice {
    pub fn new(flows: &[Flow<SecurityClass>]) -> SecurityLattice {
        let mut allowed: HashSet<Flow<SecurityClass>> = flows.iter().cloned().collect();
        let mut last_len = 0;
        loop {
            let mut to_add: HashSet<Flow<SecurityClass>> = HashSet::new();
            for f in &allowed {
                for a in &allowed {
                    // a -> b, b -> c
                    // --------------
                    //     a -> c
                    let new_flow = Flow(f.0.clone(), a.1.clone());
                    if f.1 == a.0 && !allowed.contains(&new_flow) {
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
    pub fn parse(src: &str) -> anyhow::Result<SecurityLattice> {
        let flows = parse::gcl::SecurityLatticeParser::new()
            .parse(src)
            .map_err(|e| ParseError::new(src, e))?;

        Ok(Self::new(&flows))
    }
    pub fn allows(&self, f: &Flow<SecurityClass>) -> bool {
        f.0 == f.1 || self.allowed.contains(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SecurityAnalysis {
    pub actual: Vec<Flow<Variable>>,
    pub allowed: Vec<Flow<Variable>>,
    pub violations: Vec<Flow<Variable>>,
}

impl SecurityAnalysis {
    pub fn run(
        mapping: &HashMap<Variable, SecurityClass>,
        lattice: &SecurityLattice,
        cmds: &Commands,
    ) -> Self {
        let actual = cmds.flows();
        let (allowed, violations) = actual
            .iter()
            .cloned()
            .partition(|flow| lattice.allows(&flow.map(|f| mapping[f].clone())));

        Self {
            actual: actual.into_iter().collect(),
            allowed,
            violations,
        }
    }
}
