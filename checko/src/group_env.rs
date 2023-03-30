use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{config::GroupConfig, retry, test_runner::TestRunResults};

use color_eyre::{eyre::Context, Result};
use tracing::{info, warn};
use xshell::{cmd, Shell};

pub struct GroupEnv<'a> {
    submissions_folder: &'a Path,
    config: &'a GroupConfig,
}

impl<'a> GroupEnv<'a> {
    pub fn new(submissions_folder: &'a Path, config: &'a GroupConfig) -> Self {
        Self {
            submissions_folder,
            config,
        }
    }

    pub fn latest_run_path(&self) -> PathBuf {
        self.group_folder().join("run.json")
    }
    pub fn write_latest_run(&self, run: &TestRunResults) -> Result<()> {
        fs::write(self.latest_run_path(), serde_json::to_string(run)?)?;
        Ok(())
    }
    pub fn latest_run(&self) -> Result<TestRunResults> {
        let p = self.latest_run_path();
        let src = fs::read_to_string(&p)
            .wrap_err_with(|| format!("could not read latest run at {p:?}"))?;
        let parsed = serde_json::from_str(&src)
            .wrap_err_with(|| format!("error parsing latest run from file {p:?}"))?;
        Ok(parsed)
    }
    fn group_folder(&self) -> PathBuf {
        self.submissions_folder.join(&self.config.name)
    }
    fn shell_in_folder(&self) -> Result<Shell> {
        let g_dir = self.group_folder();
        let sh = Shell::new()?;
        sh.create_dir(&g_dir)?;
        sh.change_dir(&g_dir);
        Ok(sh)
    }
    pub fn shell_in_default_branch(&self) -> Result<Shell> {
        let sh = self.shell_in_folder()?;
        sh.remove_path("repo")?;

        let before_clone = std::time::Instant::now();
        let git = &self.config.git;
        let dst = sh.current_dir().join("repo");
        info!(repo = git, dst = dst.display().to_string(), "cloning");
        cmd!(sh, "git clone --filter=blob:none --no-checkout {git} {dst}")
            .ignore_stdout()
            .ignore_stderr()
            .quiet()
            .run()?;

        sh.change_dir("repo");

        // TODO: This should not be hardcoded to master, but rather look up the default branch
        cmd!(sh, "git checkout master")
            .ignore_stdout()
            .ignore_stderr()
            .quiet()
            .run()?;
        info!(took = format!("{:?}", before_clone.elapsed()), "cloned");

        // TODO: possibly change to the latest commit just before a deadline

        Ok(sh)
    }
    pub fn shell_in_results_branch(&self) -> Result<Shell> {
        let sh = self.shell_in_default_branch()?;

        retry(
            5.try_into().expect("it's positive"),
            Duration::from_millis(500),
            || -> Result<()> {
                if cmd!(sh, "git checkout results").run().is_err() {
                    cmd!(sh, "git switch --orphan results").run()?;
                }
                cmd!(sh, "git reset --hard").run()?;
                cmd!(sh, "git clean -xdf").run()?;
                if let Err(err) = cmd!(sh, "git pull").run() {
                    warn!("failed to pull, but continuing anyway");
                    eprintln!("{err:?}");
                }
                Ok(())
            },
        )?;

        Ok(sh)
    }
}
