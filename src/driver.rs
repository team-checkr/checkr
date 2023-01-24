use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use tracing::error;

use crate::{ast::Commands, env::Environment};

pub struct Driver {
    dir: PathBuf,
    run_cmd: String,
    compile_output: Option<std::process::Output>,
}

#[derive(Debug, thiserror::Error)]
pub enum DriverError {
    #[error("running compile failed")]
    RunCompile(#[source] std::io::Error),
    #[error("failed to compile:\n  {}", std::str::from_utf8(&_0.stdout).unwrap())]
    CompileFailure(std::process::Output),
}

#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error(transparent)]
    Serialize(serde_json::Error),
    #[error("running exec failed")]
    RunExec(#[source] std::io::Error),
    #[error("command failed:\n{:?}", _0.stdout)]
    CommandFailed(std::process::Output, Duration),
    #[error("parse failed")]
    Parse {
        #[source]
        inner: serde_json::Error,
        run_output: std::process::Output,
        time: Duration,
    },
}

impl Driver {
    pub fn new(dir: impl AsRef<Path>, run_cmd: &str) -> Driver {
        Driver {
            dir: dir.as_ref().to_owned(),
            run_cmd: run_cmd.to_string(),
            compile_output: None,
        }
    }
    pub fn compile(
        dir: impl AsRef<Path>,
        compile: &str,
        run_cmd: &str,
    ) -> Result<Driver, DriverError> {
        let mut args = compile.split(' ');
        let program = args.next().unwrap();

        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd.current_dir(&dir);

        let compile_output = cmd.output().map_err(DriverError::RunCompile)?;

        if !compile_output.status.success() {
            return Err(DriverError::CompileFailure(compile_output));
        }

        Ok(Driver {
            dir: dir.as_ref().to_owned(),
            run_cmd: run_cmd.to_string(),
            compile_output: Some(compile_output),
        })
    }
    fn new_command(&self) -> Command {
        let mut args = self.run_cmd.split(' ');

        let mut cmd = Command::new(args.next().unwrap());
        cmd.args(args);
        cmd.current_dir(&self.dir);

        cmd
    }
    pub fn exec_raw_cmds<E>(&self, cmds: &str, input: &E::Input) -> Result<ExecOutput<E>, ExecError>
    where
        E: Environment,
    {
        let mut cmd = self.new_command();
        cmd.arg(E::command());
        cmd.arg(cmds.to_string());

        cmd.arg(serde_json::to_string(input).map_err(ExecError::Serialize)?);

        let before = std::time::Instant::now();
        let cmd_output = cmd.output().map_err(ExecError::RunExec)?;
        let took = before.elapsed();

        if !cmd_output.status.success() {
            // error!(
            //     stdout = std::str::from_utf8(&cmd_output.stdout).unwrap(),
            //     stderr = std::str::from_utf8(&cmd_output.stderr).unwrap(),
            //     "failed to run command",
            // );
            return Err(ExecError::CommandFailed(cmd_output, took));
        }

        match serde_json::from_slice(&cmd_output.stdout) {
            Ok(parsed) => Ok(ExecOutput {
                output: cmd_output,
                parsed,
                took,
            }),
            Err(err) => Err(ExecError::Parse {
                inner: err,
                run_output: cmd_output,
                time: took,
            }),
        }
    }
    pub fn exec<E>(&self, cmds: &Commands, input: &E::Input) -> Result<ExecOutput<E>, ExecError>
    where
        E: Environment,
    {
        self.exec_raw_cmds(&cmds.to_string(), input)
    }

    pub fn compile_output(&self) -> Option<&std::process::Output> {
        self.compile_output.as_ref()
    }
}

#[derive(Debug)]
pub struct ExecOutput<E: Environment> {
    pub output: std::process::Output,
    pub parsed: E::Output,
    pub took: Duration,
}
