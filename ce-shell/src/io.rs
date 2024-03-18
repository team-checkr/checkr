use std::sync::Arc;

use ce_core::Env;

use crate::{Analysis, EnvExt};

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Input {
    analysis: Analysis,
    json: Arc<serde_json::Value>,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Output {
    analysis: Analysis,
    json: Arc<serde_json::Value>,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Meta {
    analysis: Analysis,
    json: Arc<serde_json::Value>,
}

impl Input {
    pub fn new<E: EnvExt>(data: &E::Input) -> Self {
        Self {
            analysis: E::ANALYSIS,
            json: serde_json::to_value(data)
                .expect("all output should be serializable")
                .into(),
        }
    }

    pub fn analysis(&self) -> Analysis {
        self.analysis
    }

    pub fn json(&self) -> Arc<serde_json::Value> {
        self.json.clone()
    }

    pub fn data<E: Env>(&self) -> Result<E::Input, serde_json::Error> {
        serde_json::from_value((*self.json).clone())
    }

    pub fn hash(&self) -> [u8; 16] {
        md5::compute(format!("{:?}::{self}", self.analysis())).0
    }
}

impl Output {
    pub fn new<E: EnvExt>(data: &E::Output) -> Self {
        Self {
            analysis: E::ANALYSIS,
            json: serde_json::to_value(data)
                .expect("all output should be serializable")
                .into(),
        }
    }

    pub fn analysis(&self) -> Analysis {
        self.analysis
    }

    pub fn json(&self) -> Arc<serde_json::Value> {
        self.json.clone()
    }

    pub fn data<E: Env>(&self) -> Result<E::Output, serde_json::Error> {
        serde_json::from_value((*self.json).clone())
    }

    pub fn hash(&self) -> [u8; 16] {
        md5::compute(format!("{:?}::{self}", self.analysis())).0
    }
}

impl Meta {
    pub fn new<E: EnvExt>(data: &E::Meta) -> Self {
        Self {
            analysis: E::ANALYSIS,
            json: serde_json::to_value(data)
                .expect("all output should be serializable")
                .into(),
        }
    }

    pub fn analysis(&self) -> Analysis {
        self.analysis
    }

    pub fn json(&self) -> Arc<serde_json::Value> {
        self.json.clone()
    }

    pub fn data<E: Env>(&self) -> Result<E::Meta, serde_json::Error> {
        serde_json::from_value((*self.json).clone())
    }
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.json.fmt(f)
    }
}
impl std::fmt::Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.json.fmt(f)
    }
}
impl std::fmt::Display for Meta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.json.fmt(f)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),
}
