#[cfg(test)]
mod tests;

use std::{path::Path, process::Stdio};

use color_eyre::eyre::{bail, Context};
use tokio::process::Command;

static GIT_SSH_SEMAPHORE: stdx::concurrency::Semaphore = stdx::concurrency::semaphore();

static SSH_CONTROL_FOLDER: once_cell::sync::Lazy<tempfile::TempDir> =
    once_cell::sync::Lazy::new(|| {
        tempfile::tempdir().expect("could not create temporary directory for ssh control path")
    });
static GIT_SSH_COMMAND: once_cell::sync::Lazy<String> = once_cell::sync::Lazy::new(|| {
    format!(
        "ssh -o ControlPath={control_path}/%r@%h:%p -o ControlMaster=auto -o ControlPersist=60",
        control_path = SSH_CONTROL_FOLDER.path().display()
    )
});

trait CommandExt {
    async fn success_with_output(&mut self) -> color_eyre::Result<Vec<u8>>;
    async fn success_without_output(&mut self) -> color_eyre::Result<()>;
}

impl CommandExt for Command {
    async fn success_with_output(&mut self) -> color_eyre::Result<Vec<u8>> {
        let output = self
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .await
            .wrap_err("could not run command")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::debug!(?stdout, ?stderr, "command output");

        if !output.status.success() {
            let err = String::from_utf8(output.stderr).wrap_err("stderr is not valid utf8")?;
            bail!("command failed: {err}");
        }
        Ok(output.stdout)
    }
    async fn success_without_output(&mut self) -> color_eyre::Result<()> {
        let output = self
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .await
            .wrap_err("could not run command")?;
        if !output.status.success() {
            let err = String::from_utf8(output.stderr).wrap_err("stderr is not valid utf8")?;
            bail!("command failed: {err}");
        }
        Ok(())
    }
}

pub async fn clone_or_pull(git: &str, path: impl AsRef<Path>) -> color_eyre::Result<()> {
    let path = path.as_ref();
    if !path.join(".git").try_exists().unwrap_or(false) {
        clone(git, path).await
    } else {
        checkout_main(git, path).await?;
        pull(git, path).await
    }
}

pub async fn clone_or_clean_reset_pull(
    git: &str,
    path: impl AsRef<Path>,
) -> color_eyre::Result<()> {
    let path = path.as_ref();
    if !path.join(".git").try_exists().unwrap_or(false) {
        clone(git, path).await
    } else {
        checkout_main(git, path).await?;
        clean(git, path).await?;
        reset_hard(git, path).await?;
        pull(git, path).await
    }
}

pub async fn clone(git: &str, path: impl AsRef<Path>) -> color_eyre::Result<()> {
    let _permit = GIT_SSH_SEMAPHORE.acquire().await;

    tracing::debug!(?git, "cloning group git repository");
    Command::new("git")
        .arg("clone")
        .arg(git)
        .args(["."])
        .env("GIT_SSH_COMMAND", GIT_SSH_COMMAND.as_str())
        .current_dir(path)
        .success_without_output()
        .await
        .wrap_err_with(|| format!("could not clone group git repository: '{git}'"))?;
    Ok(())
}

pub async fn checkout_main(git: &str, path: impl AsRef<Path>) -> color_eyre::Result<()> {
    let _permit = GIT_SSH_SEMAPHORE.acquire().await;

    tracing::debug!(?git, "checking out main branch");
    Command::new("git")
        .arg("checkout")
        .arg("main")
        .env("GIT_SSH_COMMAND", GIT_SSH_COMMAND.as_str())
        .current_dir(path)
        .success_without_output()
        .await
        .wrap_err_with(|| {
            format!("could not checkout main branch of group git repository: '{git}'")
        })?;
    Ok(())
}

pub async fn clean(git: &str, path: impl AsRef<Path>) -> color_eyre::Result<()> {
    let _permit = GIT_SSH_SEMAPHORE.acquire().await;

    tracing::debug!(?git, "checking out main branch");
    Command::new("git")
        .arg("clean")
        .arg("-f")
        .env("GIT_SSH_COMMAND", GIT_SSH_COMMAND.as_str())
        .current_dir(path)
        .success_without_output()
        .await
        .wrap_err_with(|| format!("could clean git repository: '{git}'"))?;
    Ok(())
}

pub async fn reset_hard(git: &str, path: impl AsRef<Path>) -> color_eyre::Result<()> {
    let _permit = GIT_SSH_SEMAPHORE.acquire().await;

    tracing::debug!(?git, "checking out main branch");
    Command::new("git")
        .arg("reset")
        .arg("--hard")
        .env("GIT_SSH_COMMAND", GIT_SSH_COMMAND.as_str())
        .current_dir(path)
        .success_without_output()
        .await
        .wrap_err_with(|| format!("could reset --hard git repository: '{git}'"))?;
    Ok(())
}

pub async fn pull(git: &str, path: impl AsRef<Path>) -> color_eyre::Result<()> {
    let _permit = GIT_SSH_SEMAPHORE.acquire().await;

    tracing::debug!(?git, "pulling group git repository");
    Command::new("git")
        .arg("pull")
        .env("GIT_SSH_COMMAND", GIT_SSH_COMMAND.as_str())
        .current_dir(&path)
        .success_without_output()
        .await
        .wrap_err_with(|| format!("could not pull group git repository: '{git}'"))?;
    Ok(())
}

pub async fn hash(path: impl AsRef<Path>, rev: Option<&str>) -> color_eyre::Result<String> {
    let _permit = GIT_SSH_SEMAPHORE.acquire().await;

    let output = Command::new("git")
        .arg("rev-parse")
        .arg(rev.unwrap_or("HEAD"))
        .current_dir(path)
        .success_with_output()
        .await
        .wrap_err("could not get git hash")?;
    let hash = String::from_utf8(output).wrap_err("git hash is not valid utf8")?;
    Ok(hash.trim().to_string())
}

pub async fn latest_commit_before(
    path: impl AsRef<Path>,
    before: chrono::DateTime<chrono::FixedOffset>,
    ignored_authors: &[String],
) -> color_eyre::Result<Option<String>> {
    let _permit = GIT_SSH_SEMAPHORE.acquire().await;

    let path = path.as_ref();
    tracing::debug!(?before, "checking out latest commit before");
    let before = before.format("%Y-%m-%d %H:%M:%S").to_string();
    let mut cmd = Command::new("git");
    cmd.args(["rev-list", "-n", "1"])
        .arg(format!("--before='{before}'"));
    if !ignored_authors.is_empty() {
        cmd.arg("--perl-regexp")
            .arg(format!("--author=^(?!{})", ignored_authors.join("|")));
    }
    let commit_rev_bytes = cmd
        .arg("HEAD")
        .current_dir(path)
        .success_with_output()
        .await
        .wrap_err_with(|| format!("could not get latest commit before {before}"))?;
    let commit_rev = std::str::from_utf8(&commit_rev_bytes).unwrap().trim();
    if commit_rev.is_empty() {
        tracing::debug!("no commit found before {before}");
        return Ok(None);
    }
    Ok(Some(commit_rev.to_string()))
}

pub async fn checkout_latest_before(
    git: &str,
    path: impl AsRef<Path>,
    before: chrono::DateTime<chrono::FixedOffset>,
    ignored_authors: &[String],
) -> color_eyre::Result<bool> {
    let path = path.as_ref();
    checkout_main(git, path).await?;
    let commit_rev = latest_commit_before(path, before, ignored_authors).await?;
    let Some(commit_rev) = commit_rev else {
        tracing::debug!("no commit found before {before}");
        return Ok(false);
    };
    tracing::debug!(?commit_rev, ?path, "latest commit before");
    Command::new("git")
        .arg("checkout")
        .arg(&commit_rev)
        .current_dir(path)
        .success_without_output()
        .await
        .wrap_err_with(|| format!("could not checkout latest commit: {commit_rev}"))?;
    Ok(true)
}

pub async fn checkout_commit(group_path: impl AsRef<Path>, commit: &str) -> color_eyre::Result<()> {
    let _permit = GIT_SSH_SEMAPHORE.acquire().await;

    tracing::debug!(?commit, "checking out commit");
    Command::new("git")
        .arg("checkout")
        .arg(commit)
        .env("GIT_SSH_COMMAND", GIT_SSH_COMMAND.as_str())
        .current_dir(group_path)
        .success_without_output()
        .await
        .wrap_err_with(|| format!("could not checkout commit: {commit}"))?;
    Ok(())
}
