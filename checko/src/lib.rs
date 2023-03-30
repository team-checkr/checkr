pub mod cli;
pub mod config;
pub mod docker;
pub mod fmt;
pub mod group_env;
pub mod test_runner;

use std::{num::NonZeroUsize, path::Path, time::Duration};

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

pub fn retry<T, E>(
    tries: NonZeroUsize,
    delay: Duration,
    mut f: impl FnMut() -> Result<T, E>,
) -> Result<T, E> {
    let mut error = None;

    for _ in 0..tries.get() {
        match f() {
            Ok(res) => return Ok(res),
            Err(err) => error = Some(err),
        }
        std::thread::sleep(delay);
    }

    Err(error.unwrap())
}
