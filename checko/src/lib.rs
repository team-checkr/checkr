pub mod config;
pub mod fmt;
pub mod test_runner;

use std::path::Path;

use checkr::driver::{Driver, DriverError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunOption {
    pub run: String,
    pub compile: Option<String>,
    #[serde(default)]
    pub watch: Vec<String>,
}

impl RunOption {
    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
    }
    pub fn driver(&self, dir: impl AsRef<Path>) -> Result<Driver, DriverError> {
        if let Some(compile) = &self.compile {
            Driver::compile(dir, compile, &self.run)
        } else {
            Ok(Driver::new(dir, &self.run))
        }
    }
}
