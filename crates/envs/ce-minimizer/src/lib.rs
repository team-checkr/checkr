mod dfa;
mod minimizer;

use ce_core::{Env, Generate, ValidationResult, define_env, rand, EnvError};
use serde::{Deserialize, Serialize};

use dfa::*;
use minimizer::*;

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

    fn validate(_input: &Self::Input, _output: &Self::Output) -> Result<(ValidationResult, ()), EnvError> {
        Ok((ValidationResult::Correct, ()))
    }
}

impl Generate for Input {
    type Context = ();

    fn gn<R: rand::Rng>(_cx: &mut Self::Context, _rng: &mut R) -> Self {
        Self {
            dfa: "".to_string()
        }
    }
}
