use ce_core::{Env, ValidationResult};
use gcl::{ast::Variable, pg::Determinism};
use stdx::stringify::Stringify;

use crate::{Input, InterpreterEnv, InterpreterMemory};

#[test]
fn initially_stuck_program() {
    let input = Input {
        commands: Stringify::Unparsed("if false -> skip fi".to_string()),
        determinism: Determinism::Deterministic,
        assignment: Default::default(),
        trace_length: 1,
    };
    let output = InterpreterEnv::run(&input).unwrap();
    match InterpreterEnv::validate(&input, &output).unwrap() {
        ValidationResult::CorrectTerminated | ValidationResult::CorrectNonTerminated { .. } => (),
        ValidationResult::Mismatch { .. } | ValidationResult::TimeOut => panic!(),
    }
}

#[test]
fn test_true_skip() {
    let commands = Stringify::Unparsed(
        r#"
    if true -> skip fi
    "#
        .to_string(),
    );

    let input = Input {
        commands,
        determinism: Determinism::Deterministic,
        assignment: InterpreterMemory {
            variables: [
                (Variable("a".to_string()), -8),
                (Variable("b".to_string()), -9),
                (Variable("c".to_string()), -3),
                (Variable("d".to_string()), -6),
            ]
            .into_iter()
            .collect(),
            arrays: Default::default(),
        },
        trace_length: 11,
    };
    let output = InterpreterEnv::run(&input).unwrap();
    match InterpreterEnv::validate(&input, &output).unwrap() {
        ValidationResult::CorrectTerminated | ValidationResult::CorrectNonTerminated { .. } => (),
        ValidationResult::Mismatch { reason } => panic!("reason: {reason:?}"),
        ValidationResult::TimeOut => panic!(),
    }
}

#[test]
fn test_thingy() {
    let commands = Stringify::Unparsed(
        r#"
        if true ->
           if false ->
              skip
           fi
        fi
    "#
        .to_string(),
    );

    let input = Input {
        commands,
        determinism: Determinism::Deterministic,
        assignment: InterpreterMemory {
            variables: [
                (Variable("a".to_string()), -8),
                (Variable("b".to_string()), -9),
                (Variable("c".to_string()), -3),
                (Variable("d".to_string()), -6),
            ]
            .into_iter()
            .collect(),
            arrays: Default::default(),
        },
        trace_length: 11,
    };
    let output = InterpreterEnv::run(&input).unwrap();
    match InterpreterEnv::validate(&input, &output).unwrap() {
        ValidationResult::CorrectTerminated | ValidationResult::CorrectNonTerminated { .. } => (),
        ValidationResult::Mismatch { reason } => panic!("reason: {reason:?}"),
        ValidationResult::TimeOut => panic!(),
    }
}
