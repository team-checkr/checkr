use serde::{Deserialize, Serialize};

use crate::{ast::Commands, generation::Generate};

use super::{Analysis, EnvError, Environment, ToMarkdown, ValidationResult};

#[derive(Debug)]
pub struct ParseEnv;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseInput {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseOutput(String);

impl Environment for ParseEnv {
    type Input = ParseInput;

    type Output = ParseOutput;

    const ANALYSIS: Analysis = Analysis::Parse;

    fn run(&self, cmds: &Commands, _input: &Self::Input) -> Result<Self::Output, EnvError> {
        Ok(ParseOutput(cmds.to_string()))
    }

    fn validate(
        &self,
        _cmds: &Commands,
        _input: &Self::Input,
        _output: &Self::Output,
    ) -> Result<ValidationResult, EnvError> {
        Ok(ValidationResult::CorrectTerminated)
    }
}

impl Generate for ParseInput {
    type Context = Commands;

    fn gen<R: rand::Rng>(_cx: &mut Self::Context, _rng: &mut R) -> Self {
        Self {}
    }
}

impl ToMarkdown for ParseInput {
    fn to_markdown(&self) -> super::Markdown {
        super::Markdown(String::new())
    }
}
impl ToMarkdown for ParseOutput {
    fn to_markdown(&self) -> super::Markdown {
        super::Markdown(format!("```\n{}\n```", self.0))
    }
}
