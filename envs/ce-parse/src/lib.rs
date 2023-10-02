use ce_core::{
    basic_env_test,
    components::{GclEditor, StandardLayout},
    rand, Env, Generate, RenderProps, ValidationResult,
};
use dioxus::prelude::*;
use gcl::ast::Commands;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
pub struct ParseEnv;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParseInput {
    commands: Commands,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
                commands: props.input.commands.clone(),
                on_change: move |commands| props.set_input(ParseInput { commands }),
            })),
            output: cx.render(rsx!(div {
                class: "grid grid-rows-2",
                pre {
                    class: "rounded border shadow p-2 overflow-auto text-xs ml-2",
                    "{props.real_output.formatted}"
                }
                pre {
                    class: "rounded border shadow p-2 overflow-auto text-xs mr-2",
                    "{props.reference_output.formatted}"
                }
            })),
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

basic_env_test!(ParseEnv);
