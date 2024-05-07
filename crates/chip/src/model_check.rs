use std::collections::{hash_map::Entry, HashMap, HashSet, VecDeque};

use itertools::Itertools;
use mcltl::{
    buchi::{Alphabet, AtomicProperty, ProductBuchi},
    ltl::expression::Literal,
    state::State as _,
};

use crate::{
    ast::{BExpr, Locator, Target},
    ast_ext::FreeVariables,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum State {
    Initial,
    Real(crate::interpreter::State),
}

impl mcltl::state::State for State {
    fn initial() -> Self {
        State::Initial
    }

    fn name(&self) -> String {
        match self {
            State::Initial => "INIT".to_string(),
            State::Real(s) => s.raw_id(),
        }
    }
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name().fmt(f)
    }
}
pub struct ReachableStates {
    pub program: crate::interpreter::Program,
    pub states: Vec<crate::interpreter::State>,
    pub relations: HashMap<usize, HashSet<usize>>,
}

pub struct StateSpaceExplosion;

impl ReachableStates {
    pub fn generate(
        ltl_program: &crate::ast::LTLProgram,
        mut fuel: u32,
    ) -> Result<ReachableStates, StateSpaceExplosion> {
        let program = crate::interpreter::Program::compile(
            &ltl_program.commands,
            ltl_program
                .properties
                .iter()
                .flat_map(|(_, property)| property.fv())
                .filter_map(|t| match t {
                    Target::Variable(v) => Some(v),
                    _ => None,
                })
                .chain(ltl_program.initial.keys().cloned()),
        );
        let state =
            program.initial_state(|var| ltl_program.initial.get(var).copied().unwrap_or_default());
        let mut states = Vec::new();
        let mut visited = HashMap::new();
        let mut relations: HashMap<usize, HashSet<_>> = HashMap::new();
        let mut queue = VecDeque::new();
        states.push(state.clone());
        visited.insert(state.clone(), 0);
        queue.push_back(0);
        while let Some(state_id) = queue.pop_front() {
            for next_state in states[state_id].step(&program).collect_vec() {
                if let Some(new_fuel) = fuel.checked_sub(1) {
                    fuel = new_fuel;
                } else {
                    return Err(StateSpaceExplosion);
                }

                let id = match visited.entry(next_state.clone()) {
                    Entry::Occupied(id) => *id.get(),
                    Entry::Vacant(v) => {
                        let id = states.len();
                        v.insert(id);
                        states.push(next_state.clone());
                        queue.push_back(id);
                        id
                    }
                };
                relations.entry(state_id).or_default().insert(id);
                relations.entry(id).or_default();
            }
        }
        Ok(ReachableStates {
            program,
            states,
            relations,
        })
    }

    fn build_kripke(
        &self,
        relational_properties: &[(crate::ast::AExpr, crate::ast::RelOp, crate::ast::AExpr)],
    ) -> mcltl::verifier::kripke::KripkeStructure<State, Literal> {
        let mut kripke: mcltl::verifier::kripke::KripkeStructure<State, Literal> =
            mcltl::verifier::kripke::KripkeStructure::new(
                [State::Real(self.states.first().unwrap().clone())].to_vec(),
            );
        let worlds: Vec<_> = self
            .states
            .iter()
            .map(|state| {
                let mut assignment = relational_properties
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, (l, op, r))| {
                        let holds = BExpr::Rel(l.clone(), *op, r.clone())
                            .evaluate(&self.program, state)
                            .is_ok_and(|x| x);
                        holds.then(|| Literal::from(format!("p{idx}")))
                    })
                    .collect::<<Literal as AtomicProperty>::Set>();

                if state.is_terminated(&self.program) {
                    assignment.insert(Locator::Terminated.to_lit());
                } else if state.is_stuck(&self.program) {
                    assignment.insert(Locator::Stuck.to_lit());
                } else if self.states.first().unwrap() == state {
                    assignment.insert(Locator::Init.to_lit());
                }

                kripke.add_node(State::Real(state.clone()), assignment)
            })
            .collect();

        for (src, dsts) in self.relations.iter() {
            let worlds = &worlds;
            for dst in dsts.iter() {
                kripke.add_relation(worlds[*src], worlds[*dst]);
            }
        }
        kripke
    }

    pub fn pipeline(&self, property: &crate::ast::LTLFormula) -> Pipeline {
        let mut relational_properties = Vec::new();
        let ltl_property: mcltl::ltl::expression::LTLExpression =
            !property.to_mcltl(&mut relational_properties);
        let nnf_ltl_property: mcltl::ltl::expression::NnfLtl<Literal> = ltl_property.nnf();

        let alphabet: Alphabet<Literal> = [
            Locator::Init.to_lit(),
            Locator::Stuck.to_lit(),
            Locator::Terminated.to_lit(),
        ]
        .into_iter()
        .collect();

        let kripke: mcltl::verifier::kripke::KripkeStructure<State, Literal> =
            self.build_kripke(&relational_properties);

        let buchi: mcltl::buchi::Buchi<State, Literal> = {
            let mut buchi = kripke.to_buchi(Some(&alphabet));
            buchi.add_necessary_self_loops();
            buchi
        };

        let gbuchi_property: mcltl::buchi::GeneralBuchi<mcltl::ltl::automata::AutomataId, Literal> =
            nnf_ltl_property.gba(Some(&alphabet));

        let buchi_property: mcltl::buchi::Buchi<
            (mcltl::ltl::automata::AutomataId, usize),
            Literal,
        > = gbuchi_property.to_buchi();

        Pipeline {
            relational_properties: relational_properties.to_vec(),
            ltl_property,
            nnf_ltl_property,
            kripke,
            buchi,
            gbuchi_property,
            buchi_property,
        }
    }
}

pub struct Pipeline {
    pub relational_properties: Vec<(crate::ast::AExpr, crate::ast::RelOp, crate::ast::AExpr)>,
    pub ltl_property: mcltl::ltl::expression::LTLExpression,
    pub nnf_ltl_property: mcltl::ltl::expression::NnfLtl<Literal>,
    pub kripke: mcltl::verifier::kripke::KripkeStructure<State, Literal>,
    pub buchi: mcltl::buchi::Buchi<State, Literal>,
    pub gbuchi_property: mcltl::buchi::GeneralBuchi<mcltl::ltl::automata::AutomataId, Literal>,
    pub buchi_property: mcltl::buchi::Buchi<(mcltl::ltl::automata::AutomataId, usize), Literal>,
}

impl Pipeline {
    pub fn product_ba(
        &self,
    ) -> ProductBuchi<State, (mcltl::ltl::automata::AutomataId, usize), Literal> {
        ProductBuchi::new(&self.buchi, &self.buchi_property)
    }
}
