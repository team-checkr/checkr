use checkr::{
    ast::Commands,
    env::{
        graph::{GraphEnv, GraphEnvInput},
        Analysis, Environment,
    },
    pg::Determinism,
};
use serde::{Deserialize, Serialize};
use tsify::Tsify;
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
pub fn generate_program(analysis: Analysis) -> String {
    let builder = analysis.setup_generation();
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

#[wasm_bindgen]
pub fn generate_sample_for(src: &str, analysis: Analysis) -> Sample {
    let Ok(cmds) = checkr::parse::parse_commands(src) else {
        todo!("Parse error");
    };
    let mut rng = Commands::builder().build().rng;
    let sample = analysis.gen_sample(&cmds, &mut rng);
    sample.into()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct Sample {
    pub input_json: String,
    pub input_markdown: String,
    pub output_markdown: String,
}

impl From<checkr::env::Sample> for Sample {
    fn from(value: checkr::env::Sample) -> Self {
        Sample {
            input_json: value.input_json.to_string(),
            input_markdown: value.input_markdown,
            output_markdown: value.output_markdown,
        }
    }
}
