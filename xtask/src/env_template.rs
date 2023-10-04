use ce_core::{
    components::StandardLayout, define_env, rand, Env, Generate, RenderProps, ValidationResult,
};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

define_env!(TemplateEnv);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateInput {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateOutput {}

impl Env for TemplateEnv {
    type Input = TemplateInput;

    type Output = TemplateOutput;

    fn run(_input: &Self::Input) -> ce_core::Result<Self::Output> {
        Ok(TemplateOutput {})
    }

    fn validate(_input: &Self::Input, _output: &Self::Output) -> ce_core::Result<ValidationResult> {
        Ok(ValidationResult::CorrectTerminated)
    }

    fn render<'a>(cx: &'a ScopeState, _props: &'a RenderProps<'a, Self>) -> Element<'a> {
        cx.render(rsx!(StandardLayout {
            input: cx.render(rsx!(div {
                class: "grid place-items-center",
                "Input goes here"
            })),
            output: cx.render(rsx!(div {
                class: "grid place-items-center",
                "Output goes here"
            })),
        }))
    }
}

impl Generate for TemplateInput {
    type Context = ();

    fn gen<R: rand::Rng>(_cx: &mut Self::Context, _rng: &mut R) -> Self {
        Self {}
    }
}
