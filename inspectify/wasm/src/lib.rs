use checkr::{
    ast::Commands,
    env::{
        graph::{GraphEnv, GraphEnvInput},
        Analysis, Environment, Markdown,
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
pub fn complete_input_from_json(analysis: Analysis, input_json: String) -> Input {
    let markdown = analysis
        .input_markdown(&input_json)
        .expect("failed to parse given json");
    Input {
        analysis,
        json: input_json,
        markdown,
    }
}

#[wasm_bindgen]
pub fn generate_input_for(src: &str, analysis: Analysis) -> Input {
    let Ok(cmds) = checkr::parse::parse_commands(src) else {
        todo!("Parse error");
    };
    let mut rng = Commands::builder().build().rng;
    let json = analysis.gen_input(&cmds, &mut rng);
    let markdown = analysis
        .input_markdown(&json)
        .expect("we just generated it, so it should be fine");
    Input {
        analysis,
        json,
        markdown,
    }
}

#[wasm_bindgen]
pub fn run_analysis(src: &str, input: Input) -> Output {
    let Ok(cmds) = checkr::parse::parse_commands(src) else {
        todo!("Parse error");
    };
    let json = input
        .analysis
        .run(&cmds, &input.json)
        .expect("parsing input json failed");
    let markdown = input
        .analysis
        .output_markdown(&json)
        .expect("we just generated it, so it should be fine");
    Output {
        analysis: input.analysis,
        json,
        markdown,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct Input {
    analysis: Analysis,
    json: String,
    markdown: Markdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct Output {
    analysis: Analysis,
    json: String,
    markdown: Markdown,
}
