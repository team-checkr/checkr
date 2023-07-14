use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use tokio::process::Command;
use tracing::error;

use crate::{
    ast::Commands,
    env::{Analysis, EnvError, Environment, Output},
};

pub struct Driver {
    dir: PathBuf,
    run_cmd: String,
    compile_output: Option<std::process::Output>,
}

#[derive(Debug, thiserror::Error)]
pub enum DriverError {
    #[error("running compile failed")]
    RunCompile(#[source] std::io::Error),
    #[error("failed to compile:\n  {}\n\n  {}", std::str::from_utf8(&_0.stdout).unwrap(), std::str::from_utf8(&_0.stderr).unwrap())]
    CompileFailure(std::process::Output),
}

#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error(transparent)]
    Serialize(serde_json::Error),
    #[error("running `{cmd}` failed")]
    RunExec {
        cmd: String,
        #[source]
        source: std::io::Error,
    },
    #[error("command failed:\n  {}\n\n  {}", std::str::from_utf8(&_0.stdout).unwrap(), std::str::from_utf8(&_0.stderr).unwrap())]
    CommandFailed(std::process::Output, Duration),
    #[error("parse failed")]
    Parse {
        #[source]
        inner: EnvError,
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
    pub async fn compile(
        dir: impl AsRef<Path>,
        compile: &str,
        run_cmd: &str,
    ) -> Result<Driver, DriverError> {
        let mut args = compile.split(' ');
        let program = args.next().unwrap();

        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd.current_dir(&dir);

        let compile_output = cmd.output().await.map_err(DriverError::RunCompile)?;

        if !compile_output.status.success() {
            return Err(DriverError::CompileFailure(compile_output));
        }

        Ok(Driver {
            dir: dir.as_ref().to_owned(),
            run_cmd: run_cmd.to_string(),
            compile_output: Some(compile_output),
        })
    }

    //Generates a string with initial dot removed (if it contains this), and any initial path / or \, as well as doing the same from the back
    fn clear_format(st: String) -> String {
        let mut process: std::collections::VecDeque<char> =
            std::collections::VecDeque::from(st.chars().collect::<Vec<char>>());

        //Remove initial dot if it exists and there is only 1 of them
        if !process.is_empty() && *process.front().unwrap() == '.' {
            process.pop_front();
            if !process.is_empty() && *process.front().unwrap() == '.' {
                process.push_front('.');
            }
        }

        //Remove / and \ from the front
        while !process.is_empty()
            && (*process.front().unwrap() == '/' || *process.front().unwrap() == '\\')
        {
            process.pop_front();
        }

        //Remove / and \ from the back
        while !process.is_empty()
            && (*process.back().unwrap() == '/' || *process.back().unwrap() == '\\')
        {
            process.pop_back();
        }

        let mut result: String = String::new();

        while !process.is_empty() {
            result.push(*process.front().unwrap());
            process.pop_front();
        }

        result
    }

    //Formats the cmd argument to have the directory pre-pended to it.
    fn format_cmd(cmd: String, dir: String) -> String {
        let format_cmd = Driver::clear_format(cmd);
        let format_dir = Driver::clear_format(dir);
        let mut result: String = String::from(""); //new cmd with directory path pre-pended to it.

        //Checks if the path is not absolute (Single Drive letter + : on Windows)
        if !(format_dir.len() >= 2 && &format_dir[1..2] == ":") {
            result.push_str("./");
        }

        if format_dir.len() != 0 {
            result.push_str(&format_dir);
            result.push_str("/");
        }
        result.push_str(&format_cmd);
        result
    }

    fn new_command(&self) -> Command {
        let new_cmd: String =
            Driver::format_cmd(self.run_cmd.clone(), self.dir.to_str().unwrap().to_string()); //new cmd with directory path pre-pended to it.

        let mut args = new_cmd.split(' ');
        let mut cmd = Command::new(args.next().unwrap());
        cmd.args(args);
        cmd.current_dir(&self.dir);

        cmd
    }
    pub async fn exec_dyn_raw_cmds(
        &self,
        analysis: Analysis,
        cmds: &str,
        input: &str,
    ) -> Result<ExecOutput<Output>, ExecError> {
        let mut cmd = self.new_command();
        cmd.arg(analysis.command());
        cmd.arg(cmds);

        cmd.arg(input);

        let before = std::time::Instant::now();
        let cmd_output = cmd.output().await.map_err(|source| ExecError::RunExec {
            cmd: self.run_cmd.clone(),
            source,
        })?;
        let took = before.elapsed();

        if !cmd_output.status.success() {
            // error!(
            //     stdout = std::str::from_utf8(&cmd_output.stdout).unwrap(),
            //     stderr = std::str::from_utf8(&cmd_output.stderr).unwrap(),
            //     "failed to run command",
            // );
            return Err(ExecError::CommandFailed(cmd_output, took));
        }

        match analysis.output_from_slice(&cmd_output.stdout) {
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
    pub async fn exec_raw_cmds<E>(
        &self,
        cmds: &str,
        input: &E::Input,
    ) -> Result<ExecOutput<E::Output>, ExecError>
    where
        E: Environment + ?Sized,
    {
        let output = self
            .exec_dyn_raw_cmds(
                E::ANALYSIS,
                cmds,
                &serde_json::to_string(input).map_err(ExecError::Serialize)?,
            )
            .await?;

        match output.parsed.parsed::<E>() {
            Ok(parsed) => Ok(ExecOutput {
                output: output.output,
                parsed,
                took: output.took,
            }),
            Err(err) => Err(ExecError::Parse {
                inner: err,
                run_output: output.output,
                time: output.took,
            }),
        }
    }
    pub async fn exec<E>(
        &self,
        cmds: &Commands,
        input: &E::Input,
    ) -> Result<ExecOutput<E::Output>, ExecError>
    where
        E: Environment + ?Sized,
    {
        self.exec_raw_cmds::<E>(&cmds.to_string(), input).await
    }

    pub fn compile_output(&self) -> Option<&std::process::Output> {
        self.compile_output.as_ref()
    }
}

#[derive(Debug)]
pub struct ExecOutput<O> {
    pub output: std::process::Output,
    pub parsed: O,
    pub took: Duration,
}
