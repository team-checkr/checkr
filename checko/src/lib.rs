mod batch;
pub mod cli;
mod config;
mod docker;
mod fmt;
mod group_env;
mod test_runner;
mod ui;

use std::{fs, num::NonZeroUsize, path::Path, time::Duration};

use checkr::config::RunOption;
use color_eyre::{eyre::Context, Result};
use config::{GroupsConfig, ProgramsConfig};

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

pub fn read_programs(programs: impl AsRef<Path>) -> Result<ProgramsConfig> {
    let p = programs.as_ref();
    let src =
        fs::read_to_string(p).wrap_err_with(|| format!("could not read programs at {p:?}"))?;
    let parsed =
        toml::from_str(&src).wrap_err_with(|| format!("error parsing programs from file {p:?}"))?;
    Ok(parsed)
}
pub fn read_groups(groups: impl AsRef<Path>) -> Result<GroupsConfig> {
    let p = groups.as_ref();
    let src = fs::read_to_string(p).wrap_err_with(|| format!("could not read groups at {p:?}"))?;
    let parsed =
        toml::from_str(&src).wrap_err_with(|| format!("error parsing groups from file {p:?}"))?;
    Ok(parsed)
}
pub fn collect_programs(
    programs: impl IntoIterator<Item = impl AsRef<Path>>,
) -> Result<ProgramsConfig> {
    programs
        .into_iter()
        .map(read_programs)
        .reduce(|acc, p| {
            let mut acc = acc?;
            acc.extend(p?);
            Ok(acc)
        })
        .unwrap_or_else(|| Ok(Default::default()))
}
