mod dfa;
mod minimizer;
mod dfa_gen;

use ce_core::{Env, Generate, ValidationResult, define_env, rand, EnvError};
use serde::{Deserialize, Serialize};
use crate::rand::{seq::IndexedRandom};

use dfa::*;
use minimizer::*;
use dfa_gen::*;
use std::collections::{HashSet, VecDeque};

define_env!(MinimizerEnv);

#[derive(tapi::Tapi, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Input {
    dfa: String
}

#[derive(tapi::Tapi, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Output {
    dfa: String,
    dot: String, 
    minimized_dot: String,
    errors: Vec<SemanticErrorDFA>
}

impl Env for MinimizerEnv {
    type Input = Input;

    type Output = Output;

    type Meta = ();

    type Annotation = ();

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        let test_output = parse_dfa(&input.dfa)
            .map_err(ce_core::EnvError::invalid_input_for_program("failed to parse DFA"))?;
        
        let named_dfa = NamedDFA::build(test_output)
            .map_err(ce_core::EnvError::invalid_input_for_program("failed to parse DFA"))?;

        let dot = named_dfa.to_dot();

        let semantic_errors = named_dfa.dfa.validate();

        let mut minimized_dot = "".to_string();

        if semantic_errors.is_empty() {
            let minimized_dfa = named_dfa.minimize()
                .map_err(ce_core::EnvError::invalid_input_for_program("failed to minimize dfa"))?;
            minimized_dot = minimized_dfa.to_dot();
        }

        Ok( Output { dfa: format!("{:?} \n {:?}", named_dfa.dfa, named_dfa.names), dot, minimized_dot, errors: semantic_errors })        
    }

    fn validate(input: &Self::Input, output: &Self::Output) -> Result<(ValidationResult, ()), EnvError> {
        //input is for reference implementation and output is for the student
        
        let reference_dfa = parse_dfa(&input.dfa)
            .map_err(ce_core::EnvError::invalid_input_for_program("failed to parse DFA"))?;
        let reference_dfa = NamedDFA::build(reference_dfa)
            .map_err(ce_core::EnvError::invalid_input_for_program("failed to parse DFA"))?;
        let reference_dfa_min = reference_dfa.minimize()
                .map_err(ce_core::EnvError::invalid_input_for_program("failed to minimize dfa"))?;

        let their_dfa = parse_dfa(&output.dfa)
            .map_err(ce_core::EnvError::invalid_input_for_program("failed to parse DFA"))?;
        let their_dfa = NamedDFA::build(their_dfa)
            .map_err(ce_core::EnvError::invalid_input_for_program("failed to parse DFA"))?;
        let their_dfa_min =  their_dfa.minimize()
                .map_err(ce_core::EnvError::invalid_input_for_program("failed to minimize dfa"))?;

        let mut original_dfa_match = false;

        // -- original dfa validation
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back((reference_dfa.dfa.initial, their_dfa.dfa.initial, String::new()));

        while let Some((p, q, path)) = queue.pop_front() {
            if !visited.insert((p, q)) { continue; }

            if reference_dfa.dfa.is_accepting(p) != their_dfa.dfa.is_accepting(q) {
                return Ok((ValidationResult::Mismatch { reason: format!(
                    "original dfa disagrees on the string \"{path}\" - expected {}, got {}", 
                    if reference_dfa.dfa.is_accepting(p) {"accepted"} else {"rejected"}, 
                    if their_dfa.dfa.is_accepting(q) {"accepted"} else {"rejected"}
                ) }, ()));
            }

            if reference_dfa.dfa.alphabet != their_dfa.dfa.alphabet {
                return Ok((ValidationResult::Mismatch { reason: format!("alphabets differ") }, ()))
            }

            for symbol in &reference_dfa.dfa.alphabet {
                let p2 = reference_dfa.dfa.delta(p, *symbol)
                    .ok_or(MinimizationError::IncompleteInput)
                    .map_err(ce_core::EnvError::invalid_input_for_program("failted to traverse dfa"))?;
                let q2 = their_dfa.dfa.delta(q, *symbol)
                    .ok_or(MinimizationError::IncompleteInput)
                    .map_err(ce_core::EnvError::invalid_input_for_program("failed to traverse dfa"))?;
                
                queue.push_back((p2, q2, format!("{path}{symbol}")));
            }   
        }

        original_dfa_match = true;

        // -- minimized dfa validation
        // while let Some((p, q, path)) = queue.pop_front() {
        //     if !visited.insert((p, q)) { continue; }

        //     if reference_dfa_min.dfa.is_accepting(p) != their_dfa_min.dfa.is_accepting(q) {
        //         return Ok((ValidationResult::Mismatch { reason: format!(
        //             "minimized dfa disagrees on the string \"{path}\" - expected {}, got {}", 
        //             if reference_dfa_min.dfa.is_accepting(p) {"accepted"} else {"rejected"}, 
        //             if their_dfa_min.dfa.is_accepting(p) {"accepted"} else {"rejected"}
        //         ) }, ()));
        //     }

        //     if reference_dfa_min.dfa.alphabet != their_dfa_min.dfa.alphabet {
        //         return Ok((ValidationResult::Mismatch { reason: format!("alphabets differ") }, ()))
        //     }

        //     for symbol in &reference_dfa_min.dfa.alphabet {
        //         let p2 = reference_dfa_min.dfa.delta(p, *symbol)
        //             .ok_or(MinimizationError::IncompleteInput)
        //             .map_err(ce_core::EnvError::invalid_input_for_program("failted to traverse dfa"))?;
        //         let q2 = their_dfa_min.dfa.delta(q, *symbol)
        //             .ok_or(MinimizationError::IncompleteInput)
        //             .map_err(ce_core::EnvError::invalid_input_for_program("failed to traverse dfa"))?;
                
        //         queue.push_back((p2, q2, format!("{path}{symbol}")));
        //     }   
        // }

        Ok((ValidationResult::Correct, ()))
    }
}

impl Generate for Input {
    type Context = ();

    fn gn<R: rand::Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        let state_count = if rng.random_bool(0.6) {rng.random_range(2..=4)} else { rng.random_range(5..=8)};
        let allow_nondeterminism = rng.random_bool(0.3);

        Self { dfa: generate_random_dfa(rng, state_count, allow_nondeterminism) }
    }
}

