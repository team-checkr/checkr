#![allow(non_snake_case)]

mod def;
mod io;

pub use io::{Input, Output};

define_shell!(
    ce_graph::GraphEnv[Graph, "Graph"],
    ce_parse::ParseEnv[Parse, "Parse"],
    ce_pv::PvEnv[Pv, "Program Verification"],
    ce_sign::SignEnv[Sign, "Sign Analysis"],
);
