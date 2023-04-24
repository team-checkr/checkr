use std::{
    path::{Path, PathBuf},
    process::Stdio,
};

use color_eyre::{eyre::eyre, Result};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    ReuseHost,
    Build,
}

#[derive(Clone)]
pub struct DockerImage {
    pub kind: ImageKind,
    name: String,
}

impl DockerImage {
    /// Build a docker image where checko is not included, and must be mounted
    /// as a volume when running.
    pub async fn build() -> Result<DockerImage> {
        const IMAGE_NAME: &str = "checko-reuse-host";

        let dockerfile_src = include_str!("../Dockerfile.reuse-host");

        // cat Dockerfile.reuse-host | docker build -t checko-reuse-host
        let mut child = tokio::process::Command::new("docker")
            .arg("build")
            .args(["-t", IMAGE_NAME])
            .arg("-")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .stdin(Stdio::piped())
            .spawn()?;

        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(dockerfile_src.as_bytes()).await?;
        drop(stdin);

        let exit_status = child.wait().await?;
        if !exit_status.success() {
            return Err(eyre!("failed to build docker image"));
        }

        Ok(DockerImage {
            kind: ImageKind::ReuseHost,
            name: IMAGE_NAME.to_string(),
        })
    }

    /// Build a docker image where checko is build as part of the process. This
    /// requires the command to be built in the root of the project.
    pub async fn build_in_tree() -> Result<DockerImage> {
        const IMAGE_NAME: &str = "checko-build";

        let mut child = tokio::process::Command::new("docker")
            .arg("build")
            .args(["-t", IMAGE_NAME])
            .args(["-f", "./checko/Dockerfile"])
            .arg(".")
            .current_dir(project_root())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        let exit_status = child.wait().await?;
        if !exit_status.success() {
            return Err(eyre!("failed to build docker image"));
        }

        Ok(DockerImage {
            kind: ImageKind::Build,
            name: IMAGE_NAME.to_string(),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn run_cmd(&self, flags: &[impl AsRef<std::ffi::OsStr>]) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new("docker");
        cmd.arg("run").arg("--rm").arg("-i");
        cmd.args(flags).args([self.name()]);
        cmd
    }
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
