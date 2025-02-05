use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RunRunOption {
    Unified(String),
    Platform { unix: String, win: String },
}

impl RunRunOption {
    pub fn run(&self) -> &str {
        match self {
            RunRunOption::Unified(run) => run,
            RunRunOption::Platform { unix, win } => {
                if cfg!(windows) {
                    win
                } else {
                    unix
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunOption {
    pub run: RunRunOption,
    #[serde(default)]
    pub compile: Option<String>,
    #[serde(default)]
    pub watch: Vec<String>,
    #[serde(default)]
    pub ignore: Vec<String>,
}

impl RunOption {
    pub fn run(&self) -> &str {
        self.run.run()
    }
}
