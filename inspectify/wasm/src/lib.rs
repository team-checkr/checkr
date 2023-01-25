use checkr::{
    ast::Commands,
    env::{
        graph::{GraphEnv, GraphEnvInput},
        pv::ProgramVerificationEnv,
        Analysis, AnyEnvironment, Environment, InterpreterEnv, Sample, SecurityEnv, SignEnv,
    },
    pg::Determinism,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn init_hook() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
}

/// Returns a GCL string given an analysis
#[wasm_bindgen]
pub fn generate_program(analysis: String) -> String {
    let analysis: Analysis = analysis.parse().unwrap();
    let builder = match analysis {
        Analysis::Graph => GraphEnv.setup_generation(),
        Analysis::Interpreter => InterpreterEnv.setup_generation(),
        Analysis::ProgramVerification => ProgramVerificationEnv.setup_generation(),
        Analysis::Sign => SignEnv.setup_generation(),
        Analysis::Security => SecurityEnv.setup_generation(),
    };
    builder.build().cmds.to_string()
}

/// Returns a `string` in DOT format
#[wasm_bindgen]
pub fn dot(deterministic: bool, src: &str) -> String {
    let Ok(cmds) = checkr::parse::parse_commands(src) else {
            return "Parse error".to_string()
        };
    GraphEnv
        .run(
            &cmds,
            &GraphEnvInput {
                determinism: if deterministic {
                    Determinism::Deterministic
                } else {
                    Determinism::NonDeterministic
                },
            },
        )
        .dot
}

/// Returns a `Sample`
#[wasm_bindgen]
pub fn security(src: &str) -> String {
    let Ok(cmds) = checkr::parse::parse_commands(src) else {
            return "Parse error".to_string()
        };
    let mut rng = Commands::builder().build().rng;
    let sample = SecurityEnv.gen_sample(&cmds, &mut rng);
    serde_json::to_string(&sample).unwrap()
}

/// Returns a `Sample`
#[wasm_bindgen]
pub fn interpreter(src: &str) -> String {
    let Ok(cmds) = checkr::parse::parse_commands(src) else {
            return "Parse error".to_string()
        };
    let mut rng = Commands::builder().build().rng;
    let sample = InterpreterEnv.gen_sample(&cmds, &mut rng);
    serde_json::to_string(&sample).unwrap()
}

/// Returns a `Sample`
#[wasm_bindgen]
pub fn sign(src: &str) -> String {
    let Ok(cmds) = checkr::parse::parse_commands(src) else {
            return "Parse error".to_string()
        };
    let mut rng = Commands::builder().build().rng;
    let sample = SignEnv.gen_sample(&cmds, &mut rng);
    serde_json::to_string(&sample).unwrap()
}

/// Returns a `Sample`
#[wasm_bindgen]
pub fn pv(src: &str) -> String {
    let Ok(cmds) = checkr::parse::parse_commands(src) else {
            return "Parse error".to_string()
        };
    let mut rng = Commands::builder().build().rng;
    let sample = ProgramVerificationEnv.gen_sample(&cmds, &mut rng);
    serde_json::to_string(&sample).unwrap()
}

#[typeshare::typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Env {
    analysis: Analysis,
    sample: Sample,
}
#[typeshare::typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Generation {
    program: String,
    dot: String,
    envs: Vec<Env>,
}
