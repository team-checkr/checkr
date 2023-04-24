use std::path::Path;

use crate::driver::{Driver, DriverError};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunOption {
    pub run: String,
    pub compile: Option<String>,
    #[serde(default)]
    pub watch: Vec<String>,
    #[serde(default)]
    pub ignore: Vec<String>,
}

impl RunOption {
    pub async fn driver(&self, dir: impl AsRef<Path>) -> Result<Driver, DriverError> {
        if let Some(compile) = &self.compile {
            Driver::compile(dir, compile, &self.run).await
        } else {
            Ok(Driver::new(dir, &self.run))
        }
    }
}
