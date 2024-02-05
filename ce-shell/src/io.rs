use std::sync::Arc;

use crate::Analysis;

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Input {
    pub(crate) analysis: Analysis,
    pub(crate) json: Arc<serde_json::Value>,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Output {
    pub(crate) analysis: Analysis,
    pub(crate) json: Arc<serde_json::Value>,
}

impl Input {
    pub fn analysis(&self) -> Analysis {
        self.analysis
    }

    pub fn data(&self) -> Arc<serde_json::Value> {
        self.json.clone()
    }
}

impl Output {
    pub fn analysis(&self) -> Analysis {
        self.analysis
    }

    pub fn data(&self) -> Arc<serde_json::Value> {
        self.json.clone()
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

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),
}
