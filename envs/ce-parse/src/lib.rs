use ce_core::{
    components::{GclEditor, StandardLayout},
    define_env, rand, Env, Generate, RenderProps, ValidationResult,
};
use dioxus::prelude::*;
use gcl::ast::Commands;
use serde::{Deserialize, Serialize};

define_env!(ParseEnv);

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParseInput {
    commands: Commands,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParseOutput {
    formatted: String,
}

impl Env for ParseEnv {
    type Input = ParseInput;

    type Output = ParseOutput;

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        Ok(ParseOutput {
            formatted: input.commands.to_string(),
        })
    }

    fn validate(_input: &Self::Input, _output: &Self::Output) -> ce_core::Result<ValidationResult> {
        Ok(ValidationResult::CorrectTerminated)
    }

    fn render<'a>(cx: &'a ScopeState, props: &'a RenderProps<'a, Self>) -> Element<'a> {
        cx.render(rsx!(StandardLayout {
            input: cx.render(rsx!(GclEditor {
                commands: props.input().commands.clone(),
                on_change: move |commands| props.set_input(ParseInput { commands }),
            })),
            output: props.with_result(cx, |res| cx.render(rsx!(div {
                class: "grid grid-rows-2 divide-y",
                div {
                    h2 { class: "italic font-semibold px-2 py-1", "Real" }
                    pre {
                        class: "p-2 overflow-auto text-xs",
                        "{res.real().formatted}"
                    }
                }
                div {
                    h2 { class: "italic font-semibold px-2 py-1", "Reference" }
                    pre {
                        class: "p-2 overflow-auto text-xs",
                        "{res.reference().formatted}"
                    }
                }
            }))),
        }))
    }
}

impl Generate for ParseInput {
    type Context = ();

    fn gen<R: rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        Self {
            commands: Commands::gen(&mut Default::default(), rng),
        }
    }
}
