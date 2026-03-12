mod dfa;

use ce_core::{Env, Generate, ValidationResult, define_env, rand};
use serde::{Deserialize, Serialize};

use dfa::*;

define_env!(MinimizerEnv);

#[derive(tapi::Tapi, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Input {
    dfa: String
}

#[derive(tapi::Tapi, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Output {
    dfa: String,
    dot: String, 
    minimized_dot: String
}

impl Env for MinimizerEnv {
    type Input = Input;

    type Output = Output;

    type Meta = ();

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        eprintln!("run called with: {:?}", input.dfa);
        
        let test_output = parse_dfa(&input.dfa)
            .map_err(ce_core::EnvError::invalid_input_for_program("failed to parse DFA"))?;
        
        let dfa = NamedDFA::build(test_output)
            .map_err(ce_core::EnvError::invalid_input_for_program("failed to parse DFA"))?;

        let dot = dfa.to_dot();

        Ok( Output { dfa: format!("{:?} \n {:?}", dfa.dfa, dfa.names), dot, ..Default::default() })
        
    }

    fn validate(_input: &Self::Input, _output: &Self::Output) -> ce_core::Result<ValidationResult> {
        Ok(ValidationResult::Correct)
    }
}

impl Generate for Input {
    type Context = ();

    fn gn<R: rand::Rng>(_cx: &mut Self::Context, _rng: &mut R) -> Self {
        Self {
            dfa: "states: q0 q1\nalphabet: a b\ninitial: q0\naccepting: q1\ntransitions:\nq0,a -> q1\nq1,b -> q0".to_string()
        }
    }
}
