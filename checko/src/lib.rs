pub mod config;
pub mod fmt;
pub mod test_runner;

use std::path::Path;

use checkr::driver::{Driver, DriverError};
use color_eyre::{eyre::Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunOption {
    pub run: String,
    pub compile: Option<String>,
    #[serde(default)]
    pub watch: Vec<String>,
}

impl RunOption {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let p = path.as_ref();
        let src = std::fs::read_to_string(p)
            .wrap_err_with(|| format!("could not read run options at {p:?}"))?;
        let parsed = toml::from_str(&src)
            .wrap_err_with(|| format!("error parsing run options from file {p:?}"))?;
        Ok(parsed)
    }
    pub fn driver(&self, dir: impl AsRef<Path>) -> Result<Driver, DriverError> {
        if let Some(compile) = &self.compile {
            Driver::compile(dir, compile, &self.run)
        } else {
            Ok(Driver::new(dir, &self.run))
        }
    }
}
