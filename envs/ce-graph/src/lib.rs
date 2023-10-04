use ce_core::{
    components::{GclEditor, Network, StandardLayout},
    define_env, Env, Generate, ValidationResult,
};
use dioxus::prelude::*;
use gcl::{
    ast::Commands,
    pg::{Determinism, ProgramGraph},
};
use serde::{Deserialize, Serialize};

define_env!(GraphEnv);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphInput {
    commands: Commands,
    deterministic: Determinism,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphOutput {
    dot: String,
}

impl Env for GraphEnv {
    type Input = GraphInput;

    type Output = GraphOutput;

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        let dot = ProgramGraph::new(input.deterministic, &input.commands).dot();
        Ok(GraphOutput { dot })
    }

    fn validate(_input: &Self::Input, _output: &Self::Output) -> ce_core::Result<ValidationResult> {
        Ok(ValidationResult::CorrectTerminated)
    }

    fn render<'a>(cx: &'a ScopeState, props: &'a ce_core::RenderProps<'a, Self>) -> Element<'a> {
        cx.render(rsx!(StandardLayout {
            input: cx.render(rsx!(GclEditor {
                commands: props.input().commands.clone(),
                on_change: move |commands| props.set_input(GraphInput {
                    commands,
                    deterministic: Determinism::Deterministic
                }),
            })),
            output: props.with_result(cx, |res| cx.render(rsx!(
                div {
                    class: "grid grid-rows-2 divide-y",
                    div {
                        class: "grid grid-rows-[auto_1fr]",
                        h2 { class: "italic font-semibold px-2 py-1", "Real" }
                        Network {
                            dot: res.real().dot.clone()
                        }
                    }
                    div {
                        class: "grid grid-rows-[auto_1fr]",
                        h2 { class: "italic font-semibold px-2 py-1", "Reference" }
                        Network {
                            dot: res.reference().dot.clone()
                        }
                    }
                }
            )))
        }))
    }
}

impl Generate for GraphInput {
    type Context = ();

    fn gen<R: ce_core::rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        GraphInput {
            commands: Commands::gen(&mut Default::default(), rng),
            deterministic: Determinism::NonDeterministic,
        }
    }
}
