#![allow(non_snake_case)]

mod def;
mod io;

pub use io::{Error, Hash, Input, Meta, Output};

pub trait EnvExt: Env {
    const ANALYSIS: Analysis;

    fn generalize_input(input: &Self::Input) -> Input;
    fn generalize_output(output: &Self::Output) -> Output;
}

define_shell!(
    ce_calculator::CalcEnv[Calculator, "Calculator"],
    ce_parser::ParserEnv[Parser, "Parser"],
    ce_compiler::CompilerEnv[Compiler, "Compiler"],
    ce_interpreter::InterpreterEnv[Interpreter, "Interpreter"],
    ce_security::SecurityEnv[Security, "Security"],
    ce_sign::SignEnv[Sign, "Sign Analysis"],
);
