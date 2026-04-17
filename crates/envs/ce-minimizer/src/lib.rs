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
    //dfa: String,
    dot: String, 
    minimized_dot: String,
    //errors: Vec<SemanticErrorDFA>
    deterministic: bool
}

impl Env for MinimizerEnv {
    type Input = Input;

    type Output = Output;

    type Meta = ();

    type Annotation = ();

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        let test_output = parse_dfa(&input.dfa)
            .map_err(ce_core::EnvError::invalid_input_for_program("failed to parse DFA"))?;
        
        let mut named_dfa = NamedDFA::build(test_output)
            .map_err(ce_core::EnvError::invalid_input_for_program("failed to parse DFA"))?;

        let dot = named_dfa.to_dot();

        let deterministic = named_dfa.dfa.check_determinism();

        let mut minimized_dot = "".to_string();

        if deterministic {
            let minimized_dfa = named_dfa.minimize()
                .map_err(ce_core::EnvError::invalid_input_for_program("failed to minimize dfa"))?;
            minimized_dot = minimized_dfa.to_dot();
        }

        Ok( Output { dot, minimized_dot, deterministic})        
    }

    fn validate(input: &Self::Input, output: &Self::Output) -> Result<(ValidationResult, ()), EnvError> {
        //input is for reference implementation and output is for the student

        Ok((ValidationResult::Correct, ()))
    }
}

impl Generate for Input {
    type Context = ();

    fn gn<R: rand::Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        let state_count = if rng.random_bool(0.6) {rng.random_range(2..=4)} else { rng.random_range(5..=8)};
        let allow_nondeterminism = rng.random_bool(0.1);

        Self { dfa: generate_random_dfa(rng, state_count, allow_nondeterminism) }
    }
}

