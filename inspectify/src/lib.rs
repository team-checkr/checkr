pub mod compilation;
mod core;
pub mod routes;

use std::sync::Arc;

use checkr::driver::Driver;
use compilation::CompilationStatus;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum ValidationResult {
    CorrectTerminated,
    CorrectNonTerminated { iterations: u32 },
    Mismatch { reason: String },
    TimeOut,
}

impl From<checkr::env::ValidationResult> for ValidationResult {
    fn from(r: checkr::env::ValidationResult) -> Self {
        use checkr::env::ValidationResult as VR;

        match r {
            VR::CorrectTerminated => ValidationResult::CorrectTerminated,
            VR::CorrectNonTerminated { iterations } => ValidationResult::CorrectNonTerminated {
                iterations: iterations as _,
            },
            VR::Mismatch { reason } => ValidationResult::Mismatch { reason },
            VR::TimeOut => ValidationResult::TimeOut,
        }
    }
}

#[derive(Clone)]
pub struct ApplicationState {
    pub driver: Arc<Mutex<Driver>>,
    pub compilation_status: Arc<Mutex<CompilationStatus>>,
}

pub async fn do_self_update() -> color_eyre::Result<()> {
    binswap_github::builder()
        .repo_author("team-checkr")
        .repo_name("checkr")
        .bin_name("inspectify")
        .build()?
        .fetch_and_write_in_place_of_current_exec()
        .await?;

    Ok(())
}

pub fn clear_terminal() -> std::io::Result<std::io::Stdout> {
    use crossterm::{cursor, terminal, ExecutableCommand};
    use std::io::stdout;

    let mut stdout = stdout();
    stdout
        .execute(terminal::Clear(terminal::ClearType::All))?
        .execute(cursor::MoveTo(0, 0))?;
    Ok(stdout)
}
