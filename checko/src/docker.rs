use std::{
    path::{Path, PathBuf},
    process::Stdio,
};

use color_eyre::{eyre::eyre, Result};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone)]
pub enum ImageKind {
    GitHub,
    Local,
}

#[derive(Clone)]
pub struct DockerImage {
    kind: ImageKind,
    name: String,
}

impl DockerImage {
    pub async fn build() -> Result<DockerImage> {
        const IMAGE_NAME: &str = "checko-github";

        let dockerfile_src = include_str!("../Dockerfile.github");

        // cat Dockerfile.github | docker build --platform linux/x86_64 -t checko-github
        let mut child = tokio::process::Command::new("docker")
            .arg("build")
            .args(["--platform", "linux/x86_64"])
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
            kind: ImageKind::GitHub,
            name: IMAGE_NAME.to_string(),
        })
    }

    pub async fn build_in_tree() -> Result<DockerImage> {
        const IMAGE_NAME: &str = "checko-local";

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
            kind: ImageKind::Local,
            name: IMAGE_NAME.to_string(),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn run_cmd(&self, flags: &[impl AsRef<std::ffi::OsStr>]) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new("docker");
        cmd.arg("run").arg("--rm");
        match &self.kind {
            ImageKind::GitHub => {
                cmd.args(["--platform", "linux/x86_64"]);
            }
            ImageKind::Local => {}
        }
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
