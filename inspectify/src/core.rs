use axum::Json;
use checkr::{
    ast::Commands,
    env::{graph::GraphEnvInput, Analysis, Environment, GraphEnv, Markdown},
    miette,
    pg::Determinism,
};
use serde::{Deserialize, Serialize};
use tracing::error;
use typeshare::typeshare;

/// Returns a GCL string given an analysis
#[axum::debug_handler]
pub async fn generate_program(Json(analysis): Json<Analysis>) -> Json<String> {
    let builder = analysis.setup_generation();
    builder.build().cmds.to_string().into()
}

/// Returns a `string` in DOT format
#[axum::debug_handler]
pub async fn dot(Json((deterministic, src)): Json<(bool, String)>) -> Json<String> {
    let Ok(cmds) = checkr::parse::parse_commands(&src) else {
        return "Parse error".to_string().into()
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
        .into()
}

#[axum::debug_handler]
pub async fn complete_input_from_json(
    Json((analysis, input_json)): Json<(Analysis, String)>,
) -> Json<Input> {
    let markdown = analysis
        .input_markdown(&input_json)
        .expect("failed to parse given json");
    Json(Input {
        analysis,
        json: input_json,
        markdown,
    })
}

#[axum::debug_handler]
pub async fn generate_input_for(
    Json((src, analysis)): Json<(String, Analysis)>,
) -> Json<Option<Input>> {
    let cmds = match checkr::parse::parse_commands(&src) {
        Ok(cmds) => cmds,
        Err(err) => {
            error!("Parse error: {:?}", miette::Error::new(err));
            return None.into();
        }
    };
    let mut rng = Commands::builder().build().rng;
    let json = analysis.gen_input(&cmds, &mut rng);
    let markdown = analysis
        .input_markdown(&json)
        .expect("we just generated it, so it should be fine");
    Some(Input {
        analysis,
        json,
        markdown,
    })
    .into()
}

#[axum::debug_handler]
pub async fn run_analysis(Json((src, input)): Json<(String, Input)>) -> Json<Option<Output>> {
    let cmds = match checkr::parse::parse_commands(&src) {
        Ok(cmds) => cmds,
        Err(err) => {
            error!("Parse error: {:?}", miette::Error::new(err));
            return None.into();
        }
    };
    let json = input
        .analysis
        .run(&cmds, &input.json)
        .expect("parsing input json failed");
    let markdown = input
        .analysis
        .output_markdown(&json)
        .expect("we just generated it, so it should be fine");
    Some(Output {
        analysis: input.analysis,
        json,
        markdown,
    })
    .into()
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Input {
    analysis: Analysis,
    json: String,
    markdown: Markdown,
}

#[typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Output {
    analysis: Analysis,
    json: String,
    markdown: Markdown,
}
