pub mod config;
pub mod docker;
pub mod fmt;
pub mod test_runner;

use std::path::Path;

use checkr::config::RunOption;
use color_eyre::{eyre::Context, Result};

pub fn run_options_from_file(path: impl AsRef<Path>) -> Result<RunOption> {
    let p = path.as_ref();
    let src = std::fs::read_to_string(p)
        .wrap_err_with(|| format!("could not read run options at {p:?}"))?;
    let parsed = toml::from_str(&src)
        .wrap_err_with(|| format!("error parsing run options from file {p:?}"))?;
    Ok(parsed)
}
