use axum::Json;
use checkr::{
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
        .expect("the input was just given, so it should work")
        .dot
        .into()
}

#[axum::debug_handler]
pub async fn complete_input_from_json(
    Json((analysis, input_json)): Json<(Analysis, String)>,
) -> Json<Input> {
    let input = analysis
        .input_from_str(&input_json)
        .expect("failed to parse input json");
    let markdown = input.to_markdown().expect("failed to parse given json");
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
    use rand::SeedableRng;
    let mut rng = rand::rngs::SmallRng::from_entropy();
    let json = analysis.gen_input(&cmds, &mut rng);
    let markdown = json
        .to_markdown()
        .expect("we just generated it, so it should be fine");
    Some(Input {
        analysis,
        json: json.to_string(),
        markdown,
    })
    .into()
}

#[axum::debug_handler]
pub async fn run_analysis(Json((src, input)): Json<(String, Input)>) -> Json<Option<Output>> {
    let input_json = input
        .analysis
        .input_from_str(&input.json)
        .expect("failed to parse input json");
    let cmds = match checkr::parse::parse_commands(&src) {
        Ok(cmds) => cmds,
        Err(err) => {
            error!("Parse error: {:?}", miette::Error::new(err));
            return None.into();
        }
    };
    let json = if let Ok(json) = input.analysis.run(&cmds, input_json) {
        json
    } else {
        return Json(None);
    };
    let markdown = json
        .to_markdown()
        .expect("we just generated it, so it should be fine");
    Some(Output {
        analysis: input.analysis,
        json: json.to_string(),
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
