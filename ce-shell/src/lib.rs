#![allow(non_snake_case)]

mod def;
mod io;

pub use io::{Error, Input, Output};

pub trait EnvExt: Env {
    const ANALYSIS: Analysis;

    fn generalize_input(input: &Self::Input) -> Input;
    fn generalize_output(output: &Self::Output) -> Output;
}

define_shell!(
    ce_parse::ParseEnv[Parse, "Parse"],
    ce_graph::GraphEnv[Graph, "Graph"],
    ce_sign::SignEnv[Sign, "Sign Analysis"],
);
