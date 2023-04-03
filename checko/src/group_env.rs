use crate::config::GroupConfig;

use color_eyre::Result;
use tracing::info;
use xshell::{cmd, Shell, TempDir};

pub struct GroupEnv<'a> {
    dir: TempDir,
    config: &'a GroupConfig,
}

impl<'a> GroupEnv<'a> {
    pub fn new(config: &'a GroupConfig) -> Result<Self> {
        let sh = Shell::new()?;
        let dir = sh.create_temp_dir()?;
        Ok(Self { dir, config })
    }
    pub fn shell_in_default_branch(&self) -> Result<Shell> {
        let sh = Shell::new()?;
        sh.change_dir(self.dir.path());

        let before_clone = std::time::Instant::now();
        let git = &self.config.git;

        info!(
            repo = git,
            dst = self.dir.path().display().to_string(),
            "cloning"
        );

        macro_rules! cmdq {
            ($($t:tt)*) => {
                cmd!($($t)*).ignore_stdout().ignore_stderr().quiet()
            };
        }

        cmdq!(sh, "git init").run()?;
        set_checko_git_account(&sh)?;
        cmdq!(sh, "git remote add -f origin {git}").run()?;
        cmdq!(sh, "git config core.sparsecheckout true").run()?;
        sh.write_file(".git/info/sparse-checkout", "/*\n!dev/")?;
        // TODO: This should not be hardcoded to master, but rather look up the default branch
        // git rev-list -n 1 --before="2023-02-01" main
        cmdq!(sh, "git pull origin master").run()?;

        info!(took = format!("{:?}", before_clone.elapsed()), "cloned");

        // TODO: possibly change to the latest commit just before a deadline

        Ok(sh)
    }
}

pub fn set_checko_git_account(sh: &Shell) -> Result<()> {
    cmd!(sh, "git config user.name Checko")
        .ignore_stdout()
        .ignore_stderr()
        .quiet()
        .run()?;
    cmd!(sh, "git config user.email 'checko@checko.org'")
        .ignore_stdout()
        .ignore_stderr()
        .quiet()
        .run()?;
    Ok(())
}
