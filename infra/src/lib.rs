use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunOption {
    pub run: String,
    pub compile: Option<String>,
}

impl RunOption {
    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
    }
}
